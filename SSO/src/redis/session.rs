use redis::{
    AsyncCommands, FromRedisValue, ParsingError, RedisWrite, ToRedisArgs, ToSingleRedisArg,
    Value as RedisValue, aio::ConnectionManager,
};

#[derive(Debug, Clone)]
pub struct AttemptTokenVerify {
    pub token_hash: String,
    pub password_hash: String,
    pub attempt: usize,
}

impl ToRedisArgs for AttemptTokenVerify {
    fn write_redis_args<W: ?Sized + RedisWrite>(&self, out: &mut W) {
        out.write_arg(
            format!(
                "token_hash:{};password:{};attempt:{}",
                self.token_hash, self.password_hash, self.attempt
            )
            .as_bytes(),
        );
    }
}

impl ToSingleRedisArg for AttemptTokenVerify {}

impl FromRedisValue for AttemptTokenVerify {
    fn from_redis_value(v: RedisValue) -> Result<Self, ParsingError> {
        let s = String::from_redis_value(v).map_err(|e| {
            log::error!("Error parsing redis value: {:?}", e);
            ParsingError::from("Error converting redis value")
        })?;

        let parts: Vec<&str> = s.split(';').collect();
        if parts.len() != 3 {
            return Err(ParsingError::from("Invalid redis record format"));
        }

        let token_hash = parts[0]
            .strip_prefix("token_hash:")
            .ok_or_else(|| ParsingError::from("Missing token_hash prefix"))?
            .to_string();

        let password_hash = parts[1]
            .strip_prefix("password:")
            .ok_or_else(|| ParsingError::from("Missing password prefix"))?
            .to_string();

        let attempt = parts[2]
            .strip_prefix("attempt:")
            .ok_or_else(|| ParsingError::from("Missing attempt prefix"))?
            .parse::<usize>()
            .map_err(|_| ParsingError::from("Invalid attempt number"))?;

        Ok(AttemptTokenVerify {
            token_hash,
            password_hash,
            attempt,
        })
    }
}

fn pending_verify_key(email: &str) -> String {
    format!("pending_verify:{}", email.trim())
}

/// Lưu phiên xác thực tạm thời vào Redis với TTL
pub async fn save_pending_verification(
    redis: &mut ConnectionManager,
    email: &str,
    token_hash: String,
    password_hash: String,
    expire_seconds: u64,
) -> anyhow::Result<()> {
    let key = pending_verify_key(email);
    let record = AttemptTokenVerify {
        token_hash,
        password_hash,
        attempt: 1,
    };
    redis
        .set_ex::<_, _, ()>(&key, record, expire_seconds)
        .await?;
    Ok(())
}

/// Lấy phiên xác thực tạm thời từ Redis
pub async fn get_pending_verification(
    redis: &mut ConnectionManager,
    email: &str,
) -> anyhow::Result<Option<AttemptTokenVerify>> {
    let key = pending_verify_key(email);
    let record: Option<AttemptTokenVerify> = redis.get(&key).await?;
    Ok(record)
}

/// Xóa phiên xác thực tạm thời khỏi Redis khi hoàn tất
pub async fn delete_pending_verification(
    redis: &mut ConnectionManager,
    email: &str,
) -> anyhow::Result<()> {
    let key = pending_verify_key(email);
    let _: () = redis.del(&key).await.unwrap_or(());
    Ok(())
}
