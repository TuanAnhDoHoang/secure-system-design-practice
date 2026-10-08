use rpassword::read_password;

fn prompt_input(prompt: &str) -> String {
    print!("{}", prompt);
    std::io::Write::flush(&mut std::io::stdout()).ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    input.trim().to_string()
}

#[tokio::main]
async fn main() {
    let server = prompt_input("Server URL (default http://127.0.0.1:3000): ");
    let server = if server.is_empty() { "http://127.0.0.1:3000".to_string() } else { server };

    loop {
        println!("\nChoose action:\n1) Register\n2) Login\n3) Logout\n4) Quit\nEnter number:");
        let mut sel = String::new();
        std::io::stdin().read_line(&mut sel).ok();
        match sel.trim() {
            "1" => {
                let user = prompt_input("Username: ");
                print!("Password: ");
                std::io::Write::flush(&mut std::io::stdout()).ok();
                let pass = read_password().expect("Failed to read password");
                match rbac::client::register(&server, &user, &pass).await {
                    Ok(resp) => println!("Registered user id={}", resp.user_id),
                    Err(e) => eprintln!("Register failed: {}", e),
                }
            }
            "2" => {
                let user = prompt_input("Username: ");
                print!("Password: ");
                std::io::Write::flush(&mut std::io::stdout()).ok();
                let pass = read_password().expect("Failed to read password");
                match rbac::client::login(&server, &user, &pass).await {
                    Ok(resp) => {
                        println!("Logged in: {} (id={})", resp.username, resp.user_id);
                        println!("access_token: {}", resp.access_token);
                        println!("refresh_token: {}", resp.refresh_token);
                    }
                    Err(e) => eprintln!("Login failed: {}", e),
                }
            }
            "3" => {
                println!("Logged out (local only)");
            }
            "4" => break,
            _ => println!("Unknown selection"),
        }
    }
}
