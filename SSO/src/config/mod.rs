use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub mail: MailConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub base_url: String,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub namespace: String,
    pub db_name: String,
}

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct MailConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub sender_email: String,
    pub verification_base_url: String,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub token_expire_seconds: u64,
    pub jwt_secret: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let host = env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("SERVER_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(3000);
        let base_url = env::var("HOST_URL").unwrap_or_else(|_| format!("http://{}:{}", host, port));

        let server = ServerConfig {
            host,
            port,
            base_url: base_url.clone(),
        };

        let database = DatabaseConfig {
            host: env::var("SURREALDB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("SURREALDB_PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(8000),
            username: env::var("SURREALDB_USERNAME").unwrap_or_else(|_| "root".to_string()),
            password: env::var("SURREALDB_PASSWORD").unwrap_or_else(|_| "root".to_string()),
            namespace: env::var("SURREALDB_NAMESPACE").unwrap_or_else(|_| "test".to_string()),
            db_name: env::var("SURREALDB_NAME").unwrap_or_else(|_| "test".to_string()),
        };

        let redis = RedisConfig {
            url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
        };

        let mail = MailConfig {
            smtp_server: env::var("SMTP_SERVER").unwrap_or_else(|_| "localhost".to_string()),
            smtp_port: env::var("SMTP_PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(1025),
            smtp_username: env::var("SMTP_USERNAME")
                .unwrap_or_else(|_| "test".to_string()),
            smtp_password: env::var("SMTP_PASSWORD")
                .unwrap_or_else(|_| "test".to_string()),
            sender_email: env::var("MY_EMAIL")
                .unwrap_or_else(|_| "no-reply@auth.idp.local".to_string()),
            verification_base_url: env::var("VERIFICATION_LINK").unwrap_or(base_url),
        };

        let token_expire_seconds = env::var("EXPIRE_TIME_EMAIL_VALIDATE_TOKEN")
            .ok()
            .and_then(|t| t.parse::<u64>().ok())
            .unwrap_or(900);

        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());

        let auth = AuthConfig {
            token_expire_seconds,
            jwt_secret,
        };

        Ok(Config {
            server,
            database,
            redis,
            mail,
            auth,
        })
    }
}
