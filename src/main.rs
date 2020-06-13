#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate lazy_static;

use crate::constants::*;
use anyhow::Result;
use clap::{App, Arg, SubCommand};

mod commands;
mod config;
mod constants;
mod jot;
mod utils;

fn main() -> Result<()> {
    let config = config::load_config()?;

    if config.journal_path.is_file() {
        println!("journal incompatable with this version of jot! We now work on a directory instead of a single file");
        return Ok(());
    }

    let matches = App::new("jot")
        .version("0.1")
        .about("Jot down quick notes")
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
                    Arg::with_name("REVERSE")
                        .short("r")
                        .long("reverse")
                        .help("Reverse the output"),
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
        // TODO: this command should be moved to the visualization sub commands like a -i flag
        // or something.
        .subcommand(
            SubCommand::with_name("search")
                .about("Perform interactive fuzzy searching on the journal."),
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
                    Arg::with_name("REVERSE")
                        .short("r")
                        .long("reverse")
                        .help("Reverse the output"),
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
                    Arg::with_name("REVERSE")
                        .short("r")
                        .long("reverse")
                        .help("Reverse the output"),
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
            SubCommand::with_name("edit")
                .about("Edit the contents of a note/todo")
                .arg(
                    Arg::with_name("ID")
                        .value_name("ID")
                        .takes_value(true)
                        .help("The id of the note you wish to edit"),
                ),
        )
        .subcommand(
            SubCommand::with_name("complete")
                .about("Complete a todo")
                .arg(
                    Arg::with_name("ID")
                        .value_name("ID")
                        .takes_value(true)
                        .help("The id of the todo you wish to complete"),
                ),
        )
        .subcommand(
            SubCommand::with_name("delete")
                .about("Complete a todo")
                .arg(
                    Arg::with_name("ID")
                        .value_name("ID")
                        .takes_value(true)
                        .help("The id of the todo you wish to delete"),
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

    if let Some(matches) = matches.subcommand_matches("edit") {
        let id_or_uuid = matches.value_of("ID").unwrap();
        return commands::edit::edit_jot_contents(config, id_or_uuid);
    }

    if let Some(matches) = matches.subcommand_matches("delete") {
        let id_or_uuid = matches.value_of("ID").unwrap();
        return commands::edit::delete_jot(config, id_or_uuid);
    }

    if let Some(matches) = matches.subcommand_matches("complete") {
        let id_or_uuid = matches.value_of("ID").unwrap();
        return commands::edit::mark_todo_complete_command(config, id_or_uuid);
    }

    if let Some(_matches) = matches.subcommand_matches("search") {
        return commands::view::interactive_search(config);
    }

    // Commands for displaying various note types.
    let read_sub_cmd = vec![NOTES, TODOS, "cat"]
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
