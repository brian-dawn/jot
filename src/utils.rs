use unicode_segmentation::UnicodeSegmentation;

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

/// Remove ANSI escape codes and count real graphemes.
pub fn count_real_chars(input: &str) -> Option<usize> {
    Some(
        std::str::from_utf8(&strip_ansi_escapes::strip(input).ok()?)
            .ok()?
            .graphemes(true)
            .count(),
    )
}
