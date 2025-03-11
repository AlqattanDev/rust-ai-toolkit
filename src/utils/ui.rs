use colored::Colorize;
use std::io::{self, Write};

/// Print a success message
pub fn print_success(message: &str) {
    println!("{}", message.green());
}

/// Print an error message
pub fn print_error(message: &str) {
    eprintln!("{}", message.red());
}

/// Print a warning message
pub fn print_warning(message: &str) {
    println!("{}", message.yellow());
}

/// Print an info message
pub fn print_info(message: &str) {
    println!("{}", message.blue());
}

/// Print a stage header
pub fn print_stage_header(stage_number: u8, name: &str) {
    println!("\n{}", format!(">>> Stage {}: {} <<<", stage_number, name).green().bold());
}

/// Prompt the user for input with a message
pub fn prompt(message: &str) -> io::Result<String> {
    print!("{} ", message);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    Ok(input.trim().to_string())
}

/// Prompt the user for a yes/no answer
pub fn prompt_yes_no(message: &str, default: bool) -> io::Result<bool> {
    let prompt_suffix = if default { "[Y/n]" } else { "[y/N]" };
    let full_prompt = format!("{} {}", message, prompt_suffix);
    
    let input = prompt(&full_prompt)?;
    
    Ok(if input.is_empty() {
        default
    } else {
        input.to_lowercase().starts_with('y')
    })
}

/// Prompt the user to select from a list of options
pub fn prompt_select<T: AsRef<str>>(message: &str, options: &[T]) -> io::Result<usize> {
    println!("{}", message);
    
    for (i, option) in options.iter().enumerate() {
        println!("  {}. {}", i + 1, option.as_ref());
    }
    
    loop {
        let input = prompt("Enter your choice (number):")?;
        
        match input.parse::<usize>() {
            Ok(n) if n >= 1 && n <= options.len() => return Ok(n - 1),
            _ => {
                print_error(&format!("Please enter a number between 1 and {}", options.len()));
                continue;
            }
        }
    }
}

/// Display a spinner while executing a task
pub async fn with_spinner<F, T, E>(message: &str, task: F) -> Result<T, E>
where
    F: std::future::Future<Output = Result<T, E>>,
{
    use indicatif::{ProgressBar, ProgressStyle};
    
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner} {msg}")
            .unwrap()
    );
    spinner.set_message(message.to_string());
    
    let result = task.await;
    
    spinner.finish_and_clear();
    
    result
}

/// Display progress for a task with known steps
pub fn progress_bar(len: u64, message: &str) -> indicatif::ProgressBar {
    use indicatif::{ProgressBar, ProgressStyle};
    
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("=> ")
    );
    pb.set_message(message.to_string());
    
    pb
} 