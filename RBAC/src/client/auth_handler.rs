use crate::{client::utils};
use crate::client::LoginResponse;
use rpassword::read_password;

pub async fn register_handler(server: &str) {
    let user = utils::prompt_input("Username: ");
    print!("Password: ");
    std::io::Write::flush(&mut std::io::stdout()).ok();
    let pass = read_password().expect("Failed to read password");
    match crate::client::register(server, &user, &pass).await {
        Ok(resp) => println!("Registered user id={}", resp.user_id),
        Err(e) => eprintln!("Register failed: {}", e),
    }
}

pub async fn login_handler(server: &str) -> anyhow::Result<LoginResponse>{
    let user = crate::client::utils::prompt_input("Username: ");
    print!("Password: ");
    std::io::Write::flush(&mut std::io::stdout()).ok();
    let pass = read_password().expect("Failed to read password");
    match crate::client::login(&server, &user, &pass).await {
        Ok(resp) => {
            println!("Logged in: {} (id={})", resp.username, resp.user_id);
            println!("access_token: {}", resp.access_token);
            println!("refresh_token: {}", resp.refresh_token);
            Ok(resp)
        }
        Err(e) => {
            eprintln!("Login failed: {}", e);
            Err(e)
        }
    }
}
