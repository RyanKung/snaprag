/// Output formatting and streaming helpers
use crate::cli::output::*;

pub fn display_response(
    profile: &crate::models::UserProfile,
    response: &str,
    total_casts: usize,
    relevant_casts: usize,
) {
    let username = profile
        .username
        .as_ref()
        .map(|u| format!("@{}", u))
        .unwrap_or_else(|| format!("FID {}", profile.fid));
    let display_name = profile.display_name.as_deref().unwrap_or("Unknown");

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!(
        "║  {} ({})                                           ",
        display_name, username
    );
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    print_wrapped(response, 70);

    println!();
    println!("─────────────────────────────────────────────────────────────────");
    println!(
        "💬 Based on {} casts  |  🎯 Context: {} relevant casts",
        total_casts, relevant_casts
    );
    println!("─────────────────────────────────────────────────────────────────");
}

/// Word wrap text to specified width
pub fn print_wrapped(text: &str, max_width: usize) {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut current_line = String::new();

    for word in words {
        if current_line.len() + word.len() + 1 > max_width {
            println!("{}", current_line);
            current_line = word.to_string();
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
    }
    if !current_line.is_empty() {
        println!("{}", current_line);
    }
}
