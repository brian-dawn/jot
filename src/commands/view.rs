/// Commands related to viewing notes/todos/reminders live here.
use crate::config::Config;
use crate::constants::*;
use crate::jot::{stream_jots, Jot, MessageType};
use anyhow::Result;
use colorful::Colorful;
use regex::Regex;
use std::collections::HashSet;

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

        let mut msg = crate::utils::break_apart_long_string(&jot.message.clone());
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
    Ok(())
}

pub fn get_all_uuids(config: Config) -> Result<HashSet<String>> {
    Ok(stream_jots(config)?
        .map(|jot| jot.uuid)
        .filter_map(|uuid| uuid)
        .collect())
}
