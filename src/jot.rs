use anyhow::Result;
use chrono::prelude::*;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;

use colorful::Colorful;
use itertools::Itertools;
use regex::Regex;
use std::collections::HashSet;

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
    // The path to the jot on disk.
    pub path: PathBuf,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum MessageType {
    Note,

    // Completed date, if not present we haven't completed yet.
    Todo(Option<DateTime<Local>>),
}

impl Jot {
    pub fn new(
        path: &Path,
        message: &str,
        message_type: MessageType,
        previous_uuids: &HashSet<String>,
    ) -> Jot {
        let local: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();

        Jot {
            datetime: local,
            message: message.trim().to_string(),
            msg_type: message_type,
            id: 0,
            uuid: Some(utils::generate_new_uuid(previous_uuids)), // todo replace with randomize fn, we need to know all previous
            tags: HashSet::new(),
            path: path.to_owned(),
        }
    }

    pub fn pprint(&self) {
        let msg = crate::utils::break_apart_long_string(&self.message.clone());
        self.pprint_with_custom_msg(Some(&msg));
    }

    /// Pretty print a Jot, we need to support custom messages for
    /// highlighting (such as via grep).
    pub fn pprint_with_custom_msg(&self, msg_override: Option<&str>) {
        let now: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();
        let time_difference = now - self.datetime;
        let (amount, amount_unit) = pretty_duration(time_difference);
        let plural_amount_unit = pluralize_time_unit(amount, amount_unit);
        let header_string = match self.msg_type {
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
                    TODO.green().bold(),
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
                .map(|line| count_real_chars(line).unwrap_or(0))
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

        // Make any tags be bold.
        // TODO: We should probably move greps into here as well, right now it is in the view
        // command and that's not where it should live IMO.
        let mut tag_msg = msg.to_string();
        let found = TAG_RE.find_iter(&msg).collect::<Vec<_>>().into_iter().rev();
        for m in found {
            let highlighted = &tag_msg[m.start()..m.end()].to_string().bold();
            tag_msg.replace_range(m.start()..m.end(), &highlighted.to_string());
        }

        println!("{}{}{}{}", "┌─", header, s_header, "─┐");
        println!("{}", tag_msg);
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
        write!(f, "{}\n\n", &st.trim())
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

pub fn stream_jots(config: config::Config) -> Result<impl Iterator<Item = Jot>> {
    assert!(config.journal_path.is_dir());

    let mut dirs = std::fs::read_dir(config.journal_path)?
        .map(|entry| Ok(entry?.path()))
        .collect::<Result<Vec<_>>>()?;

    dirs.sort();

    // TODO: We can parallelize this.
    let jot_stream = dirs
        .into_iter()
        .filter(|entry| entry.is_file())
        .filter_map(|file_path| {
            // Load the file
            let mut file = File::open(&file_path).ok()?;
            let mut contents = String::new();
            file.read_to_string(&mut contents).ok()?;
            let lines = contents.lines().collect::<Vec<_>>();
            let header_line = lines.first()?;
            let message = lines.iter().skip(1).join("\n");
            Some(parse_jot(header_line, &message, &file_path)?)
        })
        // Give each jot a real ID based on its position in the journal.
        .zip(1..)
        .map(|(mut jot, index)| {
            jot.id = index;
            jot
        });

    Ok(jot_stream)
}

lazy_static! {
    static ref TAG_RE: Regex = Regex::new(r"@[a-zA-Z][0-9a-zA-Z_]*").unwrap();
}

/// Parse a line in our jot log.
fn parse_jot(header_line: &str, message: &str, path: &Path) -> Option<Jot> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"\[(\d\d\d\d\-\d\d\-\d\dT\d\d:\d\d:\d\d-\d\d:\d\d)(.*?)\].*").unwrap();
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
        path: path.to_owned(),
    })
}
