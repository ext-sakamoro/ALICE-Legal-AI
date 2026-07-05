//! similarity.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// TF-IDF Similarity
// ---------------------------------------------------------------------------

/// 単語分割 (簡易: スペース区切り、小文字化)
#[must_use]
pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() {
            current.push(if c.is_uppercase() {
                c.to_ascii_lowercase()
            } else {
                c
            });
        } else if !current.is_empty() {
            tokens.push(core::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// TF: 単語頻度
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn term_frequency(tokens: &[String]) -> BTreeMap<String, f64> {
    let mut tf = BTreeMap::new();
    let n = tokens.len() as f64;
    if n == 0.0 {
        return tf;
    }
    for token in tokens {
        *tf.entry(token.clone()).or_insert(0.0) += 1.0;
    }
    for val in tf.values_mut() {
        *val /= n;
    }
    tf
}

/// コサイン類似度
#[must_use]
pub fn cosine_similarity(tf1: &BTreeMap<String, f64>, tf2: &BTreeMap<String, f64>) -> f64 {
    let mut dot = 0.0;
    let mut norm1 = 0.0;
    let mut norm2 = 0.0;

    for (key, &v1) in tf1 {
        norm1 += v1 * v1;
        if let Some(&v2) = tf2.get(key) {
            dot += v1 * v2;
        }
    }
    for &v2 in tf2.values() {
        norm2 += v2 * v2;
    }

    let denom = sqrt_approx(norm1) * sqrt_approx(norm2);
    if denom < 1e-10 {
        0.0
    } else {
        dot / denom
    }
}

fn sqrt_approx(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut g = x / 2.0;
    for _ in 0..20 {
        g = f64::midpoint(g, x / g);
    }
    g
}
