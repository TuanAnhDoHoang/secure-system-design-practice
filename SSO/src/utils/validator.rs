use regex::Regex;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RegisterValidationErrors {
    pub email_error: Option<String>,
    pub password_error: Option<String>,
    pub confirm_password_error: Option<String>,
}

impl RegisterValidationErrors {
    pub fn is_empty(&self) -> bool {
        self.email_error.is_none()
            && self.password_error.is_none()
            && self.confirm_password_error.is_none()
    }
}

/// Kiểm tra định dạng email hợp lệ theo RFC
pub fn email_validation(email: &str) -> bool {
    let re = Regex::new(r"^[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}$")
        .expect("email validation regex should be valid");
    re.is_match(email.trim())
}

/// Validate các trường nhập của form đăng ký:
/// - Email hợp lệ và không rỗng
/// - Mật khẩu tối thiểu 8 ký tự, gồm ít nhất 1 chữ hoa, 1 chữ thường, 1 chữ số
/// - Mật khẩu xác nhận phải khớp hoàn toàn với mật khẩu
pub fn validate_register_input(
    email: &str,
    password: &str,
    confirm_password: &str,
) -> Result<(), RegisterValidationErrors> {
    let mut errors = RegisterValidationErrors::default();
    let email_trimmed = email.trim();

    // 1. Validate email
    if email_trimmed.is_empty() {
        errors.email_error = Some("Vui lòng nhập địa chỉ email.".to_string());
    } else if !email_validation(email_trimmed) || email_trimmed.len() > 254 {
        errors.email_error = Some("Địa chỉ email không đúng định dạng.".to_string());
    }

    // 2. Validate password strength
    let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let is_long_enough = password.len() >= 8;

    if password.is_empty() {
        errors.password_error = Some("Vui lòng nhập mật khẩu.".to_string());
    } else if !is_long_enough || !has_uppercase || !has_lowercase || !has_digit {
        errors.password_error = Some(
            "Mật khẩu phải có ít nhất 8 ký tự, bao gồm chữ hoa, chữ thường và chữ số.".to_string(),
        );
    }

    // 3. Validate confirm password
    if confirm_password.is_empty() {
        errors.confirm_password_error = Some("Vui lòng nhập lại mật khẩu xác nhận.".to_string());
    } else if password != confirm_password {
        errors.confirm_password_error = Some("Mật khẩu xác nhận không khớp.".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
