//! Time axes over Unix milliseconds, with calendar-aligned ticks.
//!
//! Times are `i64` milliseconds since 1970-01-01 UTC. Labels are shown at a
//! fixed UTC offset in minutes (the app passes the user's), which is exact
//! for daily and coarser data; a daylight-saving change moves intraday ticks
//! by the hour it shifts.

/// Milliseconds in a second.
pub const SECOND: i64 = 1_000;
/// Milliseconds in a minute.
pub const MINUTE: i64 = 60 * SECOND;
/// Milliseconds in an hour.
pub const HOUR: i64 = 60 * MINUTE;
/// Milliseconds in a day.
pub const DAY: i64 = 24 * HOUR;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// A calendar date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CivilDate {
    /// The year, such as 2026.
    pub year: i64,
    /// The month, `1..=12`.
    pub month: u32,
    /// The day of the month, `1..=31`.
    pub day: u32,
}

/// The date `days` after 1970-01-01 (Howard Hinnant's `civil_from_days`).
///
/// ```
/// use g3_charts::geom::{CivilDate, civil_from_days};
///
/// assert_eq!(civil_from_days(0), CivilDate { year: 1970, month: 1, day: 1 });
/// ```
pub fn civil_from_days(days: i64) -> CivilDate {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    CivilDate { year, month, day }
}

/// Days from 1970-01-01 to a date (Howard Hinnant's `days_from_civil`).
///
/// ```
/// use g3_charts::geom::days_from_civil;
///
/// assert_eq!(days_from_civil(2000, 3, 1), 11_017);
/// ```
pub fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let month = i64::from(month);
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The spacing between time ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeInterval {
    /// A fixed number of milliseconds (seconds to days).
    Fixed(i64),
    /// Whole months: 1, 3 or 6.
    Months(u32),
    /// Whole years: 1, 2, 5, 10 and so on.
    Years(i64),
}

impl TimeInterval {
    /// Roughly how long one interval is, for choosing between them.
    pub fn approx_ms(self) -> i64 {
        match self {
            TimeInterval::Fixed(ms) => ms,
            TimeInterval::Months(n) => i64::from(n) * 30 * DAY + i64::from(n) * 10 * HOUR,
            TimeInterval::Years(n) => n * 365 * DAY + n * 6 * HOUR,
        }
    }
}

const FIXED: [i64; 15] = [
    SECOND,
    5 * SECOND,
    15 * SECOND,
    30 * SECOND,
    MINUTE,
    5 * MINUTE,
    15 * MINUTE,
    30 * MINUTE,
    HOUR,
    3 * HOUR,
    6 * HOUR,
    12 * HOUR,
    DAY,
    2 * DAY,
    7 * DAY,
];

/// The calendar interval that splits `span_ms` into about `count` ticks.
///
/// ```
/// use g3_charts::geom::{DAY, TimeInterval, time_interval};
///
/// assert_eq!(time_interval(365 * DAY, 4), TimeInterval::Months(3));
/// ```
pub fn time_interval(span_ms: i64, count: usize) -> TimeInterval {
    let target = span_ms.max(1) as f64 / count.max(1) as f64;
    let mut candidates: Vec<TimeInterval> =
        FIXED.iter().map(|&ms| TimeInterval::Fixed(ms)).collect();
    candidates.extend([1, 3, 6].map(TimeInterval::Months));
    let years = span_ms as f64 / (365.25 * DAY as f64);
    let year_step = super::nice_step(years, count.max(1)).max(1.0) as i64;
    candidates.push(TimeInterval::Years(1));
    if year_step > 1 {
        candidates.push(TimeInterval::Years(year_step));
    }
    // Closest on a log scale, so 40 days picks one month rather than a week.
    candidates
        .into_iter()
        .min_by(|a, b| {
            let da = (a.approx_ms() as f64 / target).ln().abs();
            let db = (b.approx_ms() as f64 / target).ln().abs();
            da.total_cmp(&db)
        })
        .unwrap_or(TimeInterval::Fixed(DAY))
}

/// Tick times inside `[start, end]` at `interval`, aligned to the calendar at
/// `offset_minutes` from UTC (midnight, the first of a month, January).
pub fn time_ticks(start: i64, end: i64, interval: TimeInterval, offset_minutes: i32) -> Vec<i64> {
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    let offset = i64::from(offset_minutes) * MINUTE;
    let (local_start, local_end) = (start + offset, end + offset);
    let mut ticks = Vec::new();
    match interval {
        TimeInterval::Fixed(step) => {
            let step = step.max(1);
            // Weeks start on Monday; 1970-01-01 was a Thursday.
            let anchor = if step == 7 * DAY { 4 * DAY } else { 0 };
            let mut t = (local_start - anchor).div_euclid(step) * step + anchor;
            if t < local_start {
                t += step;
            }
            while t <= local_end && ticks.len() < 1_000 {
                ticks.push(t - offset);
                t += step;
            }
        }
        TimeInterval::Months(_) | TimeInterval::Years(_) => {
            let months_per = match interval {
                TimeInterval::Months(n) => i64::from(n.max(1)),
                TimeInterval::Years(n) => n.max(1) * 12,
                TimeInterval::Fixed(_) => 1,
            };
            let first = civil_from_days(local_start.div_euclid(DAY));
            let mut index = first.year * 12 + i64::from(first.month - 1);
            index = index.div_euclid(months_per) * months_per;
            while ticks.len() < 1_000 {
                let (year, month) = (index.div_euclid(12), index.rem_euclid(12) as u32 + 1);
                let t = days_from_civil(year, month, 1) * DAY;
                if t > local_end {
                    break;
                }
                if t >= local_start {
                    ticks.push(t - offset);
                }
                index += months_per;
            }
        }
    }
    ticks
}

/// A short label for a tick at `interval`: `2026`, `Mar 2026` (January and
/// the first tick), `Mar`, `Mar 5`, `14:00` or `14:00:05`.
pub fn time_label(t: i64, interval: TimeInterval, offset_minutes: i32, first: bool) -> String {
    let local = t + i64::from(offset_minutes) * MINUTE;
    let date = civil_from_days(local.div_euclid(DAY));
    let in_day = local.rem_euclid(DAY);
    let (hour, minute, second) = (
        in_day / HOUR,
        in_day % HOUR / MINUTE,
        in_day % MINUTE / SECOND,
    );
    let month = MONTHS[(date.month - 1) as usize];
    match interval {
        TimeInterval::Years(_) => date.year.to_string(),
        TimeInterval::Months(_) if first || date.month == 1 => format!("{month} {}", date.year),
        TimeInterval::Months(_) => month.to_string(),
        TimeInterval::Fixed(step) if step >= DAY => format!("{month} {}", date.day),
        TimeInterval::Fixed(step) if step >= MINUTE => {
            if in_day == 0 && !first {
                format!("{month} {}", date.day)
            } else {
                format!("{hour:02}:{minute:02}")
            }
        }
        TimeInterval::Fixed(_) => format!("{hour:02}:{minute:02}:{second:02}"),
    }
}

/// A full date for a tooltip or table row: `Mar 5, 2026`, plus `14:05` when
/// the time is not midnight.
pub fn date_label(t: i64, offset_minutes: i32) -> String {
    let local = t + i64::from(offset_minutes) * MINUTE;
    let date = civil_from_days(local.div_euclid(DAY));
    let in_day = local.rem_euclid(DAY);
    let month = MONTHS[(date.month - 1) as usize];
    if in_day == 0 {
        format!("{month} {}, {}", date.day, date.year)
    } else {
        format!(
            "{month} {}, {} {:02}:{:02}",
            date.day,
            date.year,
            in_day / HOUR,
            in_day % HOUR / MINUTE
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(year: i64, month: u32, day: u32) -> i64 {
        days_from_civil(year, month, day) * DAY
    }

    #[test]
    fn civil_dates_round_trip_across_eras_and_leap_years() {
        for days in (-800_000..800_000).step_by(997) {
            let date = civil_from_days(days);
            assert_eq!(days_from_civil(date.year, date.month, date.day), days);
        }
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)).day, 29);
        assert_eq!(
            days_from_civil(2024, 3, 1) - days_from_civil(2024, 2, 28),
            2
        );
        assert_eq!(
            days_from_civil(2023, 3, 1) - days_from_civil(2023, 2, 28),
            1
        );
        assert_eq!(
            days_from_civil(1900, 3, 1) - days_from_civil(1900, 2, 28),
            1
        );
        assert_eq!(
            days_from_civil(2000, 3, 1) - days_from_civil(2000, 2, 28),
            2
        );
        assert_eq!(
            civil_from_days(-1),
            CivilDate {
                year: 1969,
                month: 12,
                day: 31
            }
        );
    }

    #[test]
    fn intervals_scale_with_the_span() {
        assert_eq!(
            time_interval(60 * SECOND, 4),
            TimeInterval::Fixed(15 * SECOND)
        );
        assert_eq!(time_interval(DAY, 4), TimeInterval::Fixed(6 * HOUR));
        assert_eq!(time_interval(30 * DAY, 4), TimeInterval::Fixed(7 * DAY));
        assert_eq!(time_interval(365 * DAY, 12), TimeInterval::Months(1));
        assert_eq!(time_interval(5 * 365 * DAY, 5), TimeInterval::Years(1));
        assert_eq!(time_interval(40 * 365 * DAY, 4), TimeInterval::Years(10));
    }

    #[test]
    fn month_ticks_land_on_the_first_across_a_year_boundary() {
        let ticks = time_ticks(at(2023, 11, 15), at(2024, 3, 2), TimeInterval::Months(1), 0);
        assert_eq!(
            ticks,
            vec![
                at(2023, 12, 1),
                at(2024, 1, 1),
                at(2024, 2, 1),
                at(2024, 3, 1)
            ]
        );
        let quarters = time_ticks(at(2023, 2, 1), at(2024, 1, 1), TimeInterval::Months(3), 0);
        assert_eq!(
            quarters,
            vec![
                at(2023, 4, 1),
                at(2023, 7, 1),
                at(2023, 10, 1),
                at(2024, 1, 1)
            ]
        );
        let years = time_ticks(at(2019, 6, 1), at(2026, 1, 1), TimeInterval::Years(2), 0);
        assert_eq!(
            years,
            vec![
                at(2020, 1, 1),
                at(2022, 1, 1),
                at(2024, 1, 1),
                at(2026, 1, 1)
            ]
        );
    }

    #[test]
    fn day_ticks_follow_local_midnight() {
        // UTC-5: local midnight is 05:00 UTC.
        let offset = -300;
        let ticks = time_ticks(
            at(2026, 3, 1),
            at(2026, 3, 3),
            TimeInterval::Fixed(DAY),
            offset,
        );
        assert_eq!(
            ticks,
            vec![at(2026, 3, 1) + 5 * HOUR, at(2026, 3, 2) + 5 * HOUR]
        );
        assert_eq!(
            time_label(ticks[0], TimeInterval::Fixed(DAY), offset, false),
            "Mar 1"
        );
        let weeks = time_ticks(
            at(2026, 3, 1),
            at(2026, 3, 20),
            TimeInterval::Fixed(7 * DAY),
            0,
        );
        // 2026-03-02 is a Monday.
        assert_eq!(weeks[0], at(2026, 3, 2));
    }

    #[test]
    fn labels_match_the_interval() {
        let t = at(2026, 1, 1);
        assert_eq!(time_label(t, TimeInterval::Years(1), 0, false), "2026");
        assert_eq!(time_label(t, TimeInterval::Months(1), 0, false), "Jan 2026");
        assert_eq!(
            time_label(at(2026, 4, 1), TimeInterval::Months(3), 0, false),
            "Apr"
        );
        assert_eq!(
            time_label(at(2026, 4, 1), TimeInterval::Months(3), 0, true),
            "Apr 2026"
        );
        let afternoon = t + 14 * HOUR + 5 * SECOND;
        assert_eq!(
            time_label(afternoon, TimeInterval::Fixed(HOUR), 0, false),
            "14:00"
        );
        assert_eq!(
            time_label(afternoon, TimeInterval::Fixed(5 * SECOND), 0, false),
            "14:00:05"
        );
        assert_eq!(date_label(t, 0), "Jan 1, 2026");
        assert_eq!(
            date_label(t + 9 * HOUR + 30 * MINUTE, 0),
            "Jan 1, 2026 09:30"
        );
    }

    #[test]
    fn reversed_and_empty_ranges_are_safe() {
        let t = at(2026, 1, 1);
        assert_eq!(
            time_ticks(t + DAY, t, TimeInterval::Fixed(DAY), 0),
            vec![t, t + DAY]
        );
        assert!(time_ticks(t + 1, t + 2, TimeInterval::Years(1), 0).is_empty());
    }
}
