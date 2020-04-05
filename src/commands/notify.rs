/// Commands that work with reminder notifications live here.
use crate::config::{load_notified, mark_notified, Config};
use crate::jot::{stream_jots, MessageType};
use anyhow::Result;
use chrono::prelude::*;
use notify_rust::Notification;

pub fn notify(config: Config) -> Result<()> {
    let notified = load_notified()?;

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
            mark_notified(jot.datetime)?;
        }
    }
    return Ok(());
}

pub fn daemon_mode(config: Config) -> Result<()> {
    loop {
        let notified = load_notified()?;

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
                mark_notified(jot.datetime)?;
            }
        }

        let one_minute = std::time::Duration::from_millis(1000);
        std::thread::sleep(one_minute);
    }
}
