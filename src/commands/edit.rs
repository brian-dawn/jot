/// Commands that modify the journal (other than appending) live here.
use crate::config::Config;
use crate::jot::{stream_jots, MessageType};
use anyhow::Result;
use chrono::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

pub fn mark_todo_complete_command(config: Config, note_id_to_mark_complete: usize) -> Result<()> {
    let mut tmp_file = NamedTempFile::new()?;

    // Read in the entire file Jot file and stream them to a temp file.
    for new_jot in stream_jots(config.clone())?.map(|mut jot| {
        if jot.id == note_id_to_mark_complete {
            match jot.msg_type {
                MessageType::Todo(_) => {
                    let now: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();
                    jot.msg_type = MessageType::Todo(Some(now));
                    jot
                }

                _ => {
                    println!("you can only complete a todo");
                    std::process::exit(1)
                }
            }
        } else {
            jot
        }
    }) {
        // Write out the stream of jots to the new temp file
        writeln!(tmp_file, "{}", new_jot.to_string())?;
        writeln!(tmp_file)?;
        writeln!(tmp_file)?;
    }

    // Now we move the temp file over the journal.
    std::fs::copy(tmp_file.path(), config.journal_path)?;
    Ok(())
}

pub fn edit_jot_contents(config: Config, note_id_to_edit: usize) -> Result<()> {
    let mut tmp_file = NamedTempFile::new()?;

    // Read in the entire file Jot file and stream them to a temp file.
    for new_jot in stream_jots(config.clone())?.map(|mut jot| {
        if jot.id == note_id_to_edit {
            let message = scrawl::with(&jot.message.trim()).unwrap();

            if message.trim().is_empty() {
                jot
            } else {
                jot.message = message;
                jot
            }
        } else {
            jot
        }
    }) {
        // Write out the stream of jots to the new temp file
        writeln!(tmp_file, "{}", new_jot.to_string())?;
        writeln!(tmp_file)?;
        writeln!(tmp_file)?;
    }

    // Now we move the temp file over the journal.
    std::fs::copy(tmp_file.path(), config.journal_path)?;
    Ok(())
}
