/// Given a duration, return a tuple of (scalar, time-unit).
/// This function attempts to round far away times to the nearest large
/// unit (naively implemented so it doesn't exactly behave that way).
pub fn pretty_duration<'a>(time_difference: chrono::Duration) -> (i64, &'a str) {
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

/// Pluralize words e.g. Hour => Hours, etc.
pub fn pluralize_time_unit(amount: i64, time_unit: &str) -> String {
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

/// Remove ANSI escape codes and get the real terminal width of the text.
pub fn count_real_chars(input: &str) -> Option<usize> {
    Some(console::measure_text_width(input))
}

#[test]
fn test_count_real_chars() {
    assert_eq!(count_real_chars("hello"), Some(5));
    assert_eq!(count_real_chars("hello"), Some(5));
    assert_eq!(count_real_chars("     "), Some(5));
    assert_eq!(count_real_chars("👩"), Some(2));
    assert_eq!(count_real_chars("何"), Some(2));
    assert_eq!(count_real_chars("🖋️"), Some(1)); // TODO: This is incorrect I think?
}

/// We use this function to attempt to format messages into smaller terminals.
/// We will also render newlines similarly to how markdown does it.
pub fn break_apart_long_string(st: &str) -> String {
    let term = console::Term::stdout();
    let (_height, width) = term.size();

    let ideal_split_point = width - 4;

    format!("{}", textwrap::fill(st, ideal_split_point as usize))
}

const BASE: u32 = 26;
const BASE_2: u32 = BASE * BASE;
const BASE_3: u32 = BASE * BASE * BASE;
const BASE_4: u32 = BASE * BASE * BASE * BASE;
const LETTERS: [char; BASE as usize] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

/// Generate a new uuid that is unique favoring short and easy to type ids.
pub fn generate_new_uuid(previous_uuids: &std::collections::HashSet<String>) -> String {
    // Algorithm is look at the size of existing uuids, lock them to powers of 26 since
    //           our uuids that are random are all lowercase letters. Pick a random number
    //           inside that range of available numbers (if there are slots available) and
    //           then increase that id by 1 every repeatedly till we find one that is available.

    let len = previous_uuids.len() + BASE as usize;
    //
    // Users can name things whatever they want
    // We should prefer short names.
    // if previous_uuids is sufficiently long we increase the bit space.
    // NOTE: we add 26 to len because that's where we start counting to ensure
    //       every uuid has at least 2 digits.
    let pool_to_draw_from = if len < BASE_2 as usize {
        // 26^2, 2 letters should be available.
        BASE_2
    } else if len < BASE_3 as usize {
        // 26^3, 3 letters should be available.
        BASE_3
    } else if len < BASE_4 as usize {
        // 26^4, 4 letters should be available.
        // TODO: at this point we should parallelize the walk. :P
        BASE_4
    } else {
        // Wow you have a lot of notes, have the entire bitspace
        std::u32::MAX
    };

    let mut n = rand::random::<u32>() % pool_to_draw_from;
    loop {
        let attempt = num_to_string_id(n);
        if !previous_uuids.contains(&attempt) {
            return attempt;
        }

        // Increase us but keep us inside this pool of candidates.
        n = n + 1 % pool_to_draw_from;
    }
}

/// Encode a decimal number to a lowercase string comprised of letters.
fn num_to_string_id(num: u32) -> String {
    let mut out = String::new();
    let mut rem = num + BASE; // Allow 0 to have a code and start us off into the two digit zone.
    loop {
        let base_26_digit = rem % BASE;
        out.push(LETTERS[base_26_digit as usize]);

        rem = rem / BASE;
        if rem == 0 {
            break;
        }
    }

    return out;
}

#[test]
fn test_num_to_string_id() {
    // Make sure everything is unique.
    let mut all = std::collections::HashSet::new();
    let size = 50000;
    for i in 0..size {
        all.insert(num_to_string_id(i));
    }
    assert_eq!(size as usize, all.len());

    assert_eq!(num_to_string_id(0).len(), 2);
    assert_eq!(num_to_string_id(BASE).len(), 2);
    assert_eq!(num_to_string_id(BASE_2).len(), 3);
    assert_eq!(num_to_string_id(BASE_3).len(), 4);
    assert_eq!(num_to_string_id(BASE_4).len(), 5);
}
