use anyhow::Result;

pub mod config;
pub mod doctor;
pub mod import;
pub mod list;
pub mod remove;
pub mod scan;
pub mod tui;

fn placeholder(command: &str) {
    println!("{command} command is not implemented yet");
}

pub fn run_placeholder(command: &str) -> Result<()> {
    placeholder(command);
    Ok(())
}
