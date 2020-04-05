/// Commands that work with tags live here.
use crate::config::Config;
use crate::jot::{stream_jots, MessageType};
use anyhow::Result;
use colorful::Colorful;
use std::collections::{HashMap, HashSet};

pub fn tags_command(config: Config) -> Result<()> {
    let mut all_tags = HashSet::new();
    let mut notes = HashMap::new();
    let mut remns = HashMap::new();
    let mut todos = HashMap::new();
    let increment = |map: &mut HashMap<String, usize>, key: &str| {
        let insert = if let Some(val) = map.get(key) {
            val + 1
        } else {
            1
        };
        map.insert(key.to_string(), insert);
    };
    for jot in stream_jots(config)? {
        match jot.msg_type {
            MessageType::Note => {
                for tag in jot.tags {
                    increment(&mut notes, &tag);
                    all_tags.insert(tag);
                }
            }
            MessageType::Reminder(_) => {
                for tag in jot.tags {
                    increment(&mut remns, &tag);
                    all_tags.insert(tag);
                }
            }

            MessageType::Todo(_) => {
                for tag in jot.tags {
                    increment(&mut todos, &tag);
                    all_tags.insert(tag);
                }
            }
        }
    }

    use prettytable::{format, Cell, Table};
    let mut table = Table::new();
    let format = format::FormatBuilder::new()
        .separators(&[], format::LineSeparator::new('-', '+', '+', '+'))
        .padding(0, 0)
        .build();
    table.set_format(format);
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.add_row(row![
        "tag".bold(),
        "notes".bold().blue(),
        "todos".bold().magenta(),
        "reminders".bold().yellow()
    ]);
    for tag in itertools::sorted(all_tags.into_iter()) {
        let notes_cell = Cell::new_align(
            &notes
                .get(&tag)
                .map(|s| s.to_string().blue())
                .unwrap_or("".to_string().bold())
                .to_string(),
            format::Alignment::CENTER,
        );

        let todos_cell = Cell::new_align(
            &todos
                .get(&tag)
                .map(|s| s.to_string().magenta())
                .unwrap_or("".to_string().bold())
                .to_string(),
            format::Alignment::CENTER,
        );

        let remns_cell = Cell::new_align(
            &remns
                .get(&tag)
                .map(|s| s.to_string().yellow())
                .unwrap_or("".to_string().bold())
                .to_string(),
            format::Alignment::CENTER,
        );

        table.add_row(row![tag, notes_cell, todos_cell, remns_cell]);
    }
    table.printstd();
    Ok(())
}
