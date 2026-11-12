//! Unix timestamps to and from ISO 8601, in UTC.

use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{Op, OpError, ParamKind, ParamSpec, Tag};

const LINK: &str = "https://en.wikipedia.org/wiki/Unix_time";
const MS_PER_DAY: i128 = 86_400_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "lowercase")]
pub enum TimeUnit {
    #[default]
    Seconds,
    Milliseconds,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct TimestampConfig {
    pub unit: TimeUnit,
}

fn params() -> Vec<ParamSpec> {
    let units = ["seconds", "milliseconds"].map(String::from).to_vec();
    vec![ParamSpec::new("unit", "Unit", ParamKind::Choice { options: units })]
}

/// Days since 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
    let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let shifted_month = i64::from(if month > 2 { month - 3 } else { month + 9 });
    let day_of_year = (153 * shifted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 31,
    }
}

/// A decimal number of seconds or milliseconds, as milliseconds.
fn parse_timestamp(input: &str, unit: TimeUnit) -> Option<i128> {
    let input = input.trim();
    let (whole, fraction) = input.split_once('.').unwrap_or((input, ""));
    let negative = whole.starts_with('-');
    let whole: i128 = whole.parse::<i64>().ok()?.into();
    if !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(match unit {
        TimeUnit::Milliseconds => whole,
        TimeUnit::Seconds => {
            // Fractions of a second, to the millisecond.
            let digits: String = fraction.chars().chain("000".chars()).take(3).collect();
            let fraction: i128 = digits.parse().ok()?;
            whole * 1000 + if negative { -fraction } else { fraction }
        }
    })
}

pub struct FromUnixTimestamp;

impl Op for FromUnixTimestamp {
    const ID: &'static str = "from-unix-timestamp";
    const NAME: &'static str = "From Unix Timestamp";
    const DESCRIPTION: &'static str =
        "Turns seconds or milliseconds since 1970 into an ISO 8601 date and time in UTC";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::DataFormat];

    type Config = TimestampConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        params()
    }

    fn run(config: &TimestampConfig, input: &str) -> Result<String, OpError> {
        let total = parse_timestamp(input, config.unit).ok_or_else(|| {
            OpError::invalid_input(format!("\"{}\" is not a number", input.trim()))
        })?;
        let days = total.div_euclid(MS_PER_DAY);
        let in_day = total.rem_euclid(MS_PER_DAY) as u32;
        let days = i64::try_from(days).ok().filter(|days| days.abs() < 3_660_000);
        let (year, month, day) = civil_from_days(
            days.ok_or_else(|| OpError::invalid_input("The timestamp is outside years 0-9999"))?,
        );
        if !(0..=9999).contains(&year) {
            return Err(OpError::invalid_input("The timestamp is outside years 0-9999"));
        }
        let (hours, minutes, seconds, millis) =
            (in_day / 3_600_000, in_day / 60_000 % 60, in_day / 1000 % 60, in_day % 1000);
        let fraction = if millis == 0 { String::new() } else { format!(".{millis:03}") };
        Ok(format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}{fraction}Z"))
    }
}

/// Reads `digits` ASCII digits from the front of `rest`.
fn take_number(rest: &mut &str, digits: usize) -> Option<u32> {
    let head = rest.get(..digits)?;
    if !head.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    *rest = &rest[digits..];
    head.parse().ok()
}

fn take_char(rest: &mut &str, expected: &[char]) -> bool {
    match rest.chars().next() {
        Some(c) if expected.contains(&c) => {
            *rest = &rest[c.len_utf8()..];
            true
        }
        _ => false,
    }
}

/// `YYYY-MM-DD`, optionally followed by `THH:MM[:SS[.fff]]` and `Z` or `±HH:MM`.
/// Without an offset the time is taken as UTC.
fn parse_iso(input: &str) -> Option<i128> {
    let mut rest = input.trim();
    let year = i64::from(take_number(&mut rest, 4)?);
    take_char(&mut rest, &['-']).then_some(())?;
    let month = take_number(&mut rest, 2)?;
    take_char(&mut rest, &['-']).then_some(())?;
    let day = take_number(&mut rest, 2)?;
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }

    let mut millis: i128 = 0;
    if take_char(&mut rest, &['T', 't', ' ']) {
        let hours = take_number(&mut rest, 2)?;
        take_char(&mut rest, &[':']).then_some(())?;
        let minutes = take_number(&mut rest, 2)?;
        let seconds = if take_char(&mut rest, &[':']) { take_number(&mut rest, 2)? } else { 0 };
        // 60 is a leap second.
        if hours > 23 || minutes > 59 || seconds > 60 {
            return None;
        }
        millis = i128::from(hours * 3600 + minutes * 60 + seconds) * 1000;
        if take_char(&mut rest, &['.', ',']) {
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            let fraction: String = rest[..digits].chars().chain("000".chars()).take(3).collect();
            (digits > 0).then_some(())?;
            millis += fraction.parse::<i128>().ok()?;
            rest = &rest[digits..];
        }
        if take_char(&mut rest, &['Z', 'z']) {
        } else if let Some(sign) = rest.chars().next().filter(|c| matches!(c, '+' | '-')) {
            rest = &rest[1..];
            let hours = take_number(&mut rest, 2)?;
            take_char(&mut rest, &[':']);
            let minutes = if rest.is_empty() { 0 } else { take_number(&mut rest, 2)? };
            if hours > 23 || minutes > 59 {
                return None;
            }
            let offset = i128::from(hours * 60 + minutes) * 60_000;
            // A time written with +03:00 is three hours ahead of UTC.
            millis += if sign == '+' { -offset } else { offset };
        }
    }
    rest.is_empty().then(|| i128::from(days_from_civil(year, month, day)) * MS_PER_DAY + millis)
}

pub struct ToUnixTimestamp;

impl Op for ToUnixTimestamp {
    const ID: &'static str = "to-unix-timestamp";
    const NAME: &'static str = "To Unix Timestamp";
    const DESCRIPTION: &'static str =
        "Turns an ISO 8601 date and time into seconds or milliseconds since 1970";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::DataFormat];

    type Config = TimestampConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        params()
    }

    fn run(config: &TimestampConfig, input: &str) -> Result<String, OpError> {
        let millis = parse_iso(input).ok_or_else(|| {
            OpError::invalid_input(format!(
                "\"{}\" is not a date like 2024-02-29 or 2024-02-29T12:30:00Z",
                input.trim()
            ))
        })?;
        Ok(match config.unit {
            TimeUnit::Milliseconds => millis.to_string(),
            TimeUnit::Seconds => millis.div_euclid(1000).to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ErrorCode;

    fn from(input: &str, unit: TimeUnit) -> String {
        FromUnixTimestamp::run(&TimestampConfig { unit }, input).unwrap()
    }

    fn to(input: &str, unit: TimeUnit) -> String {
        ToUnixTimestamp::run(&TimestampConfig { unit }, input).unwrap()
    }

    #[test]
    fn known_moments() {
        assert_eq!(from("0", TimeUnit::Seconds), "1970-01-01T00:00:00Z");
        assert_eq!(from("1516239022", TimeUnit::Seconds), "2018-01-18T01:30:22Z");
        assert_eq!(from(" 951782400\n", TimeUnit::Seconds), "2000-02-29T00:00:00Z");
        assert_eq!(from("-1", TimeUnit::Seconds), "1969-12-31T23:59:59Z");
        assert_eq!(from("2147483647", TimeUnit::Seconds), "2038-01-19T03:14:07Z");
        assert_eq!(from("1516239022123", TimeUnit::Milliseconds), "2018-01-18T01:30:22.123Z");
        assert_eq!(from("1516239022.5", TimeUnit::Seconds), "2018-01-18T01:30:22.500Z");
    }

    #[test]
    fn dates_are_read_back() {
        assert_eq!(to("2018-01-18T01:30:22Z", TimeUnit::Seconds), "1516239022");
        assert_eq!(to("2018-01-18 01:30:22", TimeUnit::Seconds), "1516239022");
        assert_eq!(to("2018-01-18", TimeUnit::Seconds), "1516233600");
        assert_eq!(to("2018-01-18T04:30:22+03:00", TimeUnit::Seconds), "1516239022");
        assert_eq!(to("2018-01-17T20:30:22-0500", TimeUnit::Seconds), "1516239022");
        assert_eq!(to("2018-01-18T01:30:22.123Z", TimeUnit::Milliseconds), "1516239022123");
        assert_eq!(to("1969-12-31T23:59:59Z", TimeUnit::Seconds), "-1");
    }

    #[test]
    fn every_day_over_centuries_round_trips() {
        for days in (-200_000..200_000).step_by(37) {
            let (year, month, day) = civil_from_days(days);
            assert_eq!(days_from_civil(year, month, day), days);
            assert!(day >= 1 && day <= days_in_month(year, month));
        }
    }

    #[test]
    fn nonsense_is_an_input_error() {
        for bad in ["soon", "", "1e9", "99999999999999999999"] {
            let error = FromUnixTimestamp::run(&TimestampConfig::default(), bad).unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidInput, "{bad}");
        }
        for bad in ["2023-02-29", "2018-13-01", "2018-01-18T25:00", "2018-01-18 extra", "18/01/2018"] {
            let error = ToUnixTimestamp::run(&TimestampConfig::default(), bad).unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidInput, "{bad}");
        }
    }
}
