pub fn prompt_input(prompt: &str) -> String {
    print!("{}", prompt);
    std::io::Write::flush(&mut std::io::stdout()).ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    input.trim().to_string()
}

