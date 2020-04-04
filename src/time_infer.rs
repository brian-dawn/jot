use chrono::prelude::*;
/*

infer time

tomorrow at 3 => date
tuesday => date of the very next tuesday at 8am
4pm tuesday => yep.
4pm on tuesday => yep.
monday night => datetime monday at 6:00pm
monday evening ^
next week at 3

monday morning
monday afternoon

in 10 seconds => datetime 10 seconds from now
in 1 week
in 4 days
a week
week
5 seconds from now

we also gotta give feedback to the user.

do we break out a parser here?


use chrono::prelude::*;

/// Takes in a time, `3`, `3am`, `3:45pm` etc. and returns the normalized time.
/// also takes in `evening`, `morning`, `afternoon` and returns clock times.
fn parse_clocktime(input: &str) {

}

/// Works on tomorrow, week, tuesday, etc.
fn parse_day(input: &str) {
    // if we said tuesday and it is tuesday then we will go forward to a week.
}

/// tomorrow [clock-time] or [clock-time] tuesday
fn parse_tomorrow_with_time(parts: &Vec<String>) -> Option<DateTime<Local>> {

    match parts.len() {
        1 => {
            // If we just have a number then the day is today.

        }
        2 => {
            // Day could be in either the first or second position.

        }
    }

    None
}

*/

fn time_from_now(parts: &Vec<&str>) -> Option<DateTime<Local>> {
    let now: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();
    time_from_date(parts, now)
}

/// 1 day, 3 seconds, 2 seconds from now
fn time_from_date(parts: &Vec<&str>, datetime: DateTime<Local>) -> Option<DateTime<Local>> {
    let numeric = parts.get(0)?.parse::<f32>().ok()?;
    let time_unit = parts.get(1)?;

    let seconds_per_unit = match &time_unit[..] {
        "minute" => 60,
        "minutes" => 60,
        "hour" => 60 * 60,
        "hours" => 60 * 60,
        "day" => 24 * 60 * 60,
        "days" => 24 * 60 * 60,
        _ => return None,
    } as f32;

    let seconds_from_now = chrono::Duration::seconds((numeric * seconds_per_unit) as i64);

    Some(datetime + seconds_from_now)
}

/// Given a time of day, return a duration from midnight for that day.
fn parse_time(time: &str) -> Option<chrono::Duration> {
    // noon
    // morning
    // 10am
    // 11:34
    //

    match time {
        "noon" => return Some(chrono::Duration::hours(12)),
        "morning" => return Some(chrono::Duration::hours(8)),
        "evening" => return Some(chrono::Duration::hours(6)),
        "afternoon" => return Some(chrono::Duration::hours(2)),
        _ => {}
    }
    if let Ok(number) = time.parse::<i64>() {
        if number > 7 && number < 12 {
            return Some(chrono::Duration::hours(number));
        } else {
            return Some(chrono::Duration::hours(number + 12));
        }
    } else {
        // Maybe there was an am/pm or minute component?
        use regex::Regex;
        let full = Regex::new(r"^(\d+):(\d+)(.*)$").unwrap();
        let just_minute = Regex::new(r"^(\d+):(\d+)$").unwrap();
        let partial = Regex::new(r"^(\d+)(.*?)$").unwrap();

        if let Some(caps) = full.captures(time) {
            // we have a minute component, and maybe an am/pm one.
            let hour = caps.get(1)?.as_str().parse::<i64>().ok()?;
            let minute = caps.get(2)?.as_str().parse::<i64>().ok()?;
            let am_or_pm = caps.get(3)?.as_str();

            match am_or_pm {
                "am" => {
                    return Some(chrono::Duration::hours(hour) + chrono::Duration::minutes(minute))
                }

                "pm" => {
                    return Some(
                        chrono::Duration::hours(hour + 12) + chrono::Duration::minutes(minute),
                    )
                }
                _ => {
                    // do nothing.
                }
            }
        }

        if let Some(caps) = just_minute.captures(time) {
            let hour = caps.get(1)?.as_str().parse::<i64>().ok()?;
            let minute = caps.get(2)?.as_str().parse::<i64>().ok()?;

            if hour > 7 && hour < 12 {
                return Some(chrono::Duration::hours(hour) + chrono::Duration::minutes(minute));
            } else {
                return Some(
                    chrono::Duration::hours(hour + 12) + chrono::Duration::minutes(minute),
                );
            }
        }
        if let Some(caps) = partial.captures(time) {
            let hour = caps.get(1)?.as_str().parse::<i64>().ok()?;
            let am_or_pm = caps.get(2)?.as_str();

            match am_or_pm {
                "am" => return Some(chrono::Duration::hours(hour)),

                "pm" => return Some(chrono::Duration::hours(hour + 12)),
                _ => return None,
            }
        }
    }

    None
}

// on tuesday at noon
fn parse_on_dates(parts: &Vec<&str>, now: DateTime<Local>) -> Option<DateTime<Local>> {
    let our_midnight = now.date().and_hms(0, 0, 0);

    let current_weekday = now.weekday();
    let weekday = match parts.get(0)?.parse::<Weekday>().ok() {
        Some(weekday) => Some(weekday),
        None => match *parts.get(0)? {
            "tomorrow" => Some(current_weekday.succ()),
            _ => None,
        },
    }?;

    let days_from_now = if current_weekday.num_days_from_sunday() < weekday.num_days_from_sunday() {
        // It's the very next
        weekday.num_days_from_sunday() - current_weekday.num_days_from_sunday()
    } else {
        // We need to add 7 because it's next week.
        7 + weekday.num_days_from_sunday() - current_weekday.num_days_from_sunday()
    };

    let day_duration = chrono::Duration::hours(24 * days_from_now as i64);

    if parts.len() == 2 {
        // tuesday morning
        let hours_to_add = parse_time(parts[1])?;
        Some(our_midnight + day_duration + hours_to_add)
    } else if parts.len() == 1 {
        // monday
        Some(our_midnight + day_duration)
    } else {
        None
    }
}

fn just_time(parts: &Vec<&str>, now: DateTime<Local>) -> Option<DateTime<Local>> {
    let our_midnight = now.date().and_hms(0, 0, 0);

    let time_from_midnight = parse_time(&parts.get(0)?)?;

    if our_midnight + time_from_midnight < now {
        return Some(our_midnight + time_from_midnight + chrono::Duration::days(1));
    } else {
        return Some(our_midnight + time_from_midnight);
    }
}

pub fn infer_future_time(input: &str) -> Option<DateTime<Local>> {
    let now: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();

    let mut cleaned = String::new();
    cleaned.push_str(" ");
    cleaned.push_str(input);
    cleaned.push_str(" ");

    // We don't need these and they only get in the way of parsing.
    // uhg maybe I should just use nom.
    cleaned = cleaned.replace(" at ", " ");
    cleaned = cleaned.replace(" on ", " ");
    cleaned = cleaned.replace(" from ", " ");
    cleaned = cleaned.replace(" now ", " ");
    cleaned = cleaned.replace(" in ", " ");
    cleaned = cleaned.replace(" a ", " ");

    let parts = cleaned.split_whitespace().collect::<Vec<_>>();

    let time_from_now = time_from_now(&parts);
    if time_from_now.is_some() {
        return time_from_now;
    }

    let parse_on_date = parse_on_dates(&parts, now);
    if parse_on_date.is_some() {
        return parse_on_date;
    }

    // TODO this can create reminders in the past. Should shove to next day.
    return just_time(&parts, now);
}
