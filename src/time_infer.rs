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

/// 1 day, 3 seconds, 2 seconds from now
fn time_from_now(parts: Vec<&str>) -> Option<DateTime<Local>> {
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

    let now: DateTime<Local> = Local::now().with_nanosecond(0).unwrap();

    Some(now + seconds_from_now)
}

pub fn infer_future_time(input: &str) -> Option<DateTime<Local>> {
    let mut cleaned = String::new();
    cleaned.push_str(" ");
    cleaned.push_str(input);
    cleaned.push_str(" ");

    // We don't need these and they only get in the way of parsing.
    // uhg maybe I should just use nom.
    cleaned = cleaned.replace(" at ", "");
    cleaned = cleaned.replace(" on ", "");
    cleaned = cleaned.replace(" from ", "");
    cleaned = cleaned.replace(" now ", "");
    cleaned = cleaned.replace(" in ", "");
    cleaned = cleaned.replace(" a ", "");

    let parts = cleaned.split_whitespace().collect::<Vec<_>>();

    time_from_now(parts)
    // if parts.get(0)?.parse::<Weekday>().is_ok() {
    // }

    //let day = Weekday::Mon;
    //"Sunday".parse::<Weekday>()
}
