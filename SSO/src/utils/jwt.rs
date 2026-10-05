use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

use crate::models::Claim;

/// Tạo JWT token cho subject với thời gian sống (giờ)
pub fn create_jwt(subject: &str, secret: &str, expires_in_hours: i64) -> anyhow::Result<String> {
    let now = Utc::now();
    let expires_time = now + Duration::hours(expires_in_hours);

    let claim = Claim {
        sub: subject.to_string(),
        exp: expires_time.timestamp() as usize,
    };

    let header = Header::new(Algorithm::HS512);
    let token = encode(
        &header,
        &claim,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

/// Xác thực JWT token và trả về Claim
pub fn verify_jwt(token: &str, secret: &str) -> anyhow::Result<Claim> {
    use jsonwebtoken::{DecodingKey, Validation, decode};
    let validation = Validation::new(Algorithm::HS512);
    let token_data = decode::<Claim>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;
    Ok(token_data.claims)
}
