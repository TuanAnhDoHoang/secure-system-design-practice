use uuid::Uuid;

use crate::{db, error::AppError, redis, services::email_service, state::AppState, utils};

pub struct AuthService;

impl AuthService {
    /// Xử lý quy trình đăng ký người dùng:
    /// - Kiểm tra email tồn tại
    /// - Nếu chưa tồn tại: sinh verify token, hash mật khẩu, lưu Redis và gửi email
    /// - Nếu đã tồn tại: gửi email cảnh báo bảo mật (chống tiết lộ danh tính - user enumeration)
    pub async fn register(state: &AppState, email: &str, password: &str) -> Result<(), AppError> {
        let email_trimmed = email.trim();
        let is_available = db::is_email_available(&state.db, email_trimmed)
            .await
            .map_err(|e| AppError::Internal(format!("Error checking email existence: {}", e)))?;

        if is_available {
            let verify_token = Uuid::new_v4().to_string();
            let password_hash = utils::hash_password(password)
                .map_err(|e| AppError::Internal(format!("Error hashing password: {}", e)))?;
            let token_hash = utils::hash_password(&verify_token)
                .map_err(|e| AppError::Internal(format!("Error hashing verify token: {}", e)))?;

            // Gửi email xác thực
            email_service::send_verification_email(
                &state.mailer,
                &state.config.mail,
                email_trimmed,
                &verify_token,
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to send verification email: {}", e)))?;

            // Lưu phiên đăng ký tạm vào Redis
            let mut redis = state.redis.clone();
            redis::save_pending_verification(
                &mut redis,
                email_trimmed,
                token_hash,
                password_hash,
                state.config.auth.token_expire_seconds,
            )
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to save pending verification: {}", e))
            })?;
        } else {
            // Gửi email cảnh báo tài khoản đã tồn tại
            email_service::send_warning_email(&state.mailer, &state.config.mail, email_trimmed)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to send warning email: {}", e)))?;
        }

        Ok(())
    }

    /// Xử lý quy trình xác minh token / mã OTP 6 số:
    /// - Tìm phiên chờ trong Redis
    /// - So khớp token hash với bcrypt
    /// - Tạo tài khoản vĩnh viễn trong SurrealDB và xóa khỏi Redis
    ///
    /// Trả về user_id của người dùng mới tạo
    pub async fn verify_token(
        state: &AppState,
        email: &str,
        token: &str,
    ) -> Result<String, AppError> {
        let email_trimmed = email.trim();
        let token_trimmed = token.trim();

        let mut redis = state.redis.clone();
        let pending = redis::get_pending_verification(&mut redis, email_trimmed)
            .await
            .map_err(|e| AppError::Internal(format!("Redis query failed: {}", e)))?;

        let record = match pending {
            Some(rec) => rec,
            None => {
                return Err(AppError::BadRequest("expired".to_string()));
            }
        };

        let is_valid = utils::verify_password(token_trimmed, &record.token_hash).unwrap_or(false);

        if !is_valid {
            log::warn!("Verify token mismatch for email: {}", email_trimmed);
            return Err(AppError::BadRequest("invalid".to_string()));
        }

        // Xóa token khỏi Redis
        redis::delete_pending_verification(&mut redis, email_trimmed)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to clean up verification: {}", e)))?;

        // Tạo tài khoản trong database
        let user_id = db::create_user(&state.db, email_trimmed, &record.password_hash)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create user: {}", e)))?;

        Ok(user_id)
    }

    /// Xử lý gửi lại mã xác minh
    pub async fn resend_verification(state: &AppState, email: &str) -> Result<(), AppError> {
        let email_trimmed = email.trim();
        let mut redis = state.redis.clone();

        let pending = redis::get_pending_verification(&mut redis, email_trimmed)
            .await
            .map_err(|e| AppError::Internal(format!("Redis query failed: {}", e)))?;

        let password_hash = match pending {
            Some(rec) => rec.password_hash,
            None => "".to_string(),
        };

        let new_token = Uuid::new_v4().to_string();
        let token_hash = utils::hash_password(&new_token)
            .map_err(|e| AppError::Internal(format!("Error hashing verify token: {}", e)))?;

        email_service::send_verification_email(
            &state.mailer,
            &state.config.mail,
            email_trimmed,
            &new_token,
        )
        .await
        .map_err(|e| AppError::Internal(format!("Failed to send verification email: {}", e)))?;

        redis::save_pending_verification(
            &mut redis,
            email_trimmed,
            token_hash,
            password_hash,
            state.config.auth.token_expire_seconds,
        )
        .await
        .map_err(|e| AppError::Internal(format!("Failed to update pending verification: {}", e)))?;

        Ok(())
    }

    /// Xác thực đăng nhập bằng email và mật khẩu
    pub async fn login(
        state: &AppState,
        email: &str,
        password: &str,
    ) -> Result<crate::models::Users, AppError> {
        let email_trimmed = email.trim();
        if email_trimmed.is_empty() || password.is_empty() {
            return Err(AppError::BadRequest(
                "Vui lòng điền đầy đủ email và mật khẩu".to_string(),
            ));
        }

        let user = db::find_user_by_email(&state.db, email_trimmed)
            .await
            .map_err(|e| AppError::Internal(format!("Database error during login: {}", e)))?
            .ok_or_else(|| {
                AppError::BadRequest("Email hoặc mật khẩu không chính xác".to_string())
            })?;

        if user.status != crate::models::UserStatus::Active {
            return Err(AppError::BadRequest(
                "Tài khoản chưa được kích hoạt hoặc đã bị khóa".to_string(),
            ));
        }

        let is_valid = utils::verify_password(password, &user.password_hash).unwrap_or(false);

        if !is_valid {
            return Err(AppError::BadRequest(
                "Email hoặc mật khẩu không chính xác".to_string(),
            ));
        }

        Ok(user)
    }
}
