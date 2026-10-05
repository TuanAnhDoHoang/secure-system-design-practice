use lettre::{
    Message, SmtpTransport, Transport, message::header::ContentType,
    transport::smtp::authentication::Credentials,
};

use crate::config::MailConfig;

/// Khởi tạo SmtpTransport mailer từ cấu hình MailConfig
pub fn init_smtp(config: &MailConfig) -> SmtpTransport {
    let mut builder = SmtpTransport::builder_dangerous(&config.smtp_server).port(config.smtp_port);

    if !config.smtp_username.is_empty() && !config.smtp_password.is_empty() {
        let creds = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());
        builder = builder.credentials(creds);
    }

    builder.build()
}

/// Gửi email chứa đường dẫn xác thực và token kích hoạt
pub async fn send_verification_email(
    mailer: &SmtpTransport,
    config: &MailConfig,
    to_email: &str,
    verify_token: &str,
) -> anyhow::Result<()> {
    let verification_link = format!(
        "{}?email={}&token={}",
        config.verification_base_url, to_email, verify_token
    );

    let email = Message::builder()
        .from(format!("No-Reply <{}>", config.sender_email).parse()?)
        .to(to_email.parse()?)
        .subject("Xác thực tài khoản của bạn - IdP Portal")
        .header(ContentType::TEXT_HTML)
        .body(format!(
            "<h3>Chào mừng bạn!</h3>\
             <p>Vui lòng nhấn vào đường link bên dưới để kích hoạt tài khoản:</p>\
             <p><a href='{0}'>Kích hoạt tài khoản</a></p>",
            verification_link
        ))?;

    let mailer_clone = mailer.clone();
    tokio::task::spawn_blocking(move || mailer_clone.send(&email)).await??;
    Ok(())
}

/// Gửi email cảnh báo khi có người cố tình đăng ký lại email đã tồn tại (chống enumeration)
pub async fn send_warning_email(
    mailer: &SmtpTransport,
    config: &MailConfig,
    to_email: &str,
) -> anyhow::Result<()> {
    let login_link = format!("{}/login", config.verification_base_url);

    let email = Message::builder()
        .from(format!("No-Reply <{}>", config.sender_email).parse()?)
        .to(to_email.parse()?)
        .subject("Thông báo bảo mật: Yêu cầu đăng ký bằng email của bạn")
        .header(ContentType::TEXT_HTML)
        .body(format!(
            "<h3>Thông báo bảo mật</h3>\
             <p>Email này đã được dùng để đăng ký tài khoản tại hệ thống của chúng tôi. Nếu đó là bạn, bạn có thể đăng nhập tại đây:</p>\
             <p><a href='{0}'>Đăng nhập ngay</a></p>",
            login_link
        ))?;

    let mailer_clone = mailer.clone();
    tokio::task::spawn_blocking(move || mailer_clone.send(&email)).await??;
    Ok(())
}
