use std::{
    env,
    io::{Write, stdin, stdout},
    sync::Arc,
};

use ::redis::{
    AsyncCommands, ErrorKind as RedisErrorKind, FromRedisValue, ParsingError, RedisError,
    RedisResult, RedisWrite, ToRedisArgs, ToSingleRedisArg, Value as RedisValue,
    aio::ConnectionManager,
};
use anyhow::{Context, anyhow};
use axum::{
    Json, Router,
    extract::{Query, State, path::ErrorKind},
    http::StatusCode,
    routing::{get, post},
};
use bcrypt::{DEFAULT_COST, hash};
use chrono::{TimeDelta, Utc};
use dotenv::dotenv;
use jsonwebtoken::{Algorithm, EncodingKey, Extras, Header, encode};
use lettre::{
    Message, SmtpTransport, Transport, message::header::ContentType,
    transport::smtp::authentication::Credentials,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use surrealdb::{
    Surreal,
    engine::remote::ws::Client,
    types::{Uuid, Value},
};

use crate::{
    redis::init_redis,
    surreal::{
        connect::connect,
        schema::{NewUser, RegisterRequest, UserStatus, Users},
    },
};

mod redis;
mod surreal;

fn read_email(email: &mut String) -> anyhow::Result<()> {
    print!(">> Email: ");
    stdout().flush().context("Error during flush stdout")?;
    stdin()
        .read_line(email)
        .context("Error during get email as input")?;
    Ok(())
}
fn read_password(password: &mut String) -> anyhow::Result<()> {
    print!(">> Password: ");
    stdout().flush().context("Error during flush stdout")?;
    stdin()
        .read_line(password)
        .context("Error during get password as input")?;
    Ok(())
}

#[derive(Serialize)]
struct Claim {
    sub: String,
    exp: usize,
}

fn login() -> anyhow::Result<()> {
    let mut email = String::new();
    let mut password = String::new();

    read_email(&mut email)?;
    read_password(&mut password)?;

    let now = Utc::now().naive_utc();
    let access_expires_time = now + TimeDelta::hours(1);

    let claim = Claim {
        sub: email,
        exp: access_expires_time.and_utc().timestamp() as usize,
    };

    let mut header = Header::new(Algorithm::HS512);
    header.kid = Some("blabla".to_owned());

    let mut extras = Extras::default();
    extras.insert("custom".to_string(), "header".to_string());
    header.extras = extras;

    let token = encode(
        &header,
        &claim,
        &EncodingKey::from_secret("secret".as_ref()),
    )?;
    log::error!("Token: {}", token);
    Ok(())
}
#[derive(Clone)]
struct AppState {
    db: Surreal<Client>,
    smtp_transport: SmtpTransport,
    redis: ConnectionManager,
}

fn init_stmp() -> SmtpTransport {
    let smtp_username = "your_email@gmail.com".to_string();
    let smtp_password = "your_app_password".to_string(); // Sử dụng App Password nếu dùng Gmail
    let smtp_server = "://gmail.com";

    let creds = Credentials::new(smtp_username, smtp_password);
    let mailer = SmtpTransport::relay(smtp_server)
        .unwrap()
        .credentials(creds)
        .build();
    mailer
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let db = connect().await?;
    let smtp_transport = init_stmp();
    let redis = init_redis().await?;
    let state = Arc::new(AppState {
        db,
        smtp_transport,
        redis,
    });

    let endpoint = Router::new()
        .route("/register", post(register))
        .route("/verify_token", get(verify_token))
        .route("/authorize", get(authorize))
        .route("/token", post(token))
        .route("/userinfo", get(userinfo))
        .route("/.well-know/jwks.json", get(well_know))
        .route("/revoke", get(revoke))
        .route("/logout", get(logout))
        .with_state(state);

    let app = endpoint;

    let host_url = "localhost:3000";

    let listener = tokio::net::TcpListener::bind(&host_url)
        .await
        .context("Error during start server")?;

    log::error!("Server running on {}", host_url);
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
//Struct
#[derive(Deserialize)]
pub struct AuthorizeParams {
    client_id: String,
    redirect_uri: String,
    code_challenge: String,
}

fn email_validation(email: &str) -> bool {
    let re = Regex::new(r"^[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}$")
        .expect("email validation regex should be valid");
    re.is_match(email.trim())
}

async fn check_email(email: &str, db: &Surreal<Client>) -> anyhow::Result<bool> {
    let query = "select email from users where email = $email";
    let mut result = db.query(query).bind(("email", email)).await?;
    let exist_email = result.take::<Option<String>>(0 as usize)?;
    if exist_email.is_some() {
        return Ok(false);
    }
    Ok(true)
}

#[derive(Serialize)]
pub struct RegisterResponse {
    message: String,
}

struct AttemptTokenVerify {
    token_hash: String,
    password_hash: String,
    attempt: usize,
}
impl ToRedisArgs for AttemptTokenVerify {
    fn write_redis_args<W: ?Sized + RedisWrite>(&self, out: &mut W) {
        // Convert your type into a byte representation for a single argument
        out.write_arg(
            format!(
                "token_hash:{};password:{};attempt:{}",
                &self.token_hash, &self.password_hash, &self.attempt
            )
            .as_bytes(),
        );
    }
}

// Implementing this trait marks that your type serializes into strictly one Redis argument
impl ToSingleRedisArg for AttemptTokenVerify {}

impl FromRedisValue for AttemptTokenVerify {
    fn from_redis_value(v: RedisValue) -> Result<Self, ParsingError> {
        // v giờ là owned, gọi thẳng, không cần .clone()
        let s = String::from_redis_value(v).map_err(|e| {
            log::error!("Error during parse redis value: {:?}", e);
            ParsingError::from("Error convert value")
        })?;

        let parts: Vec<&str> = s.split(';').collect();
        if parts.len() != 3 {
            return Err(ParsingError::from("Error convert value"));
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

async fn save_token_verify_session(
    validated_email: &str,
    password_hash: &str,
    token: &str,
    state: Arc<AppState>,
) -> anyhow::Result<()> {
    let token_hash = hash(token, DEFAULT_COST)?;

    //Save to redis <email:code>
    let mut redis = state.redis.clone();
    let key = format!("pending_verify:{}", validated_email);
    let value = AttemptTokenVerify {
        token_hash,
        password_hash: password_hash.to_string(),
        attempt: 1,
    };

    let expire_time_env = env::var("EXPIRE_TIME_EMAIL_VALIDATE_TOKEN")?;
    let expire_time = expire_time_env.parse::<u64>()?;
    redis.set_ex::<_, _, ()>(&key, value, expire_time).await?;
    Ok(())
}

async fn send_verify_email_token(
    validated_email: &str,
    state: Arc<AppState>,
) -> anyhow::Result<String> {
    let verify_token = Uuid::new_v4().to_string();
    let host_url = env::var("VERIFICATION_LINK")?;
    let verification_link = format!(
        "{}/verify?email={}&token={}",
        host_url, validated_email, verify_token
    );
    let email = Message::builder()
        .from("No-Reply <your_email@gmail.com>".parse().unwrap())
        .to(validated_email.parse().unwrap())
        .subject("Xác thực tài khoản của bạn")
        .header(ContentType::TEXT_HTML)
        .body(format!(
            "<h3>Chào mừng bạn!</h3>\
             <p>Vui lòng nhấn vào đường link bên dưới để xác thực tài khoản:</p>\
             <a href='{}'>Kích hoạt tài khoản</a>",
            verification_link
        ))
        .unwrap();
    let mailer = state.smtp_transport.clone();
    let send_result = tokio::task::spawn_blocking(move || mailer.send(&email)).await?;

    match send_result {
        Ok(_) => Ok(verify_token),
        Err(e) => {
            log::error!("Error during send mail to client {e}");
            return Err(anyhow!("Error during send verify token"));
        }
    }
}

async fn send_warn(validated_email: &str, state: Arc<AppState>) -> anyhow::Result<()> {
    let login_link = env::var("HOST_URL")?;
    let my_email = env::var("MY_EMAIL")?;
    let email = Message::builder()
        .from(format!("No-Reply <{}>", my_email).parse().unwrap())
        .to(validated_email.parse().unwrap())
        .subject("Email using warn")
        .header(ContentType::TEXT_HTML)
        .body(format!(
            "<h3>Chào mừng bạn!</h3>\
             <p>Somebody are trying to login by your email. You can following this url to login: /p>\
             <a href='{}'>Login</a>",
            login_link
        ))
        .unwrap();
    let mailer = state.smtp_transport.clone();
    let send_result = tokio::task::spawn_blocking(move || mailer.send(&email)).await?;

    match send_result {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow!("Error during send verify code {e}")),
    }
}

#[derive(Deserialize)]
struct VerifyQuery {
    email: String,
    token: String,
}

#[derive(Serialize)]
struct VerifyResponse {
    user_id: String,
}
async fn verify_token(
    State(state): State<Arc<AppState>>,
    Query(query): Query<VerifyQuery>,
) -> anyhow::Result<Json<VerifyResponse>, (StatusCode, String)> {
    let email = query.email;
    let token = query.token;
    let mut redis_conn = state.redis.clone();

    let key = format!("pending_verify:{}", &email);
    let saved_records: Option<AttemptTokenVerify> = redis_conn.get(&key).await.map_err(|e| {
        log::error!("Error during redis get {}: {}", email, e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Error during verify".to_string(),
        )
    })?;

    let check_token = match &saved_records {
        Some(verify_attempt) => {
            bcrypt::verify(&token, &verify_attempt.token_hash).map_err(|e| {
                log::error!("Error during verify token {}: {}", email, e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error during verify".to_string(),
                )
            })?
        }
        None => false,
    };

    if check_token {
        let _: () = redis_conn.del(&key).await.unwrap_or(());
        let password_hash = saved_records.unwrap().password_hash;
        let user_id = create_user(state.clone(), &email, &password_hash)
            .await
            .map_err(|e| {
                log::error!("Error during create user {}: {}", email, e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error during verify".to_string(),
                )
            })?;
        Ok(Json(VerifyResponse {
            user_id,
        }))
    } else {
        log::error!("Token mismatch {}:{}", email, token);
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Wrong verify token".to_string(),
        ))
    }
}
async fn create_user(
    state: Arc<AppState>,
    email: &str,
    password_hash: &str,
) -> anyhow::Result<String> {
    let now = Utc::now();

    let new_user = NewUser {
        email: email.trim().to_string(),
        password_hash: password_hash.to_string(),
        status: UserStatus::Active,
        created_at: now,
        updated_at: now,
    };

    let created: Result<Option<Users>, _> =
        state.db.create("user").content::<NewUser>(new_user).await;

    match created {
        Ok(Some(user)) => Ok(user.id),
        Ok(None) => Err(anyhow!("User was not created")),
        Err(err) => Err(anyhow!(format!(
            "Failed to create user in database: {}",
            err
        ))),
    }
}
async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, (StatusCode, String)> {
    let db = state.db.clone();
    let email = payload.email.trim();
    let password = payload.password.trim();

    if email.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Email and password are required".to_string(),
        ));
    }

    if !email_validation(&email) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Email is invalid. Please use a valid email format".to_string(),
        ));
    }

    let email_exist = check_email(&email, &db).await.map_err(|e| {
        log::error!("Error during register {}: {}", email, e);
        return (StatusCode::BAD_REQUEST, format!("Error during register"));
    })?;

    if !email_exist {
        let verify_token = send_verify_email_token(email, state.clone())
            .await
            .map_err(|e| {
                log::error!(
                    "[REGISTER] Error during send code to email {}: {}",
                    email,
                    e
                );
                return (
                    StatusCode::BAD_REQUEST,
                    format!("Error during sending code"),
                );
            })?;
        save_token_verify_session(&email, &password, &verify_token, state.clone())
            .await
            .map_err(|e| {
                log::error!(
                    "[REGISTER] Error during send code to email {}: {}",
                    email,
                    e
                );
                return (
                    StatusCode::BAD_REQUEST,
                    format!("Error during sending code"),
                );
            })?;
    } else {
        send_warn(email, state).await.map_err(|e| {
            log::error!(
                "[REGISTER] Error during send code to email {}: {}",
                email,
                e
            );
            return (
                StatusCode::BAD_REQUEST,
                format!("Error during sending code"),
            );
        })?
    }

    Ok(Json(RegisterResponse {
        message: "Email has been sent to your email!".to_string(),
    }))
}

//nhận client_id, redirect_uri, code_challenge (PKCE), state, nonce, scope.
//Hiển thị màn hình login, sau khi xác thực thành công thì redirect về SP kèm authorization code (ngắn hạn, dùng 1 lần, ~60 giây).
async fn authorize(Json(params): Json<AuthorizeParams>) -> Result<(StatusCode, String), String> {
    todo!()
}

//SP gửi code + code_verifier để đổi lấy access_token, id_token, refresh_token.
//Đây là nơi verify PKCE: hash code_verifier bằng SHA256 rồi so với code_challenge đã lưu lúc /authorize.
async fn token() -> Result<(StatusCode, String), String> {
    todo!()
}

//trả thông tin user dựa trên access_token hợp lệ.
async fn userinfo() -> Result<(StatusCode, String), String> {
    todo!()
}

//public keys để SP/Resource Server tự verify JWT mà không cần gọi ngược về IdP mỗi request.
async fn well_know() -> Result<(StatusCode, String), String> {
    todo!()
}

//thu hồi token, thực hiện Single Logout
async fn revoke() -> Result<(StatusCode, String), String> {
    todo!()
}

//thu hồi token, thực hiện Single Logout
async fn logout() -> Result<(StatusCode, String), String> {
    revoke().await
}
