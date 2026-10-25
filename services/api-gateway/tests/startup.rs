//! The gateway refuses to start without a JWT secret (unset or empty) and never prints it.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// a secret made inside the test (no secret is kept in a fixture)
fn test_secret() -> String {
    (0u8..24)
        .map(|i| char::from(b'a' + (i.wrapping_mul(7) % 26)))
        .collect()
}

/// Run the gateway with only `env`; a gateway still running after a few seconds has started
/// (it is killed and reported as `None`).
fn run(env: &[(&str, &str)]) -> (Option<i32>, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_api-gateway"));
    cmd.env_clear()
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().expect("the gateway runs");
    let start = Instant::now();
    let code = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status.code();
        }
        if start.elapsed() > Duration::from_secs(5) {
            child.kill().expect("kill");
            child.wait().expect("reap");
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let out = child.wait_with_output().expect("output");
    (
        code,
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
    )
}

#[test]
fn without_a_jwt_secret_the_gateway_does_not_start() {
    for env in [
        vec![("GATEWAY_ADDR", "127.0.0.1:0")],
        vec![("GATEWAY_ADDR", "127.0.0.1:0"), ("JWT_SECRET", "")],
    ] {
        let (code, out) = run(&env);
        assert!(code.is_some(), "{env:?}: the gateway started: {out}");
        assert_ne!(code, Some(0), "{env:?}: {out}");
        assert!(out.contains("JWT_SECRET"), "{env:?}: {out}");
    }
}

/// with a secret, configuration passes and the gateway stops only at the address it cannot parse;
/// the secret is not printed
#[test]
fn with_a_jwt_secret_the_configuration_is_accepted() {
    let secret = test_secret();
    let (code, out) = run(&[("JWT_SECRET", &secret), ("GATEWAY_ADDR", "not an address")]);
    assert!(code.is_some(), "{out}");
    assert_ne!(code, Some(0), "{out}");
    assert!(!out.contains("JWT_SECRET"), "{out}");
    assert!(!out.contains(&secret), "{out}");
}
