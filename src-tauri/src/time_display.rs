//! Times, dates and durations in words, in the user's time zone.

use domain::{CivilDate, SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MINUTE};
use fluent_bundle::FluentArgs;

use crate::text_catalog::TextCatalog;

/// "14 h 05" in the user's time zone. O(1).
pub fn format_time_of_day(utc: i64, offset: i64, catalog: &TextCatalog) -> String {
    let seconds = (utc + offset).rem_euclid(SECONDS_PER_DAY);
    let mut args = FluentArgs::new();
    args.set("hours", (seconds / SECONDS_PER_HOUR).to_string());
    args.set(
        "minutes",
        format!("{:02}", seconds % SECONDS_PER_HOUR / SECONDS_PER_MINUTE),
    );
    catalog.render("time-of-day", Some(&args))
}

/// "12/10, 14 h 05" in the user's time zone. O(1).
pub fn format_date_time(utc: i64, offset: i64, catalog: &TextCatalog) -> String {
    let local = utc + offset;
    let CivilDate { month, day, .. } = CivilDate::from_days(local.div_euclid(SECONDS_PER_DAY));
    let mut args = FluentArgs::new();
    args.set("day", format!("{day:02}"));
    args.set("month", format!("{month:02}"));
    args.set("time", format_time_of_day(utc, offset, catalog));
    catalog.render("date-time", Some(&args))
}

/// "12/10, 14 h" in the user's time zone. O(1).
pub fn format_date_hour(utc: i64, offset: i64, catalog: &TextCatalog) -> String {
    let local = utc + offset;
    let CivilDate { month, day, .. } = CivilDate::from_days(local.div_euclid(SECONDS_PER_DAY));
    let mut args = FluentArgs::new();
    args.set("day", format!("{day:02}"));
    args.set("month", format!("{month:02}"));
    args.set(
        "hours",
        (local.rem_euclid(SECONDS_PER_DAY) / SECONDS_PER_HOUR).to_string(),
    );
    catalog.render("date-hour", Some(&args))
}

/// The two largest units: "42 s", "12 min", "3 h 12 min", "2 j 5 h". O(1).
pub fn format_duration(seconds: i64, catalog: &TextCatalog) -> String {
    let seconds = seconds.max(0);
    if seconds < SECONDS_PER_MINUTE {
        return catalog.render_value("duration-seconds", "value", seconds.to_string());
    }
    if seconds < SECONDS_PER_HOUR {
        return catalog.render_value(
            "duration-minutes",
            "value",
            (seconds / SECONDS_PER_MINUTE).to_string(),
        );
    }
    let mut args = FluentArgs::new();
    if seconds < SECONDS_PER_DAY {
        let minutes = seconds % SECONDS_PER_HOUR / SECONDS_PER_MINUTE;
        args.set("hours", (seconds / SECONDS_PER_HOUR).to_string());
        args.set("minutes", minutes.to_string());
        let key = if minutes == 0 {
            "duration-hours"
        } else {
            "duration-hours-minutes"
        };
        return catalog.render(key, Some(&args));
    }
    let hours = seconds % SECONDS_PER_DAY / SECONDS_PER_HOUR;
    args.set("days", (seconds / SECONDS_PER_DAY).to_string());
    args.set("hours", hours.to_string());
    let key = if hours == 0 {
        "duration-days"
    } else {
        "duration-days-hours"
    };
    catalog.render(key, Some(&args))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NBSP: char = '\u{a0}';
    /// 2023-11-14 00:00 UTC.
    const MIDNIGHT: i64 = 1_699_920_000;
    /// Paris in winter, one hour ahead of UTC.
    const PARIS: i64 = 3600;

    /// `~` marks a no-break space.
    fn insert_no_break_spaces(text: &str) -> String {
        text.replace('~', &NBSP.to_string())
    }

    #[test]
    fn writes_times_and_dates_in_the_user_time_zone() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let time = format_time_of_day(MIDNIGHT + 13 * 3600 + 5 * 60, PARIS, &catalog);
        let date = format_date_hour(MIDNIGHT + 23 * 3600, PARIS, &catalog);

        // Assert
        assert_eq!(time, insert_no_break_spaces("14~h~05"));
        assert_eq!(date, insert_no_break_spaces("15/11, 0~h"));
    }

    #[test]
    fn keeps_the_two_largest_units_of_a_duration() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let durations: Vec<String> = [
            42,
            12 * 60,
            3 * 3600,
            3 * 3600 + 12 * 60,
            2 * 86_400 + 5 * 3600,
        ]
        .into_iter()
        .map(|seconds| format_duration(seconds, &catalog))
        .collect();

        // Assert
        assert_eq!(
            durations,
            ["42~s", "12~min", "3~h", "3~h 12~min", "2~j 5~h"].map(insert_no_break_spaces)
        );
    }
}
