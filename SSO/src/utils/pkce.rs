use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

/// Phương thức PKCE được hỗ trợ (RFC 7636)
#[derive(Debug, Clone, PartialEq)]
pub enum CodeChallengeMethod {
    /// S256: code_challenge = BASE64URL(SHA256(code_verifier))  ← khuyến nghị
    S256,
    /// Plain: code_challenge = code_verifier (kém an toàn hơn, không khuyến khích)
    Plain,
}

impl CodeChallengeMethod {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "S256" => Some(CodeChallengeMethod::S256),
            "plain" => Some(CodeChallengeMethod::Plain),
            _ => None,
        }
    }
}

/// Xác minh code_verifier so với code_challenge đã lưu (RFC 7636 §4.6)
///
/// # Arguments
/// * `verifier`  - code_verifier do client gửi lên lúc exchange token
/// * `challenge` - code_challenge đã lưu lúc /authorize
/// * `method`    - S256 hoặc plain
pub fn verify_pkce(verifier: &str, challenge: &str, method: &CodeChallengeMethod) -> bool {
    match method {
        CodeChallengeMethod::S256 => {
            let hash = Sha256::digest(verifier.as_bytes());
            let computed = URL_SAFE_NO_PAD.encode(hash);
            // Dùng constant-time comparison để tránh timing attack
            constant_time_eq(computed.as_bytes(), challenge.as_bytes())
        }
        CodeChallengeMethod::Plain => {
            constant_time_eq(verifier.as_bytes(), challenge.as_bytes())
        }
    }
}

/// So sánh hai byte slice trong constant time (chống timing attack)
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Validate code_verifier theo RFC 7636:
/// - Độ dài 43-128 ký tự
/// - Chỉ gồm: A-Z a-z 0-9 - . _ ~
pub fn validate_code_verifier(verifier: &str) -> bool {
    let len = verifier.len();
    if len < 43 || len > 128 {
        return false;
    }
    verifier
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~'))
}

/// Validate code_challenge (BASE64URL, không padding):
/// - Độ dài 43-128 ký tự
/// - Chỉ gồm: A-Z a-z 0-9 - _
pub fn validate_code_challenge(challenge: &str) -> bool {
    let len = challenge.len();
    if len < 43 || len > 128 {
        return false;
    }
    challenge
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
}
