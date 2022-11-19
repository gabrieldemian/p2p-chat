pub mod utils {
    pub fn draw_cowsay(msg: String) {
        let lines: String = msg.chars().take(60).map(|_| "-").collect();

        println!("\n  {lines}");
        println!("< {msg} >");
        println!("  {lines}");
        println!("    \\   ^__^");
        println!("     \\  (oo)\\______");
        println!("        (__)\\      )\\/\\");
        println!("           ||----w |");
        println!("           ||     ||");
        println!("\n");
    }
}
