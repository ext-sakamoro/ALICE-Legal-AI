//! The engine binary refuses to start without its signing seed and audit log id, and never prints
//! the seed.

use std::process::Command;

/// a seed made inside the test (no key or seed is kept in a fixture)
fn test_seed() -> String {
    (0u8..32)
        .map(|i| format!("{:02x}", i.wrapping_mul(53).wrapping_add(11)))
        .collect()
}

fn run(env: &[(&str, &str)]) -> (Option<i32>, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_legal-engine"));
    cmd.env_clear();
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("the engine runs");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
    )
}

#[test]
fn without_a_seed_the_engine_does_not_start() {
    let (code, out) = run(&[("LEGAL_AUDIT_LOG_ID", "1")]);
    assert_ne!(code, Some(0), "{out}");
    assert!(out.contains("LEGAL_SIGNING_SEED"), "{out}");
}

#[test]
fn a_malformed_seed_is_refused_without_being_printed() {
    let seed = test_seed();
    let bad = format!("{}zz", &seed[..62]);
    let (code, out) = run(&[("LEGAL_SIGNING_SEED", &bad), ("LEGAL_AUDIT_LOG_ID", "1")]);
    assert_ne!(code, Some(0), "{out}");
    assert!(out.contains("LEGAL_SIGNING_SEED"), "{out}");
    assert!(!out.contains(&seed[..16]), "{out}");
}

#[test]
fn without_an_audit_log_id_the_engine_does_not_start() {
    let seed = test_seed();
    let (code, out) = run(&[("LEGAL_SIGNING_SEED", &seed)]);
    assert_ne!(code, Some(0), "{out}");
    assert!(out.contains("LEGAL_AUDIT_LOG_ID"), "{out}");
    assert!(!out.contains(&seed[..16]), "{out}");
}

/// with both set, configuration passes and the engine stops only at the address it cannot parse
#[test]
fn with_both_set_the_configuration_is_accepted() {
    let seed = test_seed();
    let (code, out) = run(&[
        ("LEGAL_SIGNING_SEED", &seed),
        ("LEGAL_AUDIT_LOG_ID", "1"),
        ("LEGAL_ADDR", "not an address"),
    ]);
    assert_ne!(code, Some(0), "{out}");
    assert!(out.contains("LEGAL_ADDR"), "{out}");
    assert!(!out.contains("LEGAL_SIGNING_SEED"), "{out}");
    assert!(!out.contains(&seed[..16]), "{out}");
}
