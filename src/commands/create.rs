/// Commands for creating new notes/todos/reminders.
use crate::config::Config;
use crate::jot::{Jot, MessageType};
use crate::time_infer;
use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::Write;

pub fn create_note_command(config: Config) -> Result<()> {
    let message = scrawl::new()?;
    if message.trim().is_empty() {
        return Ok(());
    }

    let mut file = OpenOptions::new().append(true).open(config.journal_path)?;

    let jot = Jot::new(message.trim(), MessageType::Note);
    writeln!(file, "{}", jot.to_string())?;
    writeln!(file)?;
    writeln!(file)?;

    Ok(())
}

pub fn create_todo_command(config: Config) -> Result<()> {
    let message = scrawl::new()?;
    if message.trim().is_empty() {
        return Ok(());
    }

    let mut file = OpenOptions::new().append(true).open(config.journal_path)?;

    let jot = Jot::new(message.trim(), MessageType::Todo(None));
    writeln!(file, "{}", jot.to_string())?;
    writeln!(file)?;
    writeln!(file)?;

    return Ok(());
}

pub fn create_reminder_command(config: Config, fuzzy_time_input: &str) -> Result<()> {
    let reminder_time =
        time_infer::infer_future_time(&fuzzy_time_input).context("invalid time string")?;

    let message = scrawl::new()?;
    if message.trim().is_empty() {
        return Ok(());
    }

    let mut file = OpenOptions::new().append(true).open(config.journal_path)?;
    let jot = Jot::new(message.trim(), MessageType::Reminder(reminder_time));
    writeln!(file, "{}", jot.to_string())?;
    writeln!(file)?;
    writeln!(file)?;

    return Ok(());
}
