/// Commands for creating new notes/todos/reminders.
use crate::config::Config;
use crate::jot::{Jot, MessageType};
use anyhow::Result;
use std::collections::HashSet;
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
fn write_jot_to_file(jot: &Jot) -> Result<()> {
    let mut file = std::fs::File::create(&jot.path)?;
    file.write_all(jot.to_string().as_bytes())?;

    Ok(())
}

pub fn create_note_command(config: Config, previous_uuids: &HashSet<String>) -> Result<()> {
    let message = get_user_input()?;

    let path = compute_path(config)?;
    let jot = Jot::new(&path, message.trim(), MessageType::Note, previous_uuids);

    write_jot_to_file(&jot)?;
    jot.pprint();
    Ok(())
}

pub fn create_todo_command(config: Config, previous_uuids: &HashSet<String>) -> Result<()> {
    let message = get_user_input()?;

    let path = compute_path(config)?;
    let jot = Jot::new(
        &path,
        message.trim(),
        MessageType::Todo(None),
        previous_uuids,
    );

    write_jot_to_file(&jot)?;
    jot.pprint();
    Ok(())
}

fn compute_path(config: Config) -> Result<std::path::PathBuf> {
    let now = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH)?;
    let fname = format!("{:0>14}.jot", now.as_secs());

    let mut jot_path = config.journal_path.clone();
    jot_path.push(fname);
    Ok(jot_path)
}
