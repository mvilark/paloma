//! Lull's data (one JSON file) and the sleep-time maths, pure functions of the schedule,
//! a time and a time zone.
use std::path::{Path, PathBuf};
use std::{env, fs, io::ErrorKind};

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use sunrise::{Coordinates, SolarDay, SolarEvent};

// The data shapes are one line each, as in the plan.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[rustfmt::skip]
pub struct Data { pub schedule: Schedule, pub habits: Vec<Habit>, pub nights: Vec<Night>, pub parked: Option<Parked> }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[rustfmt::skip]
pub struct Schedule { pub wake: Wake, pub sleep_minutes: u32, pub wind_down_minutes: u32, pub caffeine_hours: Option<u32>, pub ease: Option<Ease> }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
#[rustfmt::skip]
pub enum Wake { At(NaiveTime), Sunrise(Place) }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[rustfmt::skip]
pub struct Place { pub place: String, pub lat: f64, pub lon: f64 }

/// A late schedule moves 15 minutes earlier every 3 days, from `minutes` later on `from`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[rustfmt::skip]
pub struct Ease { pub minutes: u32, pub from: NaiveDate }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
#[rustfmt::skip]
pub enum Habit { Park, Screens, Light, Caffeine, Alcohol, Naps }

/// `date` is the evening the night started.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[rustfmt::skip]
pub struct Night { pub date: NaiveDate, pub bed: NaiveTime, pub up: NaiveTime, pub rested: u8, pub habits: Vec<Habit> }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[rustfmt::skip]
pub struct Parked { pub at: NaiveDateTime, pub note: String }

#[rustfmt::skip]
impl Habit {
    pub const ALL: [Habit; 6] = [Habit::Park, Habit::Screens, Habit::Light, Habit::Caffeine, Habit::Alcohol, Habit::Naps];
    const TEXT: [(&str, &str); 6] = [
        ("park your work", "write where you stopped, then stop"),
        ("screens off at wind-down", "evening screens push sleep later"),
        ("daylight within an hour of waking", "morning light sets the body clock"),
        ("no caffeine after the cut-off", "it lingers for hours"),
        ("no alcohol in the 3 hours before bed", "it breaks up the second half of the night"),
        ("naps under 20 minutes, before 15:00", "longer or later naps steal night sleep"),
    ];
    pub fn text(self) -> &'static str { Self::TEXT[self as usize].0 }
    pub fn why(self) -> &'static str { Self::TEXT[self as usize].1 }
}

#[rustfmt::skip]
impl Default for Data {
    fn default() -> Self {
        let schedule = Schedule { wake: Wake::At(at(7, 0)), sleep_minutes: 480, wind_down_minutes: 60, caffeine_hours: Some(8), ease: None };
        Data { schedule, habits: vec![Habit::Park, Habit::Screens, Habit::Light, Habit::Caffeine], nights: vec![], parked: None }
    }
}

pub fn at(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
}

/// `LULL_FILE`, else `.lull.json` in the home directory.
pub fn path() -> PathBuf {
    let home = env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default();
    env::var_os("LULL_FILE").map_or_else(|| PathBuf::from(home).join(".lull.json"), PathBuf::from)
}

/// `Ok(None)` when the file does not exist.
pub fn load(path: &Path) -> Result<Option<Data>, String> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| e.to_string()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn save(path: &Path, data: &Data) -> Result<(), String> {
    let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

/// Where `now` is in the day: the night theme shows in the last two phases.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Day,
    WindDown,
    Night,
}

/// The night that starts on a date, on one timeline: midnight needs no special case.
#[derive(Clone, Debug, PartialEq)]
#[rustfmt::skip]
pub struct Plan { pub caffeine: Option<NaiveDateTime>, pub wind_down: NaiveDateTime, pub bed: NaiveDateTime, pub wake: NaiveDateTime, pub ease: u32 }

/// The wake time on the morning of `date`, before easing. A sunrise is rounded to 5
/// minutes and kept within 06:00-08:00; with no sunrise (polar day or night) or bad coordinates it is 08:00.
pub fn wake_on(schedule: &Schedule, date: NaiveDate, tz: &impl TimeZone) -> NaiveTime {
    match &schedule.wake {
        Wake::At(time) => *time,
        Wake::Sunrise(p) => Coordinates::new(p.lat, p.lon)
            .and_then(|c| SolarDay::new(c, date).event_time(SolarEvent::Sunrise))
            .map_or(at(8, 0), |utc| {
                // To the nearest 5 minutes, so the night reads 00:30, not 00:29.
                let m = (utc.with_timezone(tz).time().num_seconds_from_midnight() + 150) / 300 * 5;
                at(m / 60 % 24, m % 60).clamp(at(6, 0), at(8, 0))
            }),
    }
}

/// Minutes the night starting on `night` is later than the goal: 15 less every 3 days.
pub fn ease_minutes(schedule: &Schedule, night: NaiveDate) -> u32 {
    schedule.ease.as_ref().map_or(0, |e| {
        e.minutes
            .saturating_sub(15 * ((night - e.from).num_days().max(0) as u32 / 3))
    })
}

pub fn plan(schedule: &Schedule, night: NaiveDate, tz: &impl TimeZone) -> Plan {
    let morning = night.succ_opt().unwrap();
    let ease = ease_minutes(schedule, night);
    let wake = morning.and_time(wake_on(schedule, morning, tz)) + Duration::minutes(ease.into());
    let bed = wake - Duration::minutes(schedule.sleep_minutes.into());
    let caffeine = schedule
        .caffeine_hours
        .map(|h| bed - Duration::hours(h.into()));
    Plan {
        caffeine,
        wind_down: bed - Duration::minutes(schedule.wind_down_minutes.into()),
        bed,
        wake,
        ease,
    }
}

/// The phase at `now`, with the plan of the night `now` belongs to or of the next one.
pub fn phase(schedule: &Schedule, now: NaiveDateTime, tz: &impl TimeZone) -> (Phase, Plan) {
    let today = now.date();
    let last = plan(schedule, today.pred_opt().unwrap(), tz);
    let tonight = plan(schedule, today, tz);
    for p in [&last, &tonight] {
        if (p.wind_down..p.bed).contains(&now) {
            return (Phase::WindDown, p.clone());
        } else if (p.bed..p.wake).contains(&now) {
            return (Phase::Night, p.clone());
        }
    }
    let tomorrow = plan(schedule, today.succ_opt().unwrap(), tz);
    (
        Phase::Day,
        [last, tonight, tomorrow]
            .into_iter()
            .find(|p| now < p.wake)
            .unwrap(),
    )
}

/// The evening the night being logged started: at 07:30 on the 12th it is the 11th.
pub fn night_date(now: NaiveDateTime) -> NaiveDate {
    (now - Duration::hours(12)).date()
}

/// `b - a` in minutes, taken in (-720, 720] across midnight.
pub fn minutes_between(a: NaiveTime, b: NaiveTime) -> i64 {
    ((b - a).num_minutes() + 719).rem_euclid(1440) - 719
}

/// Latest minus earliest bedtime in minutes: counted from 18:00, so 23:30 and 00:30 are 60 apart.
pub fn spread(times: &[NaiveTime]) -> u32 {
    let from_6pm = times
        .iter()
        .map(|t| (t.num_seconds_from_midnight() / 60 + 360) % 1440);
    from_6pm.clone().max().unwrap_or(0) - from_6pm.min().unwrap_or(0)
}

/// Minutes to ease in when you fall asleep at `usual` and the goal is `target_bed`: a
/// multiple of 15, when it is more than half an hour.
pub fn ease_for(target_bed: NaiveTime, usual: NaiveTime) -> Option<u32> {
    let late = minutes_between(target_bed, usual);
    (late > 30).then(|| (late / 15 * 15) as u32)
}

/// `7 h 25 min`, `45 min`, `2 h`.
pub fn hm(minutes: impl Into<i64>) -> String {
    let m = minutes.into();
    match (m / 60, m % 60) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

#[cfg(test)]
mod tests {
    use chrono::FixedOffset;

    use super::*;

    fn date(m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, d).unwrap()
    }

    fn time(d: NaiveDate, h: u32, m: u32) -> NaiveDateTime {
        d.and_time(at(h, m))
    }

    fn utc() -> FixedOffset {
        FixedOffset::east_opt(0).unwrap()
    }

    fn fixed() -> Schedule {
        Data::default().schedule
    }

    fn sunrise(place: &str, lat: f64, lon: f64, day: NaiveDate, hours: i32) -> NaiveTime {
        let wake = Wake::Sunrise(Place {
            place: place.into(),
            lat,
            lon,
        });
        let s = Schedule { wake, ..fixed() };
        wake_on(&s, day, &FixedOffset::east_opt(hours * 3600).unwrap())
    }

    #[test]
    fn the_night_crosses_midnight() {
        let p = plan(&fixed(), date(10, 11), &utc());
        assert_eq!(p.wake, time(date(10, 12), 7, 0));
        assert_eq!(p.bed, time(date(10, 11), 23, 0));
        assert_eq!(p.wind_down, time(date(10, 11), 22, 0));
        assert_eq!(p.caffeine, Some(time(date(10, 11), 15, 0)));
    }

    #[test]
    fn phases_follow_the_plan() {
        let phase_at = |d, h, m| phase(&fixed(), time(d, h, m), &utc());
        assert_eq!(phase_at(date(10, 11), 14, 0).0, Phase::Day);
        assert_eq!(phase_at(date(10, 11), 22, 30).0, Phase::WindDown);
        assert_eq!(phase_at(date(10, 11), 23, 30).0, Phase::Night);
        let (p, night) = phase_at(date(10, 12), 3, 0);
        assert_eq!((p, night.bed), (Phase::Night, time(date(10, 11), 23, 0)));
        assert_eq!(phase_at(date(10, 12), 7, 0).0, Phase::Day);
    }

    /// Easing in can push a night past midnight: at 00:05 it has not started yet.
    #[test]
    fn an_eased_night_is_next_until_it_starts() {
        let ease = Some(Ease {
            minutes: 120,
            from: date(10, 11),
        });
        let s = Schedule {
            ease,
            wind_down_minutes: 30,
            ..fixed()
        };
        let (p, night) = phase(&s, time(date(10, 13), 0, 5), &utc());
        assert_eq!((p, night.bed), (Phase::Day, time(date(10, 13), 1, 0)));
    }

    #[test]
    fn sunrise_is_clamped() {
        // 05:48 UTC, 07:48 in summer time, rounded to 07:50.
        assert_eq!(sunrise("Barcelona", 41.39, 2.17, date(10, 1), 2), at(7, 50));
        assert_eq!(sunrise("Helsinki", 60.17, 24.94, date(6, 21), 3), at(6, 0));
        assert_eq!(
            sunrise("Reykjavik", 64.15, -21.94, date(12, 21), 0),
            at(8, 0)
        );
        assert_eq!(sunrise("Tromso", 69.65, 18.96, date(12, 21), 1), at(8, 0));
    }

    #[test]
    fn easing_steps_every_three_days() {
        let s = Schedule {
            ease: Some(Ease {
                minutes: 120,
                from: date(10, 11),
            }),
            ..fixed()
        };
        let ease = |d| ease_minutes(&s, d);
        assert_eq!((ease(date(10, 11)), ease(date(10, 13))), (120, 120));
        assert_eq!(ease(date(10, 14)), 105);
        assert_eq!((ease(date(11, 3)), ease(date(11, 4))), (15, 0));
    }

    #[test]
    fn ease_needs_more_than_half_an_hour() {
        assert_eq!(ease_for(at(23, 0), at(23, 30)), None);
        assert_eq!(ease_for(at(23, 0), at(1, 10)), Some(120));
        assert_eq!(ease_for(at(23, 0), at(22, 0)), None);
    }

    #[test]
    fn spread_and_minutes_cross_midnight() {
        assert_eq!(spread(&[at(23, 30), at(0, 30), at(23, 45)]), 60);
        assert_eq!(minutes_between(at(23, 30), at(0, 30)), 60);
        assert_eq!(minutes_between(at(0, 30), at(23, 30)), -60);
    }

    #[test]
    fn night_date_is_the_evening() {
        assert_eq!(night_date(time(date(10, 12), 7, 30)), date(10, 11));
        assert_eq!(night_date(time(date(10, 11), 23, 0)), date(10, 11));
    }

    #[test]
    fn hm_reads_naturally() {
        assert_eq!(hm(445), "7 h 25 min");
        assert_eq!(
            (hm(45), hm(120), hm(60u32)),
            ("45 min".into(), "2 h".into(), "1 h".into())
        );
    }

    #[test]
    fn the_data_file_round_trips() {
        let data: Data =
            serde_json::from_str(include_str!("../../tests/fixtures/lull.json")).unwrap();
        let json = serde_json::to_string_pretty(&data).unwrap();
        assert_eq!(serde_json::from_str::<Data>(&json).unwrap(), data);
        assert!(serde_json::from_str::<Data>(include_str!(
            "../../tests/fixtures/lull-broken.json"
        ))
        .is_err());
    }
}
