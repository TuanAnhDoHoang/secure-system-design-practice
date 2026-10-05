use bcrypt::{DEFAULT_COST, hash, verify};

/// Hash mật khẩu bằng thuật toán bcrypt
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    hash(password, DEFAULT_COST).map_err(|e| anyhow::anyhow!("Bcrypt hash error: {}", e))
}

/// Kiểm tra mật khẩu plain-text với bcrypt hash
pub fn verify_password(password: &str, hashed: &str) -> anyhow::Result<bool> {
    verify(password, hashed).map_err(|e| anyhow::anyhow!("Bcrypt verify error: {}", e))
}
