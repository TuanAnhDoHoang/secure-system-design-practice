use std::io::{self, Write};
use rpassword::read_password;

fn prompt_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Lỗi khi đọc dữ liệu");

    input.trim().to_string()
}

fn authenticate(user: &str, pass: &str) -> bool {
    user == "admin" && pass == "anhdoo"
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Simple Login", long_about = None)]
struct Args {
    #[arg(short, long)]
    username: Option<String>,
}

pub async fn login() {
    let args = Args::parse();

    let username = match args.username {
        Some(user) => user,
        None => prompt_input("Username: "),
    };

    print!("Password: ");
    io::stdout().flush().unwrap();
    let password = read_password().expect("Không thể đọc mật khẩu");

    println!("\n--- Signing Handle ---");
    if authenticate(&username, &password) {
        println!("Signin successfully, {}!", username);
        let user = User {
            username,
            password,
            role: Role::Admin,
        };
        let permissions = Role::get_permissions(&user.role);
        println!("Permissions for {}:", user.username);
        for permission in permissions {
            println!(
                " - {}: {}",
                permission.resource as i32, permission.action as i32
            );
        }
    } else {
        println!("Sigin fail! wrong username or password.");
    }
}
