#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate lazy_static;

use tempfile::NamedTempFile;

use notify_rust::Notification;
use unicode_segmentation::UnicodeSegmentation;

use anyhow::{Context, Result};
use chrono::prelude::*;
use clap::{App, Arg, SubCommand};

use colorful::Colorful;
use itertools::Itertools;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::io::{self, BufRead};

mod config;
mod time_infer;

#[derive(Debug, Eq, PartialEq, Clone)]
enum MessageType {
    Note,

    // Due date.
    Reminder(DateTime<Local>), // [date reminder due-date]

    // Start date, period, period time unit.
    // e.g. "starting X date, every 2 weeks BLAH" last element is the cancelled time.
    //ReoccuringReminder(DateTime<FixedOffset>, usize, PeriodTimeUnit, Option<DateTime<Local>>),

    // e.g. "mark this date as Fred's birthday"
    //DateMarker(DateTime<FixedOffset>),

    // Completed date, if not present we haven't completed yet.
    Todo(Option<DateTime<Local>>),
}

const TODO_NOT_DONE_PLACEHOLDER: &'static str = "not-done";
const REMIND_HEADER: &'static str = "remind";
const TODO_HEADER: &'static str = TODO;

const REMINDER: &'static str = "reminder";
const REMINDERS: &'static str = "reminders";

const TODO: &'static str = "todo";
const TODOS: &'static str = "todos";

const NOTE: &'static str = "note";
const NOTES: &'static str = "notes";

impl MessageType {
    fn from_string(i: &str) -> Option<MessageType> {
        let parts: Vec<&str> = i.split_whitespace().collect();
        match *parts.get(0)? {
            REMIND_HEADER => {
                let date = parts.get(2)?;
                let parsed_date: DateTime<FixedOffset> =
                    DateTime::parse_from_rfc3339(&date).ok()?;
                Some(MessageType::Reminder(DateTime::from(parsed_date)))
            }
            TODO_HEADER => {
                let date = parts.get(1)?.trim();
                if date == TODO_NOT_DONE_PLACEHOLDER {
                    Some(MessageType::Todo(None))
                } else {
                    // Attempt to parse the completed date.

                    let parsed_date: DateTime<FixedOffset> =
                        DateTime::parse_from_rfc3339(&date).ok()?;
                    Some(MessageType::Todo(Some(DateTime::from(parsed_date))))
                }
            }
            _ => Some(MessageType::Note),
        }
    }
}

fn pretty_duration<'a>(time_difference: chrono::Duration) -> (i64, &'a str) {
    // Pretty print how long ago a note was taken.
    let weeks_ago = time_difference.num_weeks();
    let days_ago = time_difference.num_days();
    let hours_ago = time_difference.num_hours();
    let minutes_ago = time_difference.num_minutes();
    let seconds_ago = time_difference.num_seconds();
    let (amount, amount_unit) = if weeks_ago > 0 {
        (weeks_ago, "week")
    } else if days_ago > 0 {
        (days_ago, "day")
    } else if hours_ago > 0 {
        (hours_ago, "hour")
    } else if minutes_ago > 0 {
        (minutes_ago, "minute")
    } else {
        (seconds_ago, "second")
    };

    (amount, amount_unit)
}

#[test]
fn test_pretty_duration() {
    assert_eq!(pretty_duration(chrono::Duration::seconds(1)), (1, "second"));
    assert_eq!(
        pretty_duration(chrono::Duration::seconds(124)),
        (2, "minute")
    );
    assert_eq!(pretty_duration(chrono::Duration::minutes(64)), (1, "hour"));
    assert_eq!(pretty_duration(chrono::Duration::hours(54)), (2, "day"));
    assert_eq!(pretty_duration(chrono::Duration::days(10)), (1, "week"));
    assert_eq!(pretty_duration(chrono::Duration::days(365)), (52, "week"));
}

fn pluralize_time_unit(amount: i64, time_unit: &str) -> String {
    if amount == 1 {
        return time_unit.to_string();
    }
    return format!("{}s", time_unit);
}

#[test]
fn test_pluralize_time_unit() {
    assert_eq!(pluralize_time_unit(1, "day"), "day");
    assert_eq!(pluralize_time_unit(2, "day"), "days");
    assert_eq!(pluralize_time_unit(-2, "minute"), "minutes");
}

fn print_bar(size: usize) {
    let s = std::iter::repeat("―").take(size).collect::<String>();
    println!("{}", s);
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct JotLine {
    datetime: DateTime<Local>,
    message: String,
    msg_type: MessageType,
    // TODO: These two fields aren't needed for creating new jots but are only when it is read.
    //       Maybe we should make a ReadJot super type?
    id: usize,
    tags: HashSet<String>,
}

impl JotLine {
    fn new(message: &str, message_type: MessageType) -> JotLine {
        let local: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();

        JotLine {
            datetime: local,
            message: message.trim().to_string(),
            msg_type: message_type,
            id: 0,
            tags: HashSet::new(),
        }
    }
    fn pprint(&self) {
        self.pprint_with_custom_msg(None);
    }

    /// Pretty print a Jot, we need to support custom messages for
    /// highlighting (such as via grep).
    fn pprint_with_custom_msg(&self, msg_override: Option<&str>) {
        let now: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();
        let time_difference = now - self.datetime;
        let (amount, amount_unit) = pretty_duration(time_difference);
        let plural_amount_unit = pluralize_time_unit(amount, amount_unit);
        let header_string = match self.msg_type {
            MessageType::Reminder(reminder_date) => {
                if reminder_date < now {
                    // Reminder is in the past.

                    let remind_time = now - reminder_date;
                    let (fut_amount, fut_amount_unit) = pretty_duration(remind_time);

                    format!(
                        "{} reminded {} {} ago",
                        REMINDER.white().bold(),
                        fut_amount.to_string().bold().white(),
                        pluralize_time_unit(fut_amount, fut_amount_unit)
                    )
                } else {
                    // Reminder is in the future.
                    let remind_time = reminder_date - now;
                    let (fut_amount, fut_amount_unit) = pretty_duration(remind_time);
                    format!(
                        "{} in {} {}",
                        REMINDER.red().bold(),
                        fut_amount.to_string().bold().green(),
                        pluralize_time_unit(fut_amount, fut_amount_unit)
                    )
                }
            }
            MessageType::Todo(None) => format!(
                "{} {} {} ago",
                TODO.red().bold(),
                amount.to_string().bold().blue(),
                plural_amount_unit
            ),
            MessageType::Todo(Some(completed_date)) => {
                let time_difference = now - completed_date;
                let (amount, amount_unit) = pretty_duration(time_difference);
                let plural_amount_unit = pluralize_time_unit(amount, amount_unit);

                format!(
                    "{} completed {} {} ago",
                    TODO.white().bold(),
                    amount.to_string().bold().blue(),
                    plural_amount_unit
                )
            }
            MessageType::Note => format!(
                "{} {} {} ago",
                NOTE.white().bold(),
                amount.to_string().bold().blue(),
                plural_amount_unit
            ),
        };

        let _pretty_date = self.datetime.format("%Y-%m-%d %H:%M").to_string().blue();
        let msg = msg_override.unwrap_or(&self.message).trim();

        let header = format!("{} #{}", header_string, self.id.to_string().bold());
        let bar_length = std::cmp::max(
            msg.lines()
                .map(|line| count_real_chars(line.trim()).unwrap_or(0))
                .max()
                .unwrap_or(0),
            count_real_chars(header.trim()).unwrap_or(0),
        );

        print_bar(bar_length);
        println!("{}", header);
        println!("{}", msg);
        print_bar(bar_length);
    }

    /// Write out the header string for this particular note.
    fn write_to_header_string(&self) -> String {
        let date_str = self.datetime.to_rfc3339();

        match self.msg_type {
            MessageType::Note => format!("[{}]", date_str),

            MessageType::Reminder(reminder_date) => {
                let reminder_date_str = reminder_date.to_rfc3339();
                format!("[{} {} on {}]", date_str, REMIND_HEADER, reminder_date_str)
            }

            MessageType::Todo(maybe_completed_date) => {
                let completed_str = maybe_completed_date
                    .map(|date| date.to_rfc3339())
                    .unwrap_or(TODO_NOT_DONE_PLACEHOLDER.to_string());
                format!("[{} {} {}]", date_str, TODO_HEADER, completed_str)
            }
        }
    }

    fn to_string(&self) -> String {
        format!("{}\n{}", self.write_to_header_string(), self.message)
    }
}

/// Remove ANSI escape codes and count real graphemes.
fn count_real_chars(input: &str) -> Option<usize> {
    Some(
        std::str::from_utf8(&strip_ansi_escapes::strip(input).ok()?)
            .ok()?
            .graphemes(true)
            .count(),
    )
}

/// Stream all the jots from disk.
fn stream_jots(config: config::Config) -> Result<impl Iterator<Item = JotLine>> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"\[(\d\d\d\d\-\d\d\-\d\dT\d\d:\d\d:\d\d-\d\d:\d\d).*").unwrap();
    }
    let file = File::open(config.journal_path)?;
    let _buffer = String::new();
    Ok(io::BufReader::new(file)
        .lines()
        .filter_map(Result::ok)
        .peekable()
        .batching(|it| {
            let mut buf = String::new();
            let mut header_line = String::new();

            // Warning: It's not clear here but the loop is returning a value.
            // normally I would have bound it into a variable but clippy didn't
            // like that. :(
            loop {
                let line = it.peek();

                // If we reached the EOF then process the last in the buffer.
                if line.is_none() {
                    let result = parse_note(&header_line, &buf);
                    break result;
                }

                if RE.is_match(line?.trim()) {
                    if !buf.is_empty() {
                        // We finished reading a jot.
                        let result = parse_note(&header_line, &buf);
                        if result.is_some() {
                            break result;
                        } else {
                            buf = String::new();
                            header_line = String::new();
                        }
                    } else {
                        header_line = line?.to_string();
                    }
                } else {
                    buf.push('\n');
                    buf.push_str(&line?);
                }
                it.next()?;
            }
        })
        // Give each jot a real ID based on its position in the journal.
        .zip(1..)
        .map(|(mut jot, index)| {
            jot.id = index;
            jot
        }))
}

/// Parse a line in our jot log.
fn parse_note(header_line: &str, message: &str) -> Option<JotLine> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"\[(\d\d\d\d\-\d\d\-\d\dT\d\d:\d\d:\d\d-\d\d:\d\d)(.*?)\].*").unwrap();
        static ref TAG_RE: Regex = Regex::new(r"@[a-zA-Z][0-9a-zA-Z_]*").unwrap();
    }

    let caps = RE.captures(header_line)?;
    let date = caps.get(1)?.as_str().trim().to_owned();
    let message_type = caps.get(2).map(|m| m.as_str()).unwrap_or("").trim();

    let tags = TAG_RE
        .find_iter(message)
        .map(|tag| tag.as_str().to_owned())
        .collect();

    let parsed_date: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(&date).ok()?;
    Some(JotLine {
        datetime: DateTime::from(parsed_date),
        message: message.trim().to_string(),
        tags,
        id: 0,
        msg_type: MessageType::from_string(&message_type).unwrap_or(MessageType::Note),
    })
}

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
                .about("view all todos")
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
                .about("view all notes")
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
                .about("view all reminders")
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

        let jot = JotLine::new(message.trim(), MessageType::Note);
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

        let jot = JotLine::new(message.trim(), MessageType::Todo(None));
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
        let jot = JotLine::new(message.trim(), MessageType::Reminder(reminder_time));
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
                for new_jot in stream_jots(config.clone())?.map(|jot| {
                    if jot.id == number_to_complete {
                        let message = scrawl::with(&jot.message.trim()).unwrap();

                        if message.trim().is_empty() {
                            jot
                        } else {
                            let mut new_jot = jot.clone();
                            new_jot.message = message;
                            new_jot
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
                for new_jot in stream_jots(config.clone())?.map(|jot| {
                    if jot.id == number_to_complete {
                        match jot.msg_type {
                            MessageType::Todo(_) => {
                                let mut new_jot = jot.clone();
                                let now: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();
                                new_jot.msg_type = MessageType::Todo(Some(now));
                                new_jot
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

            for jot in stream_jots(config.clone())? {
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

        use prettytable::{format, Cell, Row, Table};
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
