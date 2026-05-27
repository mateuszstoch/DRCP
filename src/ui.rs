use crate::config::AppConfig;
use colored::*;
use crossterm::{
    cursor,
    execute,
    terminal,
};
use std::io::{stdout, Write};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Reconnecting(u64), // Seconds remaining for next attempt
}

pub fn render_dashboard(
    status: ConnectionStatus,
    config: &AppConfig,
    uptime: Duration,
    error_msg: Option<&str>,
    config_path: &Path,
) {
    let mut stdout = stdout();
    
    // Reset cursor to top-left and clear everything below it to avoid screen flickering
    let _ = execute!(
        stdout,
        cursor::MoveTo(0, 0),
        terminal::Clear(terminal::ClearType::FromCursorDown)
    );

    let status_str = match status {
        ConnectionStatus::Connected => "CONNECTED".green().bold(),
        ConnectionStatus::Disconnected => "DISCONNECTED".red().bold(),
        ConnectionStatus::Reconnecting(secs) => format!("RECONNECTING IN {}s...", secs).yellow().bold(),
    };

    let seconds = uptime.as_secs() % 60;
    let minutes = (uptime.as_secs() / 60) % 60;
    let hours = uptime.as_secs() / 3600;
    let uptime_str = format!("{:02}:{:02}:{:02}", hours, minutes, seconds).cyan();

    println!("{}", "========================================================".blue());
    println!("     {}      ", "DISCORD RICH PRESENCE (DRCP) - ACTIVE".bold().white());
    println!("{}", "========================================================".blue());
    
    println!("  Status:   {}", status_str);
    println!("  Uptime:   {}", uptime_str);
    println!("  Config:   {}", config_path.display().to_string().cyan());
    println!("{}", "--------------------------------------------------------".blue());
    println!("  Client ID: {}", config.client_id.magenta());
    
    println!("\n  {}", "Presence Details:".yellow().underline());
    if let Some(ref details) = config.presence.details {
        println!("    Details:      {}", details);
    } else {
        println!("    Details:      {}", "<none>".dimmed());
    }
    
    if let Some(ref state) = config.presence.state {
        println!("    State:        {}", state);
    } else {
        println!("    State:        {}", "<none>".dimmed());
    }

    if let Some(ref large_img) = config.presence.large_image {
        let large_txt = config.presence.large_text.as_deref().unwrap_or("");
        println!("    Large Image:  {} ({})", large_img.cyan(), large_txt.dimmed());
    }
    
    if let Some(ref small_img) = config.presence.small_image {
        let small_txt = config.presence.small_text.as_deref().unwrap_or("");
        println!("    Small Image:  {} ({})", small_img.cyan(), small_txt.dimmed());
    }

    if let Some(ref buttons) = config.presence.buttons {
        if !buttons.is_empty() {
            println!("\n  {}", "Buttons:".yellow().underline());
            for btn in buttons {
                println!("    - {} -> {}", btn.label.bold(), btn.url.dimmed());
            }
        }
    }

    if let Some(err) = error_msg {
        println!("\n  {}", "Last Error / Log:".red().underline());
        println!("    {}", err.red());
    }

    println!("\n{}", "========================================================".blue());
    println!("  {}", "Instructions:".white().bold());
    println!("  * Edit {} to dynamically update this status.", "config.toml".cyan());
    println!("  * Press {} to configure directly from console.", "E".yellow().bold());
    println!("  * Press {} to terminate the application.", "Ctrl+C".red().bold());
    println!("{}", "========================================================".blue());
    
    let _ = stdout.flush();
}
