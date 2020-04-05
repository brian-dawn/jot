/// Commands for creating new notes/todos/reminders.
use crate::config::Config;
use crate::jot::{Jot, MessageType};
use crate::time_infer;
use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::Write;

/// Get input from the users default $EDITOR.
/// If the input is empty or all whitespace we will
/// kill the process.
fn get_user_input() -> Result<String> {
    let message = scrawl::new()?;
    if message.trim().is_empty() {
        std::process::exit(0)
    }
    return Ok(message);
}

/// Append a jot to the journal specified in the config.
fn append_jot_to_journal(config: Config, jot: Jot) -> Result<()> {
    let mut file = OpenOptions::new().append(true).open(config.journal_path)?;

    writeln!(file, "{}", jot.to_string())?;
    writeln!(file)?;
    writeln!(file)?;

    Ok(())
}

pub fn create_note_command(config: Config) -> Result<()> {
    let message = get_user_input()?;

    let jot = Jot::new(message.trim(), MessageType::Note);
    append_jot_to_journal(config, jot)
}

pub fn create_todo_command(config: Config) -> Result<()> {
    let message = get_user_input()?;

    let jot = Jot::new(message.trim(), MessageType::Todo(None));

    append_jot_to_journal(config, jot)
}

pub fn create_reminder_command(config: Config, fuzzy_time_input: &str) -> Result<()> {
    let reminder_time =
        time_infer::infer_future_time(&fuzzy_time_input).context("invalid time string")?;

    let message = get_user_input()?;

    let jot = Jot::new(message.trim(), MessageType::Reminder(reminder_time));

    append_jot_to_journal(config, jot)
}