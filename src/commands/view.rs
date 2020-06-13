/// Commands related to viewing notes/todos/reminders live here.
use crate::config::Config;
use crate::constants::*;
use crate::jot::{stream_jots, Jot, MessageType};
use anyhow::Result;
use colorful::Colorful;
use regex::Regex;
use std::collections::HashSet;

fn search_string_to_regex(search: &str) -> String {
    // For now lets just ignore case and put in regex fillers.
    search
        .to_ascii_lowercase()
        .chars()
        .map(|c| format!("{}", c))
        .collect::<Vec<String>>()
        .join("[A-Za-z0-9]*?")
}

pub fn interactive_search(config: Config) -> Result<()> {
    use console::Term;
    let term = Term::stdout();
    let all_jots: Vec<Jot> = stream_jots(config)?.collect();
    let mut search_string = String::new();
    loop {
        if let Ok(key) = term.read_key() {
            match key {
                console::Key::Char(c) => {
                    search_string.push(c);
                }
                console::Key::Backspace => {
                    search_string.pop();
                }
                _ => {
                    // Do nothing
                }
            }
        }

        let re = Regex::new(&search_string_to_regex(&search_string))?;
        let mut matched_jots = all_jots
            .iter()
            .map(|jot| {
                let formatted_msg = crate::utils::break_apart_long_string(&jot.message.clone());
                let lower = formatted_msg.to_ascii_lowercase();
                let mut msg = formatted_msg.clone();
                let found = re.find_iter(&lower).collect::<Vec<_>>().into_iter().rev();

                let mut smallest_match = None;
                for m in found {
                    let size = m.end() - m.start();
                    match smallest_match {
                        None => {
                            smallest_match = Some(size);
                        }
                        Some(smallest) => {
                            if size < smallest {
                                smallest_match = Some(size)
                            } else {
                                continue;
                            }
                        }
                    }

                    msg = formatted_msg.clone();
                    let highlighted = &msg[m.start()..m.end()].to_string().red();
                    msg.replace_range(m.start()..m.end(), &highlighted.to_string());
                }

                (msg, jot, smallest_match)
            })
            .filter(|(_, _, matched_chars)| {
                match matched_chars {
                    Some(chars_matched) => {
                        // Reject long match strings.
                        *chars_matched < search_string.len() * 2
                    }
                    None => false,
                }
            })
            .collect::<Vec<_>>();

        matched_jots
            .sort_by(|(_, _, a), (_, _, b)| a.unwrap_or(0).partial_cmp(&b.unwrap_or(0)).unwrap());

        term.clear_screen()?;

        for (highlighted_msg, jot, _matched_chars) in matched_jots.iter().take(5) {
            jot.pprint_with_custom_msg(Some(&highlighted_msg));
        }

        println!("search: {}", search_string);
    }
}

pub fn display(config: Config, read_cmd: &str, matches: clap::ArgMatches) -> Result<()> {
    let reverse = matches
        .subcommand_matches(read_cmd)
        .unwrap()
        .is_present("REVERSE");

    let jots: Box<dyn Iterator<Item = Jot>> = if !reverse {
        Box::new(stream_jots(config)?)
    } else {
        let mut vecs = stream_jots(config)?.collect::<Vec<Jot>>();
        vecs.reverse();
        Box::new(vecs.into_iter())
    };
    for jot in jots {
        // See if we need to filter by the message type
        if read_cmd != "cat" {
            match jot.msg_type {
                MessageType::Note => {
                    if read_cmd != NOTES {
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

        let formatted_msg = crate::utils::break_apart_long_string(&jot.message.clone());
        let mut msg = formatted_msg.clone();

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

            if !(greps.is_empty() || greps.iter().all(|re| re.find(&formatted_msg).is_some())) {
                continue;
            } else {
                // We did find some grep'd things. Update and highlight our message.

                // We need to go in backwards order to preserve the indices.
                for re in greps {
                    let found = re
                        .find_iter(&formatted_msg)
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
    Ok(())
}

pub fn get_all_uuids(config: Config) -> Result<HashSet<String>> {
    Ok(stream_jots(config)?
        .map(|jot| jot.uuid)
        .filter_map(|uuid| uuid)
        .collect())
}
