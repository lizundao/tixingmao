use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Deserialize)]
struct CalendarFile {
    years: HashMap<String, YearData>,
}

#[derive(Debug, Deserialize)]
struct YearData {
    holidays: HashMap<String, String>,
    workdays: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OverrideKind {
    Holiday,
    Workday,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarOverride {
    pub date: String,
    pub kind: OverrideKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayStatus {
    pub date: String,
    pub is_workday: bool,
    pub kind: String,
    pub label: String,
    pub data_available: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarInfo {
    pub today: DayStatus,
    pub coverage_years: Vec<i32>,
    pub note: String,
}

static CALENDAR: OnceLock<CalendarFile> = OnceLock::new();
static OVERRIDES: OnceLock<RwLock<HashMap<String, OverrideKind>>> = OnceLock::new();

fn calendar() -> &'static CalendarFile {
    CALENDAR.get_or_init(|| {
        serde_json::from_str(include_str!("../data/china-holidays.json"))
            .expect("bundled Chinese holiday data must be valid")
    })
}

fn overrides() -> &'static RwLock<HashMap<String, OverrideKind>> {
    OVERRIDES.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn set_overrides(entries: Vec<CalendarOverride>) {
    let mut map = overrides()
        .write()
        .unwrap_or_else(|error| error.into_inner());
    map.clear();
    for entry in entries {
        map.insert(entry.date, entry.kind);
    }
}

pub fn list_overrides() -> Vec<CalendarOverride> {
    let map = overrides()
        .read()
        .unwrap_or_else(|error| error.into_inner());
    let mut entries: Vec<CalendarOverride> = map
        .iter()
        .map(|(date, kind)| CalendarOverride {
            date: date.clone(),
            kind: *kind,
        })
        .collect();
    entries.sort_by(|left, right| left.date.cmp(&right.date));
    entries
}

pub fn normalize_date(value: &str) -> Result<String, String> {
    let date = NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| "日期格式不正确，请使用 YYYY-MM-DD".to_string())?;
    Ok(key(date))
}

pub fn parse_date_lines(input: &str) -> Result<Vec<String>, String> {
    let mut dates: Vec<String> = Vec::new();
    for (index, raw) in input.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let date = NaiveDate::parse_from_str(line, "%Y-%m-%d").map_err(|_| {
            format!(
                "第 {} 行「{}」不是 YYYY-MM-DD 格式",
                index + 1,
                line
            )
        })?;
        let value = key(date);
        if !dates.contains(&value) {
            dates.push(value);
        }
    }
    Ok(dates)
}

fn key(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn is_weekend(date: NaiveDate) -> bool {
    matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}

pub fn status(date: NaiveDate) -> DayStatus {
    let date_key = key(date);
    let year_key = date.year().to_string();
    let data_available = calendar().years.contains_key(&year_key);

    let custom = overrides()
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .get(&date_key)
        .copied();
    if let Some(kind) = custom {
        return match kind {
            OverrideKind::Holiday => DayStatus {
                date: date_key,
                is_workday: false,
                kind: "holiday".to_string(),
                label: "自定义休息".to_string(),
                data_available,
            },
            OverrideKind::Workday => DayStatus {
                date: date_key,
                is_workday: true,
                kind: "workday".to_string(),
                label: "自定义调休上班".to_string(),
                data_available,
            },
        };
    }

    if let Some(year) = calendar().years.get(&year_key) {
        if let Some(label) = year.workdays.get(&date_key) {
            return DayStatus {
                date: date_key,
                is_workday: true,
                kind: "workday".to_string(),
                label: if label.is_empty() {
                    "调休上班".to_string()
                } else {
                    format!("调休上班 · {label}")
                },
                data_available,
            };
        }

        if let Some(label) = year.holidays.get(&date_key) {
            return DayStatus {
                date: date_key,
                is_workday: false,
                kind: "holiday".to_string(),
                label: label.clone(),
                data_available,
            };
        }
    }

    if is_weekend(date) {
        DayStatus {
            date: date_key,
            is_workday: false,
            kind: "weekend".to_string(),
            label: "周末".to_string(),
            data_available,
        }
    } else {
        DayStatus {
            date: date_key,
            is_workday: true,
            kind: "workday".to_string(),
            label: "工作日".to_string(),
            data_available,
        }
    }
}

pub fn day_status(value: &str) -> Result<DayStatus, String> {
    let date = NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| "日期格式不正确，请使用 YYYY-MM-DD".to_string())?;
    Ok(status(date))
}

pub fn is_workday(date: NaiveDate) -> bool {
    status(date).is_workday
}

pub fn next_workday(date: NaiveDate) -> Option<NaiveDate> {
    let mut candidate = date;
    for _ in 0..400 {
        if is_workday(candidate) {
            return Some(candidate);
        }
        candidate = candidate.checked_add_signed(Duration::days(1))?;
    }
    None
}

pub fn coverage_years() -> Vec<i32> {
    let mut years: Vec<i32> = calendar()
        .years
        .keys()
        .filter_map(|value| value.parse::<i32>().ok())
        .collect();
    years.sort_unstable();
    years
}

pub fn info(today: NaiveDate) -> CalendarInfo {
    let years = coverage_years();
    let custom_count = list_overrides().len();
    let note = if let (Some(first), Some(last)) = (years.first(), years.last()) {
        format!(
            "已内置 {first}–{last} 年中国法定节假日与调休数据，可在系统设置里补上新的休息日或调休上班日。"
        )
    } else {
        "未找到内置节假日数据，可在系统设置里手动添加休息日或调休上班日。".to_string()
    };
    let note = if custom_count > 0 {
        format!("{note} 当前有 {custom_count} 个自定义日期生效。")
    } else {
        note
    };

    CalendarInfo {
        today: status(today),
        coverage_years: years,
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn statutory_holiday_is_not_a_workday() {
        let status = status(date(2026, 10, 1));
        assert!(!status.is_workday);
        assert_eq!(status.kind, "holiday");
        assert_eq!(status.label, "国庆节");
    }

    #[test]
    fn makeup_weekend_is_a_workday() {
        assert!(is_workday(date(2026, 10, 10)));
        assert!(is_workday(date(2026, 2, 28)));
    }

    #[test]
    fn normal_weekend_is_not_a_workday() {
        assert!(!is_workday(date(2026, 10, 11)));
    }

    #[test]
    fn next_workday_skips_national_day() {
        assert_eq!(next_workday(date(2026, 10, 1)), Some(date(2026, 10, 8)));
    }

    #[test]
    fn parse_date_lines_skips_blanks_and_reports_bad_lines() {
        let parsed = parse_date_lines(
            "2026-01-01\n\n 2026-01-01 \n2026-02-15\n",
        )
        .unwrap();
        assert_eq!(parsed, vec!["2026-01-01", "2026-02-15"]);
        let error = parse_date_lines("2026-01-01\n2026/01/02").unwrap_err();
        assert!(error.contains("第 2 行"));
    }

    #[test]
    fn custom_overrides_change_workday_result() {
        set_overrides(vec![
            CalendarOverride {
                date: "2030-01-03".to_string(),
                kind: OverrideKind::Holiday,
            },
            CalendarOverride {
                date: "2030-01-05".to_string(),
                kind: OverrideKind::Workday,
            },
        ]);
        assert!(!is_workday(date(2030, 1, 3)));
        assert_eq!(status(date(2030, 1, 3)).label, "自定义休息");
        assert!(is_workday(date(2030, 1, 5)));
        assert_eq!(status(date(2030, 1, 5)).label, "自定义调休上班");
        set_overrides(Vec::new());
        assert!(is_workday(date(2030, 1, 3)));
        assert!(!is_workday(date(2030, 1, 5)));
    }
}
