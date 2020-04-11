/// Commands that modify the journal (other than appending) live here.
use crate::config::Config;
use crate::jot::{stream_jots, Jot, MessageType};
use anyhow::Result;
use chrono::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

/// Safely write to the journal by writing to a temp file then copying that temp file over our journal.
fn write_to_journal(config: Config, new_jots: impl Iterator<Item = Jot>) -> Result<()> {
    let mut tmp_file = NamedTempFile::new()?;

    // Read in the entire file Jot file and stream them to a temp file.
    for new_jot in new_jots {
        // Write out the stream of jots to the new temp file
        writeln!(tmp_file, "{}", new_jot.to_string())?;
        writeln!(tmp_file)?;
        writeln!(tmp_file)?;
    }

    // Now we move the temp file over the journal.
    std::fs::copy(tmp_file.path(), config.journal_path)?;

    // Cleanup the temp file because some operating systems may not cleanup often enough.
    std::fs::remove_file(tmp_file.path())?;

    Ok(())
}

pub fn mark_todo_complete_command(config: Config, note_id_to_mark_complete: &str) -> Result<()> {
    // If the user passed in a number we're checking the count (id) not the uuid.
    let maybe_check_id = note_id_to_mark_complete.parse::<usize>().ok();
    let uuid = Some(note_id_to_mark_complete.to_string());

    // TODO: if we didn't find the id/uuid let the user know.

    // Read in the entire file Jot file and stream them to a temp file.
    let new_jots = stream_jots(config.clone())?.map(|mut jot| {
        if jot.uuid == uuid || Some(jot.id) == maybe_check_id {
            match jot.msg_type {
                MessageType::Todo(_) => {
                    let now: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();
                    jot.msg_type = MessageType::Todo(Some(now));
                    jot.pprint();
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
    });

    write_to_journal(config, new_jots)
}

pub fn delete_jot(config: Config, note_id_to_delete: &str) -> Result<()> {
    // If the user passed in a number we're checking the count (id) not the uuid.
    let maybe_check_id = note_id_to_delete.parse::<usize>().ok();
    let uuid = Some(note_id_to_delete.to_string());

    // TODO: if we didn't find the id/uuid let the user know.

    // Read in the entire file Jot file and stream them to a temp file.
    let new_jots = stream_jots(config.clone())?.filter(|jot| {
        let this_one_should_be_deleted = jot.uuid == uuid || Some(jot.id) == maybe_check_id;
        if this_one_should_be_deleted {
            // Print it out so the user knows it got deleted.
            jot.pprint();
        }

        !this_one_should_be_deleted
    });

    write_to_journal(config, new_jots)
}

pub fn edit_jot_contents(config: Config, note_id_to_edit: &str) -> Result<()> {
    // If the user passed in a number we're checking the count (id) not the uuid.
    let maybe_check_id = note_id_to_edit.parse::<usize>().ok();
    let uuid = Some(note_id_to_edit.to_string());

    // Read in the entire file Jot file and stream them to a temp file.
    let new_jots = stream_jots(config.clone())?.map(|mut jot| {
        if jot.uuid == uuid || Some(jot.id) == maybe_check_id {
            let message = scrawl::with(&jot.message.trim()).unwrap();

            if message.trim().is_empty() {
                jot
            } else {
                jot.message = message;
                jot.pprint();
                jot
            }
        } else {
            jot
        }
    });

    write_to_journal(config, new_jots)
}
