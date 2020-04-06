#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate lazy_static;

use crate::constants::*;
use anyhow::Result;
use clap::{App, Arg, SubCommand};
use itertools::Itertools;

mod commands;
mod config;
mod constants;
mod jot;
mod time_infer;
mod utils;

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
        let previous_uuids = commands::view::get_all_uuids(config.clone())
            .unwrap_or(std::collections::HashSet::new());
        return commands::create::create_note_command(config, &previous_uuids);
    }

    if let Some(_matches) = matches.subcommand_matches(TODO) {
        let previous_uuids = commands::view::get_all_uuids(config.clone())
            .unwrap_or(std::collections::HashSet::new());
        return commands::create::create_todo_command(config, &previous_uuids);
    }

    if let Some(matches) = matches.subcommand_matches(REMINDER) {
        let time_str = matches.values_of("TIME").unwrap().join(" ");

        let previous_uuids = commands::view::get_all_uuids(config.clone())
            .unwrap_or(std::collections::HashSet::new());
        return commands::create::create_reminder_command(config, &time_str, &previous_uuids);
    }

    if let Some(matches) = matches.subcommand_matches("edit") {
        let id_or_uuid = matches.value_of("NUMBER").unwrap();
        return commands::edit::edit_jot_contents(config, id_or_uuid);
    }
    if let Some(matches) = matches.subcommand_matches("complete") {
        let id_or_uuid = matches.value_of("NUMBER").unwrap();
        return commands::edit::mark_todo_complete_command(config, id_or_uuid);
    }

    if let Some(_matches) = matches.subcommand_matches("daemon") {
        return commands::notify::daemon_mode(config);
    }

    if let Some(_matches) = matches.subcommand_matches("notify") {
        return commands::notify::notify(config);
    }

    // Commands for displaying various note types.
    let read_sub_cmd = vec![NOTES, REMINDERS, TODOS, "cat"]
        .into_iter()
        .find(|t| matches.subcommand_matches(t).is_some());
    if let Some(read_cmd) = read_sub_cmd {
        return commands::view::display(config, read_cmd, matches);
    }

    if let Some(_matches) = matches.subcommand_matches("tags") {
        return commands::tags::tags_command(config);
    }

    // matches.print_help();
    Ok(())
}
