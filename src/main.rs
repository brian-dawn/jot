use notify_rust::Notification;

use anyhow::{Context, Result};
use chrono::prelude::*;
use clap::{App, Arg, SubCommand};

use colorful::Colorful;
use itertools::Itertools;
use regex::Regex;
use std::collections::HashSet;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::io::{self, BufRead};


mod config;
mod time_infer;

#[derive(Debug, Eq, PartialEq, Clone)]
enum MessageType {
    Regular, // [date]

    // Due date.
    Reminder(DateTime<Local>), // [date reminder due-date]

    // Start date, period, period time unit.
    // e.g. "starting X date, every 2 weeks BLAH"
    //ReoccuringReminder(DateTime<FixedOffset>, u32, PeriodTimeUnit),

    // e.g. "mark this date as Fred's birthday"
    //DateMarker(DateTime<FixedOffset>),

    // Completed
    //Todo(bool),
}

impl MessageType {
    fn from_string(i: &str) -> Option<MessageType> {
        let parts: Vec<&str> = i.split_whitespace().collect();
        match *parts.get(0)? {
            "remind" => {
                let date = parts.get(2)?;
                let parsed_date: DateTime<FixedOffset> =
                    DateTime::parse_from_rfc3339(&date).ok()?;
                Some(MessageType::Reminder(DateTime::from(parsed_date)))
            }
            _ => Some(MessageType::Regular),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct JotLine {
    datetime: DateTime<Local>,
    message: String,
    tags: Vec<String>,
    msg_type: MessageType,
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

impl JotLine {
    fn pprint(&self) {
        self.pprint_with_custom_msg(None);
    }

    /// Pretty print a Jot, we need to support custom messages for
    /// highlighting (such as via grep).
    fn pprint_with_custom_msg(&self, msg_override: Option<&str>) {
        let now: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();
        let time_difference = now - self.datetime;
        let (amount, amount_unit) = pretty_duration(time_difference);
        let header_string = match self.msg_type {
            MessageType::Reminder(reminder_date) => {
                if reminder_date < now {
                    // Reminder is in the past.

                    let remind_time = now - reminder_date;
                    let (fut_amount, fut_amount_unit) = pretty_duration(remind_time);
                    format!(
                        "reminded {} {}s ago",
                        fut_amount.to_string().bold().white(),
                        fut_amount_unit
                    )
                } else {
                    // Reminder is in the future.
                    let remind_time = reminder_date - now;
                    let (fut_amount, fut_amount_unit) = pretty_duration(remind_time);
                    format!(
                        "in {} {}s",
                        fut_amount.to_string().bold().green(),
                        fut_amount_unit
                    )
                }
            }
            MessageType::Regular => {
                format!("{} {}s ago", amount.to_string().bold().blue(), amount_unit)
            }
        };

        let _pretty_date = self.datetime.format("%Y-%m-%d %H:%M").to_string().blue();
        let msg = msg_override.unwrap_or(&self.message).trim();
        println!("―――――――――――――――――――――――――――――――――");
        println!("[{}]\n{}", header_string, msg);
        println!("―――――――――――――――――――――――――――――――――");
    }
}

/// Stream all the jots from disk.
fn stream_jots(config: config::Config) -> Result<impl Iterator<Item = JotLine>> {
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

                // TODO: move to regex match for date stamp
                if line?.trim().starts_with('[') {
                    if !buf.is_empty(){
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
        }))
}

/// Return a string for the date tag that is now.
fn now() -> String {
    let local: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();

    let date_str = local.to_rfc3339();
    format!("[{}]", date_str)
}

/// Return a string for the date tag that is now.
fn now_reminder(time: DateTime<Local>) -> String {
    let local: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();

    let date_str = local.to_rfc3339();
    let reminder_date_str = time.to_rfc3339();
    format!("[{} remind on {}]", date_str, reminder_date_str)
}

/// Parse a line in our jot log.
fn parse_note(header_line: &str, message: &str) -> Option<JotLine> {
    let re = Regex::new(r"\[(\d\d\d\d\-\d\d\-\d\dT\d\d:\d\d:\d\d-\d\d:\d\d)(.*?)\].*").unwrap();
    let caps = re.captures(header_line)?;
    let date = caps.get(1)?.as_str().trim().to_owned();
    let message_type = caps.get(2).map(|m| m.as_str()).unwrap_or("").trim();

    let tag_regex = Regex::new(r"@[a-zA-Z][0-9a-zA-Z_]*").unwrap();
    let tags = tag_regex
        .find_iter(message)
        .map(|tag| tag.as_str().to_owned())
        .collect();

    let parsed_date: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(&date).ok()?;
    Some(JotLine {
        datetime: DateTime::from(parsed_date),
        message: message.to_string(),
        tags,
        msg_type: MessageType::from_string(&message_type).unwrap_or(MessageType::Regular),
    })
}

// TODO: we can provide a list of tags inside the editor that are already in use.
// TODO: lets just load everything into memory and for todos we can update the reminder
//       header itself that way you can always re-axmine files.
fn main() -> Result<()> {
    let config = config::load_config()?;
    let matches = App::new("jot")
        .version("0.1")
        .about("jot down quick notes and reminders")
        .subcommand(SubCommand::with_name("cat").about("cat out the journal"))
        .subcommand(
            SubCommand::with_name("notify")
                .about("process any notifications, this is meant to be run from cron."),
        )
        .subcommand(
            SubCommand::with_name("grep")
                .about("search the journal")
                .arg(Arg::with_name("PATTERN").help("regex to grep for")),
        )
        .subcommand(SubCommand::with_name("tags").about("list all tags"))
        .subcommand(SubCommand::with_name("down").about("write to the journal"))
        .subcommand(
            SubCommand::with_name("reminder")
                .about("write to the journal")
                .arg(
                    Arg::with_name("TIME")
                        .multiple(true)
                        .help("set a time for the reminder"),
                ),
        )
        .get_matches();

    if let Some(_matches) = matches.subcommand_matches("down") {
        let message = scrawl::new()?;

        let mut file = OpenOptions::new().append(true).open(config.journal_path)?;
        writeln!(file, "{}", now())?;
        writeln!(file, "{}", message.trim())?;
        writeln!(file)?;
        writeln!(file)?;

        return Ok(());
    }

    if let Some(matches) = matches.subcommand_matches("reminder") {
        let time_str = matches.values_of("TIME").unwrap().join(" ");
        let reminder_time =
            time_infer::infer_future_time(&time_str).context("invalid time string")?;

        let message = scrawl::new()?;

        let mut file = OpenOptions::new().append(true).open(config.journal_path)?;
        writeln!(file, "{}", now_reminder(reminder_time))?;
        writeln!(file, "{}", message.trim())?;
        writeln!(file)?;
        writeln!(file)?;

        return Ok(());
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

    if let Some(_matches) = matches.subcommand_matches("cat") {
        for x in stream_jots(config)? {
            x.pprint();
            println!();
        }
        return Ok(());
    }

    if let Some(matches) = matches.subcommand_matches("grep") {
        let pattern = matches.value_of("PATTERN").unwrap();
        // TODO: if regex is invalid then escape everything and naive search.
        let re = Regex::new(pattern).unwrap();

        for jot in stream_jots(config)? {
            if re.find(&jot.message).is_none() {
                continue;
            }

            // Highlight the found strings.
            let mut msg = jot.message.clone();
            // We need to go in backwards order to preserve the indices.
            let found = re
                .find_iter(&jot.message)
                .collect::<Vec<_>>()
                .into_iter()
                .rev();
            for m in found {
                let highlighted = &msg[m.start()..m.end()].to_string().red();
                msg.replace_range(m.start()..m.end(), &highlighted.to_string());
            }

            jot.pprint_with_custom_msg(Some(&msg));
            println!();
        }
        return Ok(());
    }
    if let Some(_matches) = matches.subcommand_matches("tags") {
        let tags: HashSet<String> = stream_jots(config)?.flat_map(|jot| jot.tags).collect();
        for tag in tags {
            println!("{}", tag);
        }
        return Ok(());
    }

    Ok(())
}
