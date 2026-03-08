//! ALICE-Legal-AI — Legal document analysis engine
//!
//! 条項抽出、リスクスコアリング、TF-IDF類似度、契約条件チェック

#![no_std]
extern crate alloc;
use alloc::{collections::BTreeMap, string::String, vec::Vec};

// ---------------------------------------------------------------------------
// Document & Clause
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    pub id: String,
    pub section: String,
    pub text: String,
    pub clause_type: ClauseType,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseType {
    Indemnification,
    Limitation,
    Termination,
    Confidentiality,
    Ip,
    Warranty,
    Governing,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

// ---------------------------------------------------------------------------
// Keyword-based Clause Classification
// ---------------------------------------------------------------------------

/// キーワードに基づく条項分類
#[must_use]
pub fn classify_clause(text: &str) -> ClauseType {
    let lower: Vec<u8> = text
        .bytes()
        .map(|b: u8| if b.is_ascii_uppercase() { b + 32 } else { b })
        .collect();
    let s = core::str::from_utf8(&lower).unwrap_or("");

    if s.contains("indemnif") || s.contains("hold harmless") {
        ClauseType::Indemnification
    } else if s.contains("limitation") || s.contains("liable") || s.contains("damages") {
        ClauseType::Limitation
    } else if s.contains("terminat") || s.contains("cancel") {
        ClauseType::Termination
    } else if s.contains("confidential") || s.contains("non-disclosure") || s.contains("nda") {
        ClauseType::Confidentiality
    } else if s.contains("intellectual property") || s.contains("patent") || s.contains("copyright")
    {
        ClauseType::Ip
    } else if s.contains("warrant") || s.contains("guarantee") {
        ClauseType::Warranty
    } else if s.contains("governing law") || s.contains("jurisdiction") {
        ClauseType::Governing
    } else {
        ClauseType::Other
    }
}

/// 条項のリスクレベル推定
#[must_use]
pub fn assess_risk(text: &str) -> RiskLevel {
    let lower: Vec<u8> = text
        .bytes()
        .map(|b: u8| if b.is_ascii_uppercase() { b + 32 } else { b })
        .collect();
    let s = core::str::from_utf8(&lower).unwrap_or("");

    let high_risk = [
        "unlimited liability",
        "sole discretion",
        "without notice",
        "irrevocable",
        "perpetual",
    ];
    let medium_risk = ["may terminate", "reasonable efforts", "material breach"];

    for &keyword in &high_risk {
        if s.contains(keyword) {
            return RiskLevel::Critical;
        }
    }
    for &keyword in &medium_risk {
        if s.contains(keyword) {
            return RiskLevel::Medium;
        }
    }
    RiskLevel::Low
}

// ---------------------------------------------------------------------------
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

// ---------------------------------------------------------------------------
// Contract Checklist
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecklistItem {
    pub name: String,
    pub required: bool,
    pub found: bool,
}

/// 契約条件チェックリスト生成
#[must_use]
pub fn check_contract(clauses: &[Clause]) -> Vec<ChecklistItem> {
    let required_types = [
        (ClauseType::Indemnification, "Indemnification"),
        (ClauseType::Limitation, "Limitation of Liability"),
        (ClauseType::Termination, "Termination"),
        (ClauseType::Confidentiality, "Confidentiality"),
        (ClauseType::Governing, "Governing Law"),
    ];

    required_types
        .iter()
        .map(|(ct, name)| ChecklistItem {
            name: String::from(*name),
            required: true,
            found: clauses.iter().any(|c| c.clause_type == *ct),
        })
        .collect()
}

/// 契約のリスクスコア: critical条項の割合
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn contract_risk_score(clauses: &[Clause]) -> f64 {
    if clauses.is_empty() {
        return 0.0;
    }
    let risk_sum: u32 = clauses
        .iter()
        .map(|c| match c.risk_level {
            RiskLevel::Low => 1,
            RiskLevel::Medium => 3,
            RiskLevel::High => 7,
            RiskLevel::Critical => 10,
        })
        .sum();
    f64::from(risk_sum) / (clauses.len() as f64 * 10.0) * 100.0
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegalError {
    ParseError,
    ClassificationFailed,
}

impl core::fmt::Display for LegalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ParseError => write!(f, "parse error"),
            Self::ClassificationFailed => write!(f, "classification failed"),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    // ヘルパー: テスト用Clause生成
    fn make_clause(id: &str, ct: ClauseType, rl: RiskLevel) -> Clause {
        Clause {
            id: String::from(id),
            section: String::from("1"),
            text: String::new(),
            clause_type: ct,
            risk_level: rl,
        }
    }

    // =======================================================================
    // classify_clause テスト (22件)
    // =======================================================================

    #[test]
    fn classify_indemnification() {
        assert_eq!(
            classify_clause("The Vendor shall indemnify the Client"),
            ClauseType::Indemnification
        );
    }

    #[test]
    fn classify_hold_harmless() {
        // "hold harmless" もIndemnificationに分類
        assert_eq!(
            classify_clause("hold harmless against claims"),
            ClauseType::Indemnification
        );
    }

    #[test]
    fn classify_indemnification_uppercase() {
        // 大文字入力の処理確認
        assert_eq!(
            classify_clause("INDEMNIFICATION CLAUSE"),
            ClauseType::Indemnification
        );
    }

    #[test]
    fn classify_limitation() {
        assert_eq!(
            classify_clause("Limitation of liability shall not exceed"),
            ClauseType::Limitation
        );
    }

    #[test]
    fn classify_liable() {
        // "liable" もLimitationに分類
        assert_eq!(
            classify_clause("Neither party shall be liable for indirect losses"),
            ClauseType::Limitation
        );
    }

    #[test]
    fn classify_damages() {
        // "damages" もLimitationに分類
        assert_eq!(
            classify_clause("consequential damages are excluded"),
            ClauseType::Limitation
        );
    }

    #[test]
    fn classify_termination() {
        assert_eq!(
            classify_clause("Either party may terminate this agreement"),
            ClauseType::Termination
        );
    }

    #[test]
    fn classify_cancel() {
        // "cancel" もTerminationに分類
        assert_eq!(
            classify_clause("The client may cancel this subscription"),
            ClauseType::Termination
        );
    }

    #[test]
    fn classify_confidentiality() {
        assert_eq!(
            classify_clause("All confidential information shall be protected"),
            ClauseType::Confidentiality
        );
    }

    #[test]
    fn classify_nondisclosure() {
        // "non-disclosure" もConfidentialityに分類
        assert_eq!(
            classify_clause("This non-disclosure agreement applies"),
            ClauseType::Confidentiality
        );
    }

    #[test]
    fn classify_nda() {
        // "nda" もConfidentialityに分類
        assert_eq!(
            classify_clause("Subject to the NDA terms"),
            ClauseType::Confidentiality
        );
    }

    #[test]
    fn classify_ip() {
        assert_eq!(
            classify_clause("Intellectual Property rights remain with the author"),
            ClauseType::Ip
        );
    }

    #[test]
    fn classify_patent() {
        // "patent" もIpに分類
        assert_eq!(
            classify_clause("Patent rights are reserved"),
            ClauseType::Ip
        );
    }

    #[test]
    fn classify_copyright() {
        // "copyright" もIpに分類
        assert_eq!(
            classify_clause("Copyright ownership transfers"),
            ClauseType::Ip
        );
    }

    #[test]
    fn classify_warranty() {
        assert_eq!(
            classify_clause("The warranty period is 12 months"),
            ClauseType::Warranty
        );
    }

    #[test]
    fn classify_guarantee() {
        // "guarantee" もWarrantyに分類
        assert_eq!(
            classify_clause("We guarantee the quality of service"),
            ClauseType::Warranty
        );
    }

    #[test]
    fn classify_governing() {
        assert_eq!(
            classify_clause("Governing law shall be the State of New York"),
            ClauseType::Governing
        );
    }

    #[test]
    fn classify_jurisdiction() {
        // "jurisdiction" もGoverningに分類
        assert_eq!(
            classify_clause("Exclusive jurisdiction of Tokyo courts"),
            ClauseType::Governing
        );
    }

    #[test]
    fn classify_other() {
        // どのキーワードにも該当しない場合
        assert_eq!(
            classify_clause("Payment shall be made within 30 days"),
            ClauseType::Other
        );
    }

    #[test]
    fn classify_empty_string() {
        // 空文字列はOtherに分類
        assert_eq!(classify_clause(""), ClauseType::Other);
    }

    #[test]
    fn classify_priority_indemnification_over_limitation() {
        // indemnifとdamages両方含む場合、先にマッチするindemnificationが返る
        assert_eq!(
            classify_clause("indemnify against all damages"),
            ClauseType::Indemnification
        );
    }

    #[test]
    fn classify_mixed_case() {
        // 大小文字混在
        assert_eq!(
            classify_clause("CoNfIdEnTiAl data protection"),
            ClauseType::Confidentiality
        );
    }

    // =======================================================================
    // assess_risk テスト (18件)
    // =======================================================================

    #[test]
    fn risk_critical_unlimited_liability() {
        assert_eq!(
            assess_risk("The company has unlimited liability for damages"),
            RiskLevel::Critical
        );
    }

    #[test]
    fn risk_critical_sole_discretion() {
        assert_eq!(
            assess_risk("at its sole discretion may modify terms"),
            RiskLevel::Critical
        );
    }

    #[test]
    fn risk_critical_without_notice() {
        assert_eq!(
            assess_risk("may change fees without notice"),
            RiskLevel::Critical
        );
    }

    #[test]
    fn risk_critical_irrevocable() {
        assert_eq!(
            assess_risk("grants an irrevocable license"),
            RiskLevel::Critical
        );
    }

    #[test]
    fn risk_critical_perpetual() {
        assert_eq!(
            assess_risk("a perpetual and worldwide license"),
            RiskLevel::Critical
        );
    }

    #[test]
    fn risk_medium_may_terminate() {
        assert_eq!(
            assess_risk("Either party may terminate upon notice"),
            RiskLevel::Medium
        );
    }

    #[test]
    fn risk_medium_reasonable_efforts() {
        assert_eq!(
            assess_risk("shall use reasonable efforts to comply"),
            RiskLevel::Medium
        );
    }

    #[test]
    fn risk_medium_material_breach() {
        assert_eq!(
            assess_risk("upon material breach of this agreement"),
            RiskLevel::Medium
        );
    }

    #[test]
    fn risk_low_standard() {
        assert_eq!(assess_risk("Standard terms apply"), RiskLevel::Low);
    }

    #[test]
    fn risk_low_empty() {
        // 空文字列はLow
        assert_eq!(assess_risk(""), RiskLevel::Low);
    }

    #[test]
    fn risk_critical_uppercase() {
        // 大文字でもCritical判定
        assert_eq!(
            assess_risk("UNLIMITED LIABILITY applies"),
            RiskLevel::Critical
        );
    }

    #[test]
    fn risk_critical_takes_priority_over_medium() {
        // CriticalとMedium両方含む場合、Criticalが先に返る
        assert_eq!(
            assess_risk("irrevocable license, may terminate on material breach"),
            RiskLevel::Critical
        );
    }

    #[test]
    fn risk_low_partial_keyword_no_match() {
        // "limit"はキーワードに該当しない（"unlimited liability"が必要）
        assert_eq!(
            assess_risk("We will limit access to authorized users"),
            RiskLevel::Low
        );
    }

    #[test]
    fn risk_low_unrelated_long_text() {
        // 長文でもキーワードなしならLow
        assert_eq!(
            assess_risk("The provider shall deliver services in accordance with the schedule"),
            RiskLevel::Low
        );
    }

    #[test]
    fn risk_critical_sole_discretion_mixed_case() {
        assert_eq!(
            assess_risk("At Its SOLE DISCRETION the vendor"),
            RiskLevel::Critical
        );
    }

    #[test]
    fn risk_medium_reasonable_efforts_sentence() {
        assert_eq!(
            assess_risk("Company will use Reasonable Efforts to maintain uptime"),
            RiskLevel::Medium
        );
    }

    #[test]
    fn risk_critical_without_notice_in_context() {
        assert_eq!(
            assess_risk("Prices may be changed without notice at any time"),
            RiskLevel::Critical
        );
    }

    #[test]
    fn risk_low_no_keywords_at_all() {
        assert_eq!(
            assess_risk("This is a simple payment clause"),
            RiskLevel::Low
        );
    }

    // =======================================================================
    // tokenize テスト (15件)
    // =======================================================================

    #[test]
    fn tokenize_basic() {
        let tokens = tokenize("Hello, World! Test 123");
        assert_eq!(tokens, vec!["hello", "world", "test", "123"]);
    }

    #[test]
    fn tokenize_empty() {
        // 空文字列は空Vec
        let tokens = tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn tokenize_whitespace_only() {
        // 空白のみは空Vec
        let tokens = tokenize("   ");
        assert!(tokens.is_empty());
    }

    #[test]
    fn tokenize_single_word() {
        let tokens = tokenize("hello");
        assert_eq!(tokens, vec!["hello"]);
    }

    #[test]
    fn tokenize_all_uppercase() {
        // 全大文字は小文字化
        let tokens = tokenize("HELLO WORLD");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn tokenize_numbers_only() {
        let tokens = tokenize("123 456 789");
        assert_eq!(tokens, vec!["123", "456", "789"]);
    }

    #[test]
    fn tokenize_mixed_separators() {
        // 複数種の区切り文字
        let tokens = tokenize("a,b.c;d:e");
        assert_eq!(tokens, vec!["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn tokenize_consecutive_separators() {
        // 連続区切り文字
        let tokens = tokenize("hello...world!!!");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn tokenize_hyphenated_word() {
        // ハイフンは区切り文字として扱われる
        let tokens = tokenize("non-disclosure");
        assert_eq!(tokens, vec!["non", "disclosure"]);
    }

    #[test]
    fn tokenize_underscore_splits() {
        // アンダースコアは区切り文字
        let tokens = tokenize("hello_world");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn tokenize_tabs_and_newlines() {
        let tokens = tokenize("word1\tword2\nword3");
        assert_eq!(tokens, vec!["word1", "word2", "word3"]);
    }

    #[test]
    fn tokenize_leading_trailing_separators() {
        let tokens = tokenize("...hello...");
        assert_eq!(tokens, vec!["hello"]);
    }

    #[test]
    fn tokenize_preserves_order() {
        let tokens = tokenize("Zebra apple Mango");
        assert_eq!(tokens, vec!["zebra", "apple", "mango"]);
    }

    #[test]
    fn tokenize_long_text() {
        let tokens = tokenize("The quick brown fox jumps over the lazy dog");
        assert_eq!(tokens.len(), 9);
        assert_eq!(tokens[0], "the");
        assert_eq!(tokens[8], "dog");
    }

    #[test]
    fn tokenize_alphanumeric_mixed() {
        // 英数字混在は1トークン
        let tokens = tokenize("abc123 def456");
        assert_eq!(tokens, vec!["abc123", "def456"]);
    }

    // =======================================================================
    // term_frequency テスト (10件)
    // =======================================================================

    #[test]
    fn tf_empty() {
        // 空トークンリストは空マップ
        let tf = term_frequency(&[]);
        assert!(tf.is_empty());
    }

    #[test]
    fn tf_single_token() {
        let tokens = vec![String::from("hello")];
        let tf = term_frequency(&tokens);
        assert_eq!(tf.len(), 1);
        assert!((tf["hello"] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn tf_uniform_distribution() {
        // 全て異なるトークン: 各TF = 1/n
        let tokens = vec![
            String::from("a"),
            String::from("b"),
            String::from("c"),
            String::from("d"),
        ];
        let tf = term_frequency(&tokens);
        assert_eq!(tf.len(), 4);
        for &val in tf.values() {
            assert!((val - 0.25).abs() < 1e-10);
        }
    }

    #[test]
    fn tf_repeated_token() {
        // 同じトークンが繰り返される場合
        let tokens = vec![
            String::from("the"),
            String::from("the"),
            String::from("the"),
        ];
        let tf = term_frequency(&tokens);
        assert_eq!(tf.len(), 1);
        assert!((tf["the"] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn tf_two_tokens_one_repeated() {
        // 2種類、片方が2回
        let tokens = vec![String::from("a"), String::from("b"), String::from("a")];
        let tf = term_frequency(&tokens);
        assert_eq!(tf.len(), 2);
        // a: 2/3, b: 1/3
        assert!((tf["a"] - 2.0 / 3.0).abs() < 1e-10);
        assert!((tf["b"] - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn tf_values_sum_to_one() {
        // TFの合計は1.0
        let tokens = tokenize("the quick brown fox jumps over the lazy dog");
        let tf = term_frequency(&tokens);
        let sum: f64 = tf.values().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn tf_all_same_token() {
        let tokens = vec![
            String::from("x"),
            String::from("x"),
            String::from("x"),
            String::from("x"),
            String::from("x"),
        ];
        let tf = term_frequency(&tokens);
        assert_eq!(tf.len(), 1);
        assert!((tf["x"] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn tf_large_token_set() {
        // 10個の異なるトークン
        let tokens: Vec<String> = (0..10).map(|i| alloc::format!("word{i}")).collect();
        let tf = term_frequency(&tokens);
        assert_eq!(tf.len(), 10);
        for &val in tf.values() {
            assert!((val - 0.1).abs() < 1e-10);
        }
    }

    #[test]
    fn tf_from_tokenize_integration() {
        // tokenize → term_frequency の統合テスト
        let tf = term_frequency(&tokenize("legal legal contract"));
        assert_eq!(tf.len(), 2);
        // legal: 2/3, contract: 1/3
        assert!((tf["legal"] - 2.0 / 3.0).abs() < 1e-10);
        assert!((tf["contract"] - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn tf_keys_are_lowercase() {
        // tokenizeが小文字化するので、TFのキーも小文字
        let tf = term_frequency(&tokenize("Hello WORLD"));
        assert!(tf.contains_key("hello"));
        assert!(tf.contains_key("world"));
    }

    // =======================================================================
    // cosine_similarity テスト (12件)
    // =======================================================================

    #[test]
    fn cosine_identical() {
        // 同一文書の類似度は1.0
        let tf = term_frequency(&tokenize("the quick brown fox"));
        let sim = cosine_similarity(&tf, &tf);
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn cosine_completely_different() {
        // 完全に異なる文書の類似度は0.0
        let tf1 = term_frequency(&tokenize("legal contract agreement"));
        let tf2 = term_frequency(&tokenize("physics quantum mechanics"));
        let sim = cosine_similarity(&tf1, &tf2);
        assert!(sim < 0.01);
    }

    #[test]
    fn cosine_both_empty() {
        // 両方空マップ → 0.0
        let empty = BTreeMap::new();
        let sim = cosine_similarity(&empty, &empty);
        assert!((sim - 0.0).abs() < 1e-10);
    }

    #[test]
    fn cosine_one_empty() {
        // 片方空 → 0.0
        let tf = term_frequency(&tokenize("hello world"));
        let empty = BTreeMap::new();
        let sim = cosine_similarity(&tf, &empty);
        assert!((sim - 0.0).abs() < 1e-10);
    }

    #[test]
    fn cosine_other_empty() {
        // 逆方向で片方空 → 0.0
        let tf = term_frequency(&tokenize("hello world"));
        let empty = BTreeMap::new();
        let sim = cosine_similarity(&empty, &tf);
        assert!((sim - 0.0).abs() < 1e-10);
    }

    #[test]
    fn cosine_partial_overlap() {
        // 部分的な重複
        let tf1 = term_frequency(&tokenize("legal contract review"));
        let tf2 = term_frequency(&tokenize("legal analysis review"));
        let sim = cosine_similarity(&tf1, &tf2);
        // 3語中2語が共通 → 0 < sim < 1
        assert!(sim > 0.1);
        assert!(sim < 1.0);
    }

    #[test]
    fn cosine_symmetry() {
        // コサイン類似度は対称
        let tf1 = term_frequency(&tokenize("the contract shall"));
        let tf2 = term_frequency(&tokenize("the agreement shall"));
        let sim12 = cosine_similarity(&tf1, &tf2);
        let sim21 = cosine_similarity(&tf2, &tf1);
        assert!((sim12 - sim21).abs() < 1e-10);
    }

    #[test]
    fn cosine_single_common_word() {
        // 1語だけ共通
        let tf1 = term_frequency(&tokenize("legal matters"));
        let tf2 = term_frequency(&tokenize("legal eagles"));
        let sim = cosine_similarity(&tf1, &tf2);
        assert!(sim > 0.0);
        assert!(sim < 1.0);
    }

    #[test]
    fn cosine_identical_single_word() {
        // 単一同一単語
        let tf = term_frequency(&tokenize("hello"));
        let sim = cosine_similarity(&tf, &tf);
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn cosine_same_words_different_frequency() {
        // 同じ単語セットだが頻度が異なる
        let tf1 = term_frequency(&tokenize("legal legal contract"));
        let tf2 = term_frequency(&tokenize("legal contract contract"));
        let sim = cosine_similarity(&tf1, &tf2);
        // 同じ語彙なので類似度は高いが1.0未満
        assert!(sim > 0.5);
        assert!(sim <= 1.0);
    }

    #[test]
    fn cosine_self_is_max() {
        // 自分自身との類似度が最大
        let tf_a = term_frequency(&tokenize("contract agreement legal"));
        let tf_b = term_frequency(&tokenize("contract payment invoice"));
        let self_sim = cosine_similarity(&tf_a, &tf_a);
        let cross_sim = cosine_similarity(&tf_a, &tf_b);
        assert!(self_sim >= cross_sim);
    }

    #[test]
    fn cosine_non_negative() {
        // TFは常に非負なのでコサイン類似度も非負
        let tf1 = term_frequency(&tokenize("alpha beta gamma"));
        let tf2 = term_frequency(&tokenize("delta epsilon zeta"));
        let sim = cosine_similarity(&tf1, &tf2);
        assert!(sim >= 0.0);
    }

    // =======================================================================
    // check_contract テスト (10件)
    // =======================================================================

    #[test]
    fn contract_checklist_basic() {
        let clauses = vec![
            make_clause("1", ClauseType::Indemnification, RiskLevel::Low),
            make_clause("2", ClauseType::Termination, RiskLevel::Low),
        ];
        let checklist = check_contract(&clauses);
        assert_eq!(checklist.len(), 5);
        assert!(checklist[0].found); // Indemnification
        assert!(!checklist[1].found); // Limitation
        assert!(checklist[2].found); // Termination
        assert!(!checklist[3].found); // Confidentiality
        assert!(!checklist[4].found); // Governing
    }

    #[test]
    fn contract_checklist_empty() {
        // 空の条項リスト: 全てfound=false
        let checklist = check_contract(&[]);
        assert_eq!(checklist.len(), 5);
        for item in &checklist {
            assert!(item.required);
            assert!(!item.found);
        }
    }

    #[test]
    fn contract_checklist_all_found() {
        // 全ての必須条項が揃っている場合
        let clauses = vec![
            make_clause("1", ClauseType::Indemnification, RiskLevel::Low),
            make_clause("2", ClauseType::Limitation, RiskLevel::Low),
            make_clause("3", ClauseType::Termination, RiskLevel::Low),
            make_clause("4", ClauseType::Confidentiality, RiskLevel::Low),
            make_clause("5", ClauseType::Governing, RiskLevel::Low),
        ];
        let checklist = check_contract(&clauses);
        for item in &checklist {
            assert!(item.found);
        }
    }

    #[test]
    fn contract_checklist_only_indemnification() {
        let clauses = vec![make_clause(
            "1",
            ClauseType::Indemnification,
            RiskLevel::Low,
        )];
        let checklist = check_contract(&clauses);
        assert!(checklist[0].found);
        assert!(!checklist[1].found);
        assert!(!checklist[2].found);
        assert!(!checklist[3].found);
        assert!(!checklist[4].found);
    }

    #[test]
    fn contract_checklist_only_limitation() {
        let clauses = vec![make_clause("1", ClauseType::Limitation, RiskLevel::Medium)];
        let checklist = check_contract(&clauses);
        assert!(!checklist[0].found);
        assert!(checklist[1].found);
        assert!(!checklist[2].found);
    }

    #[test]
    fn contract_checklist_only_confidentiality() {
        let clauses = vec![make_clause(
            "1",
            ClauseType::Confidentiality,
            RiskLevel::Low,
        )];
        let checklist = check_contract(&clauses);
        assert!(!checklist[0].found);
        assert!(checklist[3].found);
    }

    #[test]
    fn contract_checklist_only_governing() {
        let clauses = vec![make_clause("1", ClauseType::Governing, RiskLevel::Low)];
        let checklist = check_contract(&clauses);
        assert!(checklist[4].found);
        assert!(!checklist[0].found);
    }

    #[test]
    fn contract_checklist_non_required_types_ignored() {
        // Ip, Warranty, Other はチェックリストに含まれない
        let clauses = vec![
            make_clause("1", ClauseType::Ip, RiskLevel::Low),
            make_clause("2", ClauseType::Warranty, RiskLevel::Low),
            make_clause("3", ClauseType::Other, RiskLevel::Low),
        ];
        let checklist = check_contract(&clauses);
        for item in &checklist {
            assert!(!item.found);
        }
    }

    #[test]
    fn contract_checklist_duplicate_types() {
        // 同じ種類の条項が複数あってもfound=true
        let clauses = vec![
            make_clause("1", ClauseType::Termination, RiskLevel::Low),
            make_clause("2", ClauseType::Termination, RiskLevel::Medium),
        ];
        let checklist = check_contract(&clauses);
        assert!(checklist[2].found); // Termination
    }

    #[test]
    fn contract_checklist_all_required() {
        // 全項目がrequired=true
        let checklist = check_contract(&[]);
        for item in &checklist {
            assert!(item.required);
        }
    }

    // =======================================================================
    // contract_risk_score テスト (12件)
    // =======================================================================

    #[test]
    fn risk_score_empty() {
        // 空リストは0.0
        assert!((contract_risk_score(&[]) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn risk_score_all_low() {
        let clauses = vec![make_clause("1", ClauseType::Other, RiskLevel::Low)];
        let score = contract_risk_score(&clauses);
        // Low=1, score = 1/(1*10)*100 = 10.0
        assert!((score - 10.0).abs() < 0.01);
    }

    #[test]
    fn risk_score_all_medium() {
        let clauses = vec![make_clause("1", ClauseType::Other, RiskLevel::Medium)];
        let score = contract_risk_score(&clauses);
        // Medium=3, score = 3/(1*10)*100 = 30.0
        assert!((score - 30.0).abs() < 0.01);
    }

    #[test]
    fn risk_score_all_high() {
        let clauses = vec![make_clause("1", ClauseType::Other, RiskLevel::High)];
        let score = contract_risk_score(&clauses);
        // High=7, score = 7/(1*10)*100 = 70.0
        assert!((score - 70.0).abs() < 0.01);
    }

    #[test]
    fn risk_score_all_critical() {
        let clauses = vec![make_clause("1", ClauseType::Other, RiskLevel::Critical)];
        let score = contract_risk_score(&clauses);
        assert!((score - 100.0).abs() < 0.01);
    }

    #[test]
    fn risk_score_mixed_low_critical() {
        // Low(1) + Critical(10) → 11/(2*10)*100 = 55.0
        let clauses = vec![
            make_clause("1", ClauseType::Other, RiskLevel::Low),
            make_clause("2", ClauseType::Other, RiskLevel::Critical),
        ];
        let score = contract_risk_score(&clauses);
        assert!((score - 55.0).abs() < 0.01);
    }

    #[test]
    fn risk_score_mixed_three() {
        // Low(1) + Medium(3) + High(7) → 11/(3*10)*100 ≈ 36.67
        let clauses = vec![
            make_clause("1", ClauseType::Other, RiskLevel::Low),
            make_clause("2", ClauseType::Other, RiskLevel::Medium),
            make_clause("3", ClauseType::Other, RiskLevel::High),
        ];
        let score = contract_risk_score(&clauses);
        assert!((score - 100.0 * 11.0 / 30.0).abs() < 0.01);
    }

    #[test]
    fn risk_score_all_four_levels() {
        // Low(1)+Medium(3)+High(7)+Critical(10) → 21/(4*10)*100 = 52.5
        let clauses = vec![
            make_clause("1", ClauseType::Other, RiskLevel::Low),
            make_clause("2", ClauseType::Other, RiskLevel::Medium),
            make_clause("3", ClauseType::Other, RiskLevel::High),
            make_clause("4", ClauseType::Other, RiskLevel::Critical),
        ];
        let score = contract_risk_score(&clauses);
        assert!((score - 52.5).abs() < 0.01);
    }

    #[test]
    fn risk_score_multiple_low() {
        // Low*5 → 5/(5*10)*100 = 10.0
        let clauses: Vec<Clause> = (0..5)
            .map(|i| make_clause(&alloc::format!("{i}"), ClauseType::Other, RiskLevel::Low))
            .collect();
        let score = contract_risk_score(&clauses);
        assert!((score - 10.0).abs() < 0.01);
    }

    #[test]
    fn risk_score_multiple_critical() {
        // Critical*3 → 30/(3*10)*100 = 100.0
        let clauses: Vec<Clause> = (0..3)
            .map(|i| {
                make_clause(
                    &alloc::format!("{i}"),
                    ClauseType::Other,
                    RiskLevel::Critical,
                )
            })
            .collect();
        let score = contract_risk_score(&clauses);
        assert!((score - 100.0).abs() < 0.01);
    }

    #[test]
    fn risk_score_range_0_to_100() {
        // どんな組み合わせでも0〜100の範囲
        let clauses = vec![
            make_clause("1", ClauseType::Other, RiskLevel::Low),
            make_clause("2", ClauseType::Other, RiskLevel::High),
        ];
        let score = contract_risk_score(&clauses);
        assert!(score >= 0.0);
        assert!(score <= 100.0);
    }

    #[test]
    fn risk_score_clause_type_does_not_affect_score() {
        // ClauseTypeはスコアに影響しない（RiskLevelのみ）
        let c1 = vec![make_clause("1", ClauseType::Ip, RiskLevel::Medium)];
        let c2 = vec![make_clause("1", ClauseType::Warranty, RiskLevel::Medium)];
        let s1 = contract_risk_score(&c1);
        let s2 = contract_risk_score(&c2);
        assert!((s1 - s2).abs() < 1e-10);
    }

    // =======================================================================
    // LegalError テスト (5件)
    // =======================================================================

    #[test]
    fn legal_error_display_parse() {
        let e = LegalError::ParseError;
        assert_eq!(alloc::format!("{e}"), "parse error");
    }

    #[test]
    fn legal_error_display_classification() {
        let e = LegalError::ClassificationFailed;
        assert_eq!(alloc::format!("{e}"), "classification failed");
    }

    #[test]
    fn legal_error_debug() {
        let e = LegalError::ParseError;
        let debug = alloc::format!("{e:?}");
        assert!(debug.contains("ParseError"));
    }

    #[test]
    fn legal_error_clone() {
        let e1 = LegalError::ClassificationFailed;
        let e2 = e1.clone();
        assert_eq!(e1, e2);
    }

    #[test]
    fn legal_error_eq() {
        assert_eq!(LegalError::ParseError, LegalError::ParseError);
        assert_ne!(LegalError::ParseError, LegalError::ClassificationFailed);
    }

    // =======================================================================
    // 構造体・Enum テスト (8件)
    // =======================================================================

    #[test]
    fn clause_clone() {
        let c = make_clause("1", ClauseType::Ip, RiskLevel::High);
        let c2 = c.clone();
        assert_eq!(c, c2);
    }

    #[test]
    fn clause_debug() {
        let c = make_clause("1", ClauseType::Other, RiskLevel::Low);
        let debug = alloc::format!("{c:?}");
        assert!(debug.contains("Clause"));
    }

    #[test]
    fn clause_type_copy() {
        let ct = ClauseType::Warranty;
        let ct2 = ct; // Copy
        assert_eq!(ct, ct2);
    }

    #[test]
    fn risk_level_ord() {
        // RiskLevelの順序: Low < Medium < High < Critical
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn risk_level_copy() {
        let rl = RiskLevel::Critical;
        let rl2 = rl; // Copy
        assert_eq!(rl, rl2);
    }

    #[test]
    fn checklist_item_clone() {
        let item = ChecklistItem {
            name: String::from("test"),
            required: true,
            found: false,
        };
        let item2 = item.clone();
        assert_eq!(item, item2);
    }

    #[test]
    fn checklist_item_debug() {
        let item = ChecklistItem {
            name: String::from("test"),
            required: true,
            found: true,
        };
        let debug = alloc::format!("{item:?}");
        assert!(debug.contains("ChecklistItem"));
    }

    #[test]
    fn clause_fields_accessible() {
        // 各フィールドにアクセスできることを確認
        let c = Clause {
            id: String::from("42"),
            section: String::from("7A"),
            text: String::from("sample text"),
            clause_type: ClauseType::Governing,
            risk_level: RiskLevel::Medium,
        };
        assert_eq!(c.id, "42");
        assert_eq!(c.section, "7A");
        assert_eq!(c.text, "sample text");
        assert_eq!(c.clause_type, ClauseType::Governing);
        assert_eq!(c.risk_level, RiskLevel::Medium);
    }

    // =======================================================================
    // sqrt_approx 間接テスト (5件)
    // =======================================================================

    #[test]
    fn cosine_with_large_values() {
        // sqrt_approxが大きな値でも正しく動作することの間接確認
        let mut tf1 = BTreeMap::new();
        let mut tf2 = BTreeMap::new();
        tf1.insert(String::from("a"), 100.0);
        tf2.insert(String::from("a"), 100.0);
        let sim = cosine_similarity(&tf1, &tf2);
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn cosine_with_small_values() {
        // 非常に小さな値でもsqrt_approxが正しく動作
        let mut tf1 = BTreeMap::new();
        let mut tf2 = BTreeMap::new();
        tf1.insert(String::from("a"), 0.001);
        tf2.insert(String::from("a"), 0.001);
        let sim = cosine_similarity(&tf1, &tf2);
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn cosine_with_unequal_magnitudes() {
        // 異なる大きさのベクトルでも正しく正規化される
        let mut tf1 = BTreeMap::new();
        let mut tf2 = BTreeMap::new();
        tf1.insert(String::from("a"), 1.0);
        tf2.insert(String::from("a"), 1000.0);
        let sim = cosine_similarity(&tf1, &tf2);
        // 方向が同じなので類似度は1.0
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        // 直交ベクトル（共通キーなし）は類似度0
        let mut tf1 = BTreeMap::new();
        let mut tf2 = BTreeMap::new();
        tf1.insert(String::from("a"), 1.0);
        tf2.insert(String::from("b"), 1.0);
        let sim = cosine_similarity(&tf1, &tf2);
        assert!((sim - 0.0).abs() < 1e-10);
    }

    #[test]
    fn cosine_multi_dimensional() {
        // 多次元ベクトルでの類似度計算
        let mut tf1 = BTreeMap::new();
        let mut tf2 = BTreeMap::new();
        // tf1 = (1, 1, 0), tf2 = (1, 0, 1)
        tf1.insert(String::from("a"), 1.0);
        tf1.insert(String::from("b"), 1.0);
        tf2.insert(String::from("a"), 1.0);
        tf2.insert(String::from("c"), 1.0);
        let sim = cosine_similarity(&tf1, &tf2);
        // dot=1, |v1|=sqrt(2), |v2|=sqrt(2), sim=1/2=0.5
        assert!((sim - 0.5).abs() < 0.01);
    }

    // =======================================================================
    // 統合テスト: classify + assess を組み合わせ (5件)
    // =======================================================================

    #[test]
    fn integration_classify_then_assess_indemnification() {
        // 分類とリスク評価を組み合わせた統合テスト
        let text = "The vendor shall indemnify with unlimited liability";
        assert_eq!(classify_clause(text), ClauseType::Indemnification);
        assert_eq!(assess_risk(text), RiskLevel::Critical);
    }

    #[test]
    fn integration_classify_then_assess_termination_medium() {
        let text = "Either party may terminate upon material breach";
        assert_eq!(classify_clause(text), ClauseType::Termination);
        assert_eq!(assess_risk(text), RiskLevel::Medium);
    }

    #[test]
    fn integration_classify_then_assess_low_risk_warranty() {
        let text = "This warranty covers defects in materials";
        assert_eq!(classify_clause(text), ClauseType::Warranty);
        assert_eq!(assess_risk(text), RiskLevel::Low);
    }

    #[test]
    fn integration_full_contract_workflow() {
        // 完全な契約分析ワークフロー
        let texts = [
            "The vendor shall indemnify the client",
            "Limitation of liability is capped at fees paid",
            "Either party may terminate with 30 days notice",
            "All confidential information must be protected",
            "Governing law shall be Japan",
        ];
        let clauses: Vec<Clause> = texts
            .iter()
            .enumerate()
            .map(|(i, &t)| Clause {
                id: alloc::format!("{i}"),
                section: alloc::format!("{}", i + 1),
                text: String::from(t),
                clause_type: classify_clause(t),
                risk_level: assess_risk(t),
            })
            .collect();

        // 全必須条項が揃っている
        let checklist = check_contract(&clauses);
        for item in &checklist {
            assert!(item.found);
        }

        // リスクスコアが計算可能
        let score = contract_risk_score(&clauses);
        assert!(score > 0.0);
        assert!(score <= 100.0);
    }

    #[test]
    fn integration_similarity_between_related_clauses() {
        // 関連する条項間のTF-IDF類似度
        let tf1 = term_frequency(&tokenize(
            "The vendor shall indemnify the client against all claims",
        ));
        let tf2 = term_frequency(&tokenize(
            "The vendor shall hold harmless the client against losses",
        ));
        let tf3 = term_frequency(&tokenize("Governing law shall be the State of California"));
        // 類似した条項は高い類似度
        let sim_related = cosine_similarity(&tf1, &tf2);
        // 無関係な条項は低い類似度
        let sim_unrelated = cosine_similarity(&tf1, &tf3);
        assert!(sim_related > sim_unrelated);
    }

    // =======================================================================
    // チェックリスト名前テスト (3件)
    // =======================================================================

    #[test]
    fn checklist_names_correct() {
        let checklist = check_contract(&[]);
        assert_eq!(checklist[0].name, "Indemnification");
        assert_eq!(checklist[1].name, "Limitation of Liability");
        assert_eq!(checklist[2].name, "Termination");
        assert_eq!(checklist[3].name, "Confidentiality");
        assert_eq!(checklist[4].name, "Governing Law");
    }

    #[test]
    fn checklist_item_eq() {
        let a = ChecklistItem {
            name: String::from("X"),
            required: true,
            found: true,
        };
        let b = ChecklistItem {
            name: String::from("X"),
            required: true,
            found: true,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn checklist_item_ne() {
        let a = ChecklistItem {
            name: String::from("X"),
            required: true,
            found: true,
        };
        let b = ChecklistItem {
            name: String::from("X"),
            required: true,
            found: false,
        };
        assert_ne!(a, b);
    }
}
