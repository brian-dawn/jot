#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate lazy_static;

use tempfile::NamedTempFile;

use notify_rust::Notification;

use anyhow::{Context, Result};
use chrono::prelude::*;
use clap::{App, Arg, SubCommand};

use colorful::Colorful;
use itertools::Itertools;
use regex::Regex;
use std::collections::{HashMap, HashSet};

use std::fs::OpenOptions;
use std::io::Write;

mod config;
mod constants;
mod model;
mod time_infer;
mod utils;

use constants::*;
use model::{stream_jots, Jot, MessageType};

fn main() -> Result<()> {
    let config = config::load_config()?;
    let matches = App::new("jot")
        .version("0.1")
        .about("Jot down quick notes and reminders")
        .subcommand(
            SubCommand::with_name("cat")
                .about("Dump out the entire journal")
                .arg(
                    Arg::with_name("TAG")
                        .short("t")
                        .long("tag")
                        .value_name("TAG")
                        .takes_value(true)
                        .multiple(true)
                        .help("Filter by a tag"),
                )
                .arg(
                    Arg::with_name("GREP")
                        .short("g")
                        .long("grep")
                        .value_name("GREP")
                        .takes_value(true)
                        .multiple(true)
                        .help("Filter by contents"),
                ),
        )
        .subcommand(
            SubCommand::with_name("notify")
                .about("Process any notifications, this is meant to be run from cron."),
        )
        .subcommand(
            SubCommand::with_name("daemon")
                .about("Periodically check to see if we need to dump out notifications."),
        )
        .subcommand(SubCommand::with_name("tags").about("List all tags"))
        .subcommand(SubCommand::with_name(TODO).about("Write a todo"))
        .subcommand(
            SubCommand::with_name(TODOS)
                .about("View all todos")
                .arg(
                    Arg::with_name("TAG")
                        .short("t")
                        .long("tag")
                        .value_name("TAG")
                        .takes_value(true)
                        .multiple(true)
                        .help("Filter by a tag"),
                )
                .arg(
                    Arg::with_name("GREP")
                        .short("g")
                        .long("grep")
                        .value_name("GREP")
                        .takes_value(true)
                        .multiple(true)
                        .help("Filter by contents"),
                ),
        )
        .subcommand(SubCommand::with_name(NOTE).about("Write a note"))
        .subcommand(
            SubCommand::with_name(NOTES)
                .about("View all notes")
                .arg(
                    Arg::with_name("TAG")
                        .short("t")
                        .long("tag")
                        .value_name("TAG")
                        .takes_value(true)
                        .multiple(true)
                        .help("Filter by a tag"),
                )
                .arg(
                    Arg::with_name("GREP")
                        .short("g")
                        .long("grep")
                        .value_name("GREP")
                        .takes_value(true)
                        .multiple(true)
                        .help("Filter by contents"),
                ),
        )
        .subcommand(
            SubCommand::with_name(REMINDER)
                .about("write a reminder")
                .arg(
                    Arg::with_name("TIME")
                        .multiple(true)
                        .help("Set a time for the reminder"),
                ),
        )
        .subcommand(
            SubCommand::with_name("edit")
                .about("Edit the contents of a note/todo/reminder")
                .arg(
                    Arg::with_name("NUMBER")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("The note number"),
                ),
        )
        .subcommand(
            SubCommand::with_name("complete")
                .about("Complete a todo")
                .arg(
                    Arg::with_name("NUMBER")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("The note number"),
                ),
        )
        .subcommand(
            SubCommand::with_name(REMINDERS)
                .about("View all reminders")
                .arg(
                    Arg::with_name("TAG")
                        .short("t")
                        .long("tag")
                        .value_name("TAG")
                        .takes_value(true)
                        .multiple(true)
                        .help("Filter by a tag"),
                )
                .arg(
                    Arg::with_name("GREP")
                        .short("g")
                        .long("grep")
                        .value_name("GREP")
                        .takes_value(true)
                        .multiple(true)
                        .help("Filter by contents"),
                ),
        )
        .get_matches();

    if let Some(_matches) = matches.subcommand_matches(NOTE) {
        let message = scrawl::new()?;
        if message.trim().is_empty() {
            return Ok(());
        }

        let mut file = OpenOptions::new().append(true).open(config.journal_path)?;

        let jot = Jot::new(message.trim(), MessageType::Note);
        writeln!(file, "{}", jot.to_string())?;
        writeln!(file)?;
        writeln!(file)?;

        return Ok(());
    }

    if let Some(_matches) = matches.subcommand_matches(TODO) {
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

    if let Some(matches) = matches.subcommand_matches(REMINDER) {
        let time_str = matches.values_of("TIME").unwrap().join(" ");
        let reminder_time =
            time_infer::infer_future_time(&time_str).context("invalid time string")?;

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

    if let Some(matches) = matches.subcommand_matches("edit") {
        match matches.value_of("NUMBER").unwrap().parse::<usize>() {
            Ok(number_to_complete) => {
                let mut tmp_file = NamedTempFile::new()?;

                // Read in the entire file Jot file and stream them to a temp file.
                for new_jot in model::stream_jots(config.clone())?.map(|mut jot| {
                    if jot.id == number_to_complete {
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
            }
            Err(_) => {
                println!("invalid note number");
                std::process::exit(1)
            }
        }
        return Ok(());
    }
    if let Some(matches) = matches.subcommand_matches("complete") {
        match matches.value_of("NUMBER").unwrap().parse::<usize>() {
            Ok(number_to_complete) => {
                let mut tmp_file = NamedTempFile::new()?;

                // Read in the entire file Jot file and stream them to a temp file.
                for new_jot in model::stream_jots(config.clone())?.map(|mut jot| {
                    if jot.id == number_to_complete {
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
            }
            Err(_) => {
                println!("invalid note number");
                std::process::exit(1)
            }
        }
        return Ok(());
    }

    if let Some(_matches) = matches.subcommand_matches("daemon") {
        loop {
            let notified = config::load_notified()?;

            let now: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();

            for jot in model::stream_jots(config.clone())? {
                if let MessageType::Reminder(remind_time) = jot.msg_type {
                    // If the notification is too far in the past.
                    if now - jot.datetime > chrono::Duration::days(1) {
                        continue;
                    }

                    // If we already notified about this one.
                    if notified.contains(&jot.datetime) {
                        continue;
                    }

                    // BUT if we are within X seconds of it lets just wait then notify.
                    if now - remind_time < chrono::Duration::seconds(60) {
                        match (now - remind_time).to_std() {
                            Ok(duration) => std::thread::sleep(duration),
                            Err(_) => {
                                continue;
                            }
                        }
                    } else {
                        // We have to be at least past the point of the reminder.
                        if remind_time - now > chrono::Duration::seconds(0) {
                            continue;
                        }
                    }

                    println!("we got a reminder!");

                    Notification::new()
                        .summary("jot")
                        .body(&jot.message)
                        .show()
                        .unwrap();

                    // Mark it as notified.
                    config::mark_notified(jot.datetime)?;
                }
            }

            let one_minute = std::time::Duration::from_millis(1000);
            std::thread::sleep(one_minute);
        }
    }

    if let Some(_matches) = matches.subcommand_matches("notify") {
        let notified = config::load_notified()?;

        let now: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();

        for jot in stream_jots(config)? {
            if let MessageType::Reminder(remind_time) = jot.msg_type {
                // If the notification is too far in the past.
                if now - jot.datetime > chrono::Duration::days(1) {
                    continue;
                }

                // If we already notified about this one.
                if notified.contains(&jot.datetime) {
                    continue;
                }

                // We have to be at least past the point of the reminder.
                if remind_time - now > chrono::Duration::seconds(0) {
                    continue;
                }

                println!("we got a reminder!");

                Notification::new()
                    .summary("jot")
                    .body(&jot.message)
                    .show()
                    .unwrap();

                // Mark it as notified.
                config::mark_notified(jot.datetime)?;
            }
        }
        return Ok(());
    }

    // Commands for displaying various note types.
    let read_sub_cmd = vec![NOTES, REMINDERS, TODOS, "cat"]
        .into_iter()
        .find(|t| matches.subcommand_matches(t).is_some());
    if let Some(read_cmd) = read_sub_cmd {
        for jot in stream_jots(config)? {
            // See if we need to filter by the message type
            if read_cmd != "cat" {
                match jot.msg_type {
                    MessageType::Note => {
                        if read_cmd != NOTES {
                            continue;
                        }
                    }

                    MessageType::Reminder(_) => {
                        if read_cmd != REMINDERS {
                            continue;
                        }
                    }

                    MessageType::Todo(_) => {
                        if read_cmd != TODOS {
                            continue;
                        }
                    }
                }
            }

            let mut msg = jot.message.clone();
            // bleh idk if I like this, we should be able to do a grep or do a tag. -t -g etc.
            if let Some(sub_matches) = matches.subcommand_matches(read_cmd) {
                // Skip checks.
                let tags = sub_matches
                    .values_of("TAG")
                    .map(|m| m.collect::<HashSet<&str>>())
                    .unwrap_or_default();

                if !(tags.is_empty() || tags.iter().all(|tag| jot.tags.contains(*tag))) {
                    continue;
                }

                let greps = sub_matches
                    .values_of("GREP")
                    .map(|m| {
                        m.map(|grep| {
                            let re_attempt = Regex::new(grep);
                            match re_attempt {
                                Ok(re) => re,
                                Err(err) => {
                                    println!("invalid regex {:?} error={:?}", grep, err);
                                    std::process::exit(1);
                                }
                            }
                        })
                        .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                if !(greps.is_empty() || greps.iter().all(|re| re.find(&jot.message).is_some())) {
                    continue;
                } else {
                    // We did find some grep'd things. Update and highlight our message.

                    // We need to go in backwards order to preserve the indices.
                    for re in greps {
                        let found = re
                            .find_iter(&jot.message)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev();
                        for m in found {
                            let highlighted = &msg[m.start()..m.end()].to_string().red();
                            msg.replace_range(m.start()..m.end(), &highlighted.to_string());
                        }
                    }
                }
            }

            jot.pprint_with_custom_msg(Some(&msg));
            println!();
        }
        return Ok(());
    }

    if let Some(_matches) = matches.subcommand_matches("tags") {
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

        return Ok(());
    }

    Ok(())
}
