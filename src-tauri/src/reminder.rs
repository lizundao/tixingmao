use crate::calendar;
use chrono::{DateTime, Duration, FixedOffset, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RepeatKind {
    Once,
    Daily,
    Interval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: String,
    pub title: String,
    pub message: String,
    pub enabled: bool,
    pub repeat: RepeatKind,
    pub scheduled_at: Option<String>,
    pub time_of_day: Option<String>,
    pub interval_minutes: Option<i64>,
    pub workday_only: bool,
    pub next_trigger_at: String,
    pub last_triggered_at: Option<String>,
    pub created_at: String,
    pub auto_delete: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderInput {
    pub id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub message: String,
    pub repeat: RepeatKind,
    pub scheduled_at: Option<String>,
    pub time_of_day: Option<String>,
    pub interval_minutes: Option<i64>,
    #[serde(default = "default_true")]
    pub workday_only: bool,
    #[serde(default)]
    pub auto_delete: bool,
}

fn default_true() -> bool {
    true
}

fn cn_offset() -> FixedOffset {
    FixedOffset::east_opt(8 * 60 * 60).expect("UTC+8 offset is valid")
}

pub fn now_cn() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&cn_offset())
}

pub fn parse_cn_datetime(value: &str) -> Result<DateTime<FixedOffset>, String> {
    let parsed =
        DateTime::parse_from_rfc3339(value).map_err(|error| format!("时间格式不正确：{error}"))?;
    Ok(parsed.with_timezone(&cn_offset()))
}

fn parse_time(value: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(value, "%H:%M:%S"))
        .map_err(|_| "请选择有效的提醒时间".to_string())
}

fn local_datetime(date: NaiveDate, time: NaiveTime) -> DateTime<FixedOffset> {
    let naive = date.and_time(time);
    match cn_offset().from_local_datetime(&naive) {
        LocalResult::Single(value) => value,
        _ => DateTime::from_naive_utc_and_offset(naive, cn_offset()),
    }
}

fn move_to_workday(candidate: DateTime<FixedOffset>, workday_only: bool) -> DateTime<FixedOffset> {
    if !workday_only || calendar::is_workday(candidate.date_naive()) {
        return candidate;
    }

    let time = candidate.time();
    calendar::next_workday(candidate.date_naive())
        .map(|date| local_datetime(date, time))
        .unwrap_or(candidate)
}

pub fn next_daily_after(
    after: DateTime<FixedOffset>,
    time: NaiveTime,
    workday_only: bool,
) -> DateTime<FixedOffset> {
    let mut date = after.date_naive();
    let mut candidate = local_datetime(date, time);
    if candidate <= after {
        date = date
            .checked_add_signed(Duration::days(1))
            .expect("date range is valid");
        candidate = local_datetime(date, time);
    }
    move_to_workday(candidate, workday_only)
}

pub fn next_interval_after(
    base: DateTime<FixedOffset>,
    interval_minutes: i64,
    workday_only: bool,
) -> DateTime<FixedOffset> {
    let candidate = base + Duration::minutes(interval_minutes.max(1));
    move_to_workday(candidate, workday_only)
}

fn normalized_once_time(reminder: &Reminder) -> Result<NaiveTime, String> {
    let value = reminder
        .scheduled_at
        .as_deref()
        .ok_or_else(|| "一次性提醒缺少时间".to_string())?;
    Ok(parse_cn_datetime(value)?.time())
}

pub fn reschedule_skipped(
    reminder: &mut Reminder,
    now: DateTime<FixedOffset>,
) -> Result<(), String> {
    reminder.next_trigger_at = match reminder.repeat {
        RepeatKind::Once => {
            let time = normalized_once_time(reminder)?;
            next_daily_after(now, time, true).to_rfc3339()
        }
        RepeatKind::Daily => {
            let time = parse_time(reminder.time_of_day.as_deref().unwrap_or("09:00"))?;
            next_daily_after(now, time, true).to_rfc3339()
        }
        RepeatKind::Interval => {
            let minutes = reminder.interval_minutes.unwrap_or(30).max(1);
            next_interval_after(now, minutes, true).to_rfc3339()
        }
    };
    Ok(())
}

pub fn advance_after_trigger(
    reminder: &mut Reminder,
    now: DateTime<FixedOffset>,
) -> Result<(), String> {
    reminder.last_triggered_at = Some(now.to_rfc3339());
    match reminder.repeat {
        RepeatKind::Once => {
            reminder.enabled = false;
        }
        RepeatKind::Daily => {
            let time = parse_time(reminder.time_of_day.as_deref().unwrap_or("09:00"))?;
            reminder.next_trigger_at =
                next_daily_after(now, time, reminder.workday_only).to_rfc3339();
        }
        RepeatKind::Interval => {
            let minutes = reminder.interval_minutes.unwrap_or(30).max(1);
            reminder.next_trigger_at =
                next_interval_after(now, minutes, reminder.workday_only).to_rfc3339();
        }
    }
    Ok(())
}

pub fn refresh_if_past(reminder: &mut Reminder, now: DateTime<FixedOffset>) -> Result<(), String> {
    let next = parse_cn_datetime(&reminder.next_trigger_at)?;
    if next > now {
        return Ok(());
    }

    match reminder.repeat {
        RepeatKind::Once => Err("这条一次性提醒的时间已经过去，请先修改时间再启用".to_string()),
        RepeatKind::Daily => {
            let time = parse_time(reminder.time_of_day.as_deref().unwrap_or("09:00"))?;
            reminder.next_trigger_at =
                next_daily_after(now, time, reminder.workday_only).to_rfc3339();
            Ok(())
        }
        RepeatKind::Interval => {
            let minutes = reminder.interval_minutes.unwrap_or(30).max(1);
            reminder.next_trigger_at =
                next_interval_after(now, minutes, reminder.workday_only).to_rfc3339();
            Ok(())
        }
    }
}

pub fn refresh_for_calendar(
    reminder: &mut Reminder,
    now: DateTime<FixedOffset>,
) -> Result<bool, String> {
    if !reminder.enabled || !reminder.workday_only {
        return Ok(false);
    }
    let previous = reminder.next_trigger_at.clone();
    match reminder.repeat {
        RepeatKind::Once => {
            if let Some(raw) = reminder.scheduled_at.as_deref() {
                let scheduled = parse_cn_datetime(raw)?;
                if scheduled > now {
                    reminder.next_trigger_at = move_to_workday(scheduled, true).to_rfc3339();
                }
            }
        }
        RepeatKind::Daily => {
            let time = parse_time(reminder.time_of_day.as_deref().unwrap_or("09:00"))?;
            reminder.next_trigger_at = next_daily_after(now, time, true).to_rfc3339();
        }
        RepeatKind::Interval => {
            let minutes = reminder.interval_minutes.unwrap_or(30).max(1);
            let next = parse_cn_datetime(&reminder.next_trigger_at).unwrap_or(now);
            reminder.next_trigger_at = if next > now {
                move_to_workday(next, true).to_rfc3339()
            } else {
                next_interval_after(now, minutes, true).to_rfc3339()
            };
        }
    }
    Ok(previous != reminder.next_trigger_at)
}

fn validate_and_normalize(
    input: &ReminderInput,
) -> Result<(String, Option<String>, Option<String>, Option<i64>), String> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err("提醒名称不能为空".to_string());
    }
    if title.chars().count() > 80 {
        return Err("提醒名称最多 80 个字".to_string());
    }
    let message = input.message.trim().to_string();
    if message.chars().count() > 500 {
        return Err("提醒内容最多 500 个字".to_string());
    }

    match input.repeat {
        RepeatKind::Once => {
            let raw = input
                .scheduled_at
                .as_deref()
                .ok_or_else(|| "请选择一次性提醒的时间".to_string())?;
            let scheduled = parse_cn_datetime(raw)?;
            Ok((title, Some(scheduled.to_rfc3339()), None, None))
        }
        RepeatKind::Daily => {
            let raw = input
                .time_of_day
                .as_deref()
                .ok_or_else(|| "请选择每天的提醒时间".to_string())?;
            let time = parse_time(raw)?;
            Ok((title, None, Some(time.format("%H:%M").to_string()), None))
        }
        RepeatKind::Interval => {
            let minutes = input.interval_minutes.unwrap_or(0);
            if !(1..=525_600).contains(&minutes) {
                return Err("提醒间隔必须在 1 分钟到 365 天之间".to_string());
            }
            Ok((title, None, None, Some(minutes)))
        }
    }
}

fn initial_next(
    repeat: &RepeatKind,
    scheduled_at: Option<&str>,
    time_of_day: Option<&str>,
    interval_minutes: Option<i64>,
    workday_only: bool,
    now: DateTime<FixedOffset>,
) -> Result<String, String> {
    match repeat {
        RepeatKind::Once => {
            let raw = scheduled_at.ok_or_else(|| "一次性提醒缺少时间".to_string())?;
            let scheduled = parse_cn_datetime(raw)?;
            if scheduled <= now {
                return Err("提醒时间已经过去，请重新选择".to_string());
            }
            Ok(move_to_workday(scheduled, workday_only).to_rfc3339())
        }
        RepeatKind::Daily => {
            let time = parse_time(time_of_day.unwrap_or("09:00"))?;
            Ok(next_daily_after(now, time, workday_only).to_rfc3339())
        }
        RepeatKind::Interval => {
            let minutes = interval_minutes.unwrap_or(30).max(1);
            Ok(next_interval_after(now, minutes, workday_only).to_rfc3339())
        }
    }
}

pub fn from_input(
    input: &ReminderInput,
    existing: Option<&Reminder>,
    now: DateTime<FixedOffset>,
) -> Result<Reminder, String> {
    let (title, scheduled_at, time_of_day, interval_minutes) = validate_and_normalize(input)?;
    let schedule_changed = match existing {
        None => true,
        Some(old) => {
            old.repeat != input.repeat
                || old.scheduled_at != scheduled_at
                || old.time_of_day != time_of_day
                || old.interval_minutes != interval_minutes
                || old.workday_only != input.workday_only
        }
    };

    let next_trigger_at = if schedule_changed {
        initial_next(
            &input.repeat,
            scheduled_at.as_deref(),
            time_of_day.as_deref(),
            interval_minutes,
            input.workday_only,
            now,
        )?
    } else {
        existing
            .map(|item| item.next_trigger_at.clone())
            .ok_or_else(|| "提醒不存在".to_string())?
    };

    Ok(Reminder {
        id: existing
            .map(|item| item.id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string()),
        title,
        message: input.message.trim().to_string(),
        enabled: existing.map(|item| item.enabled).unwrap_or(true),
        repeat: input.repeat.clone(),
        scheduled_at,
        time_of_day,
        interval_minutes,
        workday_only: input.workday_only,
        next_trigger_at,
        last_triggered_at: existing.and_then(|item| item.last_triggered_at.clone()),
        created_at: existing
            .map(|item| item.created_at.clone())
            .unwrap_or_else(|| now.to_rfc3339()),
        auto_delete: input.repeat == RepeatKind::Once && input.auto_delete,
    })
}

pub fn preview_copy(source: Option<&Reminder>) -> Reminder {
    let now = now_cn();
    match source {
        Some(item) => Reminder {
            id: format!("preview-{}", Uuid::new_v4()),
            title: item.title.clone(),
            message: item.message.clone(),
            enabled: true,
            repeat: RepeatKind::Once,
            scheduled_at: None,
            time_of_day: None,
            interval_minutes: None,
            workday_only: false,
            next_trigger_at: now.to_rfc3339(),
            last_triggered_at: None,
            created_at: now.to_rfc3339(),
            auto_delete: false,
        },
        None => Reminder {
            id: format!("preview-{}", Uuid::new_v4()),
            title: "该喝水啦".to_string(),
            message: "已经坐了好一会儿，起来伸个懒腰，顺便喝口水吧。".to_string(),
            enabled: true,
            repeat: RepeatKind::Once,
            scheduled_at: None,
            time_of_day: None,
            interval_minutes: None,
            workday_only: false,
            next_trigger_at: now.to_rfc3339(),
            last_triggered_at: None,
            created_at: now.to_rfc3339(),
            auto_delete: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<FixedOffset> {
        local_datetime(
            NaiveDate::from_ymd_opt(year, month, day).unwrap(),
            NaiveTime::from_hms_opt(hour, minute, 0).unwrap(),
        )
    }

    #[test]
    fn daily_reminder_respects_makeup_workday() {
        let next = next_daily_after(
            at(2026, 10, 9, 10, 0),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            true,
        );
        assert_eq!(next, at(2026, 10, 10, 9, 0));
    }

    #[test]
    fn daily_reminder_skips_holiday_weekdays() {
        let next = next_daily_after(
            at(2026, 9, 30, 10, 0),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            true,
        );
        assert_eq!(next, at(2026, 10, 8, 9, 0));
    }

    #[test]
    fn once_reminder_moves_to_next_workday() {
        let next = move_to_workday(at(2026, 10, 1, 9, 0), true);
        assert_eq!(next, at(2026, 10, 8, 9, 0));
    }
}
