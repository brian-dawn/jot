use anyhow::Result;
use chrono::prelude::*;

use colorful::Colorful;
use itertools::Itertools;
use regex::Regex;
use std::collections::HashSet;
use std::fs::File;

use std::io::{self, BufRead};

use crate::config;
use crate::constants::*;
use crate::utils;
use crate::utils::{count_real_chars, pluralize_time_unit, pretty_duration};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Jot {
    pub datetime: DateTime<Local>,
    pub message: String,
    pub msg_type: MessageType,
    // TODO: These two fields aren't needed for creating new jots but are only when it is read.
    //       Maybe we should make a ReadJot super type?
    pub id: usize,
    pub uuid: Option<String>,
    pub tags: HashSet<String>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum MessageType {
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

impl Jot {
    pub fn new(message: &str, message_type: MessageType, previous_uuids: &HashSet<String>) -> Jot {
        let local: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();

        Jot {
            datetime: local,
            message: message.trim().to_string(),
            msg_type: message_type,
            id: 0,
            uuid: Some(utils::generate_new_uuid(previous_uuids)), // todo replace with randomize fn, we need to know all previous
            tags: HashSet::new(),
        }
    }

    /// Pretty print a Jot, we need to support custom messages for
    /// highlighting (such as via grep).
    pub fn pprint_with_custom_msg(&self, msg_override: Option<&str>) {
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
                        REMINDER.yellow().bold(),
                        fut_amount.to_string().bold().blue(),
                        pluralize_time_unit(fut_amount, fut_amount_unit)
                    )
                } else {
                    // Reminder is in the future.
                    let remind_time = reminder_date - now;
                    let (fut_amount, fut_amount_unit) = pretty_duration(remind_time);
                    format!(
                        "{} in {} {}",
                        REMINDER.yellow().bold(),
                        fut_amount.to_string().bold().blue(),
                        pluralize_time_unit(fut_amount, fut_amount_unit)
                    )
                }
            }
            MessageType::Todo(None) => format!(
                "{} {} {} ago",
                TODO.magenta().bold(),
                amount.to_string().bold().blue(),
                plural_amount_unit
            ),
            MessageType::Todo(Some(completed_date)) => {
                let time_difference = now - completed_date;
                let (amount, amount_unit) = pretty_duration(time_difference);
                let plural_amount_unit = pluralize_time_unit(amount, amount_unit);

                format!(
                    "{} completed {} {} ago",
                    TODO.magenta().bold(),
                    amount.to_string().bold().blue(),
                    plural_amount_unit
                )
            }
            MessageType::Note => format!(
                "{} {} {} ago",
                NOTE.blue().bold(),
                amount.to_string().bold().blue(),
                plural_amount_unit
            ),
        };

        let msg = msg_override.unwrap_or(&self.message).trim();

        let header = format!(
            "{} [{}]",
            header_string,
            self.uuid
                .clone()
                .unwrap_or(self.id.to_string())
                .cyan()
                .bold()
        );
        let bar_length = std::cmp::max(
            msg.lines()
                .map(|line| count_real_chars(line.trim()).unwrap_or(0))
                .max()
                .unwrap_or(0),
            count_real_chars(header.trim()).unwrap_or(0),
        );

        let header_chars = count_real_chars(&header).unwrap_or(0);
        let s_header = std::iter::repeat("─")
            .take(std::cmp::max(0, bar_length as i64 - header_chars as i64 - 2) as usize)
            .collect::<String>();

        let s = std::iter::repeat("─")
            .take(count_real_chars(&s_header).unwrap_or(0) + header_chars)
            .collect::<String>();

        println!("{}{}{}{}", "┌─", header, s_header, "─┐");
        println!("{}", msg);
        println!("{}{}{}", "└─", s, "─┘");
    }

    /// Write out the header string for this particular note.
    fn write_to_header_string(&self) -> String {
        let date_str = self.datetime.to_rfc3339();

        match self.msg_type {
            MessageType::Note => {
                if let Some(uuid) = &self.uuid {
                    format!("[{} id={}]", date_str, uuid)
                } else {
                    format!("[{}]", date_str)
                }
            }

            MessageType::Reminder(reminder_date) => {
                let reminder_date_str = reminder_date.to_rfc3339();
                if let Some(uuid) = &self.uuid {
                    format!(
                        "[{} {} on {} id={}]",
                        date_str, REMIND_HEADER, reminder_date_str, uuid
                    )
                } else {
                    format!("[{} {} on {}]", date_str, REMIND_HEADER, reminder_date_str,)
                }
            }

            MessageType::Todo(maybe_completed_date) => {
                let completed_str = maybe_completed_date
                    .map(|date| date.to_rfc3339())
                    .unwrap_or(TODO_NOT_DONE_PLACEHOLDER.to_string());
                if let Some(uuid) = &self.uuid {
                    format!(
                        "[{} {} {} id={}]",
                        date_str, TODO_HEADER, completed_str, uuid
                    )
                } else {
                    format!("[{} {} {}]", date_str, TODO_HEADER, completed_str)
                }
            }
        }
    }
}

impl std::fmt::Display for Jot {
    /// The standard to_string impl for Jot.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let st = format!("{}\n{}", self.write_to_header_string(), self.message);
        write!(f, "{}", &st)
    }
}

impl MessageType {
    /// Parse a message type from a string.
    fn from_string(i: &str) -> Option<(Option<String>, MessageType)> {
        let parts: Vec<&str> = i.split_whitespace().collect();

        let id_part = parts
            .iter()
            .find(|p| p.starts_with("id="))
            .map(|id_part| id_part.split("=").last().unwrap_or("").to_string());

        match *parts.get(0)? {
            REMIND_HEADER => {
                let date = parts.get(2)?;
                let parsed_date: DateTime<FixedOffset> =
                    DateTime::parse_from_rfc3339(&date).ok()?;
                Some((id_part, MessageType::Reminder(DateTime::from(parsed_date))))
            }
            TODO_HEADER => {
                let date = parts.get(1)?.trim();
                if date == TODO_NOT_DONE_PLACEHOLDER {
                    Some((id_part, MessageType::Todo(None)))
                } else {
                    // Attempt to parse the completed date.

                    let parsed_date: DateTime<FixedOffset> =
                        DateTime::parse_from_rfc3339(&date).ok()?;
                    Some((
                        id_part,
                        MessageType::Todo(Some(DateTime::from(parsed_date))),
                    ))
                }
            }
            _ => Some((id_part, MessageType::Note)),
        }
    }
}

/// Stream all the jots from disk.
pub fn stream_jots(config: config::Config) -> Result<impl Iterator<Item = Jot>> {
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
                    let result = parse_jot(&header_line, &buf);
                    break result;
                }

                if RE.is_match(line?.trim()) {
                    if !buf.is_empty() {
                        // We finished reading a jot.
                        let result = parse_jot(&header_line, &buf);
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
fn parse_jot(header_line: &str, message: &str) -> Option<Jot> {
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
    let (id, msg_type) =
        MessageType::from_string(&message_type).unwrap_or((None, MessageType::Note));
    Some(Jot {
        datetime: DateTime::from(parsed_date),
        message: message.trim().to_string(),
        tags,
        id: 0,
        uuid: id,
        msg_type: msg_type,
    })
}
