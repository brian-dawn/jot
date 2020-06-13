/// Commands that modify the journal (other than appending) live here.
use crate::config::Config;
use crate::jot::{stream_jots, Jot, MessageType};
use anyhow::Result;
use chrono::prelude::*;
use std::io::Write;

fn update_jot(jot: &Jot) -> Result<()> {
    // We are in directory mode so just overwrite that specific file.

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&jot.path)?;

    file.write_all(jot.to_string().as_bytes())?;
    return Ok(());
}

pub fn mark_todo_complete_command(config: Config, note_id_to_mark_complete: &str) -> Result<()> {
    // If the user passed in a number we're checking the count (id) not the uuid.
    let maybe_check_id = note_id_to_mark_complete.parse::<usize>().ok();
    let uuid = Some(note_id_to_mark_complete.to_string());

    // TODO: if we didn't find the id/uuid let the user know.

    // Read in the entire file Jot file and stream them to a temp file.

    let found_jot =
        stream_jots(config.clone())?.find(|jot| jot.uuid == uuid || Some(jot.id) == maybe_check_id);

    if let Some(mut jot) = found_jot {
        match jot.msg_type {
            MessageType::Todo(_) => {
                let now: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();
                jot.msg_type = MessageType::Todo(Some(now));
                jot.pprint();
                return update_jot(&jot);
            }

            _ => {
                println!("you can only complete a todo");
                std::process::exit(1)
            }
        }
    }

    // TODO: error couldn't find it.
    Ok(())
}

pub fn delete_jot(config: Config, note_id_to_delete: &str) -> Result<()> {
    // If the user passed in a number we're checking the count (id) not the uuid.
    let maybe_check_id = note_id_to_delete.parse::<usize>().ok();
    let uuid = Some(note_id_to_delete.to_string());

    // TODO: if we didn't find the id/uuid let the user know.

    for jot in stream_jots(config.clone())? {
        let this_one_should_be_deleted = jot.uuid == uuid || Some(jot.id) == maybe_check_id;
        if this_one_should_be_deleted {
            // Just delete the file and return.
            std::fs::remove_file(&jot.path)?;
            jot.pprint();

            return Ok(());
        }
    }
    Ok(())
}

pub fn edit_jot_contents(config: Config, note_id_to_edit: &str) -> Result<()> {
    // If the user passed in a number we're checking the count (id) not the uuid.
    let maybe_check_id = note_id_to_edit.parse::<usize>().ok();
    let uuid = Some(note_id_to_edit.to_string());

    let found_jot =
        stream_jots(config.clone())?.find(|jot| jot.uuid == uuid || Some(jot.id) == maybe_check_id);

    if let Some(mut jot) = found_jot {
        let message = scrawl::with(&jot.message.trim()).unwrap();

        if message.trim().is_empty() {
            return Ok(());
        } else {
            jot.message = message;
            jot.pprint();

            return update_jot(&jot);
        }
    }

    // TODO jot not found error
    Ok(())
}
