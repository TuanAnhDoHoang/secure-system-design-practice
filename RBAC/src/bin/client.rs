use rbac::{
    auth_handler::{login_handler, register_handler},
    utils,
};

struct ClientState {
    logged_in: bool,
    user_id: Option<String>,
    username: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
}
impl ClientState {
    fn new() -> Self {
        Self {
            logged_in: false,
            user_id: None,
            username: None,
            access_token: None,
            refresh_token: None,
        }
    }
}

#[tokio::main]
async fn main() {
    let server = utils::prompt_input("Server URL (default http://127.0.0.1:3000): ");
    let server = if server.is_empty() {
        "http://127.0.0.1:3000".to_string()
    } else {
        server
    };

    let state = ClientState::new();

    loop {
        if !state.logged_in {
            println!("\nChoose action:\n1) Register\n2) Login\n3) Logout\n4) Quit\nEnter number:");
            let mut sel = String::new();
            std::io::stdin().read_line(&mut sel).ok();
            match sel.trim() {
                "1" => {
                    register_handler(&server).await;
                }
                "2" => {
                    let login_response = login_handler(&server).await;
                    if let Ok(resp) = login_response {
                        println!("Logged in successfully!");
                        println!("User ID: {}", resp.user_id);
                        println!("Username: {}", resp.username);
                        println!("Access Token: {}", resp.access_token);
                        println!("Refresh Token: {}", resp.refresh_token);
                    } else {
                        eprintln!("Login failed.");
                    }
                }
                "3" => {
                    println!("Logged out (local only)");
                }
                "4" => break,
                _ => println!("Unknown selection"),
            }
        } else {
            println!("\nChoose resource:\n1) Account\n2) Organization\n3) Quit\nEnter number:");
            let mut sel = String::new();
            std::io::stdin().read_line(&mut sel).ok();
            match sel.trim() {
                "1" => {
                    println!("Account resource selected");
                }
                "2" => {
                    println!("Organization resource selected");
                }
                "3" => break,
                _ => println!("Unknown selection"),
            }
        }
    }
}
