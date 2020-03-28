use notify_rust::Notification;

use colorful::Color;
use colorful::Colorful;
use anyhow::Result;
use chrono::prelude::*;
use clap::{App, Arg, SubCommand};
use regex::Regex;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::io::{self, BufRead};
use std::path::Path;

// jot.txt contains a series of logs and stuff.
// jot also needs to be a daemon process so it can parse jot files
//
// we have a config toml file that lives somewhere.
// this stores where the jot.txt file lives.

// TODO jot init for creating a new jot db.
//
//
// jot supports hashtags for topics #foo #bar and can display/edit tags.
//
//
//
// TODO: I do think we want to support multiple lines somehow. Idk how :/

// have special tags #red, #blue, #green, #yellow, etc.
enum PeriodTimeUnit {

    Hours, Days, Weeks, Months, Years
}
enum MessageType {

    Regular,

    // Due date.
    Reminder(DateTime<FixedOffset>),

    // Start date, period, period time unit.
    // e.g. "starting X date, every 2 weeks BLAH"
    ReoccuringReminder(DateTime<FixedOffset>, u32, PeriodTimeUnit),

    // e.g. "mark this date as Fred's birthday"
    DateMarker(DateTime<FixedOffset>)

}

#[derive(PartialEq, Eq, Clone)]
struct JotLine {
    datetime: DateTime<FixedOffset>,
    raw: String,
    message: String,
    tags: Vec<String>,
}

impl JotLine {
    fn pprint(&self) {
        let pretty_date = self.datetime.format("%Y-%m-%d %H:%M").to_string().blue();
        println!("[{}] {}", pretty_date, self.message)
    }
}

/// Return a string for the date tag that is now.
fn now() -> String {
    let local: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();

    let date_str = local.to_rfc3339();
    format!("[{}]", date_str)
}

/// Parse a line in our jot log.
fn parse_line(line: &str) -> Option<JotLine> {
    let re = Regex::new(r"^\[(.*?)\] (.*)$").unwrap();
    for caps in re.captures_iter(line) {
        let date = caps.get(1)?.as_str().trim().to_owned();
        let message = caps.get(2)?.as_str();

        let tag_regex = Regex::new(r"\#[a-zA-Z][0-9a-zA-Z_]*").unwrap();
        let tags = tag_regex.find_iter(message).map(|tag| tag.as_str().to_owned()).collect();

        let parsed_date = DateTime::parse_from_rfc3339(&date).ok()?;
        return Some(JotLine {
            datetime: parsed_date,
            raw: line.to_owned(),
            message: message.to_string(),
            tags
        })
    }

    None
}

fn main() -> Result<()> {
    let journal = "/Users/brian/Sync/journal.txt";
    let matches = App::new("jot")
        .version("0.1")
        .about("jot down quick notes and reminders")
        .subcommand(SubCommand::with_name("cat").about("cat out the journal"))
        .subcommand(
            SubCommand::with_name("tag")
                .about("commands around tags")
                .arg(
                    Arg::with_name("debug")
                        .short("d")
                        .help("print debug information verbosely"),
                ),
        )
        .subcommand(
            SubCommand::with_name("down")
                .about("write to the journal")
                .arg(
                    Arg::with_name("MESSAGE")
                        .multiple(true)
                        .help("Sets the level of verbosity"),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("cat") {
        let file = File::open(journal)?;

        for line in io::BufReader::new(file).lines() {
            let ln = line?;
            if let Some(parsed) = parse_line(&ln) {

                parsed.pprint();
            } else {
                // print out just the raw string.
                //println!("{}", ln);
            }
        }
        return Ok(());
    }

    if let Some(matches) = matches.subcommand_matches("tag") {
        // TODO: Run tag subcommand.
        return Ok(());
    }

    if let Some(matches) = matches.subcommand_matches("down") {
        let local: DateTime<Local> = Local::now();
        let message = scrawl::new()?;

        let out = format!("{} {}", now(), message);

        let mut file = OpenOptions::new().append(true).open(journal)?;
        writeln!(file, "{}", out)?;

        // Notification::new()
        //     .summary("jot")
        //     .body("I was supposed to remind you about something")
        //     .show()
        //     .unwrap();

        return Ok(());
    }
    Ok(())
}
