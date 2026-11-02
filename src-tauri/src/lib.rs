mod calendar;
mod reminder;

use chrono::NaiveDate;
use reminder::{Reminder, ReminderInput};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

#[derive(Debug, Default, Serialize, Deserialize)]
struct StoreFile {
    #[serde(default)]
    reminders: Vec<Reminder>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum OverlayPosition {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    Center,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Default for OverlayPosition {
    fn default() -> Self {
        Self::BottomRight
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SettingsFile {
    #[serde(default)]
    overlay_position: OverlayPosition,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct OverridesFile {
    #[serde(default)]
    entries: Vec<calendar::CalendarOverride>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CalendarImportResult {
    entries: Vec<calendar::CalendarOverride>,
    holiday_count: usize,
    workday_count: usize,
}

#[derive(Debug, Default)]
struct OverlayState {
    queue: VecDeque<Reminder>,
    current: Option<Reminder>,
}

struct AppState {
    reminders: Mutex<Vec<Reminder>>,
    overlay: Mutex<OverlayState>,
    data_file: PathBuf,
    settings_file: PathBuf,
    overrides_file: PathBuf,
    overlay_position: Mutex<OverlayPosition>,
}

fn app_data_file() -> PathBuf {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
    };
    base.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("com.tixingmao.desktop")
        .join("reminders.json")
}

fn load_reminders(path: &Path) -> Result<Vec<Reminder>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw =
        std::fs::read_to_string(path).map_err(|error| format!("无法读取提醒数据：{error}"))?;
    let store: StoreFile =
        serde_json::from_str(&raw).map_err(|error| format!("提醒数据格式损坏：{error}"))?;
    Ok(store.reminders)
}

fn persist_reminders(path: &Path, reminders: &[Reminder]) -> Result<(), String> {
    let data = StoreFile {
        reminders: reminders.to_vec(),
    };
    let json = serde_json::to_string_pretty(&data)
        .map_err(|error| format!("无法序列化提醒数据：{error}"))?;
    std::fs::write(path, json).map_err(|error| format!("无法保存提醒数据：{error}"))
}

fn load_settings(path: &Path) -> Result<SettingsFile, String> {
    if !path.exists() {
        return Ok(SettingsFile::default());
    }
    let raw = std::fs::read_to_string(path).map_err(|error| format!("无法读取设置：{error}"))?;
    serde_json::from_str(&raw).map_err(|error| format!("设置数据格式损坏：{error}"))
}

fn persist_settings(path: &Path, settings: &SettingsFile) -> Result<(), String> {
    let json = serde_json::to_string_pretty(settings)
        .map_err(|error| format!("无法序列化设置：{error}"))?;
    std::fs::write(path, json).map_err(|error| format!("无法保存设置：{error}"))
}

fn load_calendar_overrides(
    path: &Path,
) -> Result<Vec<calendar::CalendarOverride>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("无法读取假期设置：{error}"))?;
    let store: OverridesFile = serde_json::from_str(&raw)
        .map_err(|error| format!("假期设置格式损坏：{error}"))?;
    Ok(store.entries)
}

fn persist_calendar_overrides(
    path: &Path,
    entries: &[calendar::CalendarOverride],
) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&OverridesFile {
        entries: entries.to_vec(),
    })
    .map_err(|error| format!("无法序列化假期设置：{error}"))?;
    std::fs::write(path, json).map_err(|error| format!("无法保存假期设置：{error}"))
}

fn notify_reminders_changed(app: &AppHandle) {
    let _ = app.emit_to("main", "reminders-changed", ());
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn hide_overlay(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.hide();
    }
}

fn position_overlay(window: &tauri::WebviewWindow, position: OverlayPosition) {
    let Ok(Some(monitor)) = window.primary_monitor() else {
        return;
    };
    let scale = monitor.scale_factor();
    let monitor_size = monitor.size();
    let monitor_position = monitor.position();
    let width = (440.0 * scale) as i32;
    let height = (360.0 * scale) as i32;
    let margin_x = (20.0 * scale) as i32;
    let margin_top = (24.0 * scale) as i32;
    let margin_bottom = (56.0 * scale) as i32;
    let left = monitor_position.x + margin_x;
    let right = monitor_position.x + monitor_size.width as i32 - width - margin_x;
    let top = monitor_position.y + margin_top;
    let bottom = monitor_position.y + monitor_size.height as i32 - height - margin_bottom;
    let center_x = monitor_position.x + (monitor_size.width as i32 - width) / 2;
    let center_y = monitor_position.y + (monitor_size.height as i32 - height) / 2;
    let (x, y) = match position {
        OverlayPosition::TopLeft => (left, top),
        OverlayPosition::TopCenter => (center_x, top),
        OverlayPosition::TopRight => (right, top),
        OverlayPosition::MiddleLeft => (left, center_y),
        OverlayPosition::Center => (center_x, center_y),
        OverlayPosition::MiddleRight => (right, center_y),
        OverlayPosition::BottomLeft => (left, bottom),
        OverlayPosition::BottomCenter => (center_x, bottom),
        OverlayPosition::BottomRight => (right, bottom),
    };
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}

fn show_next_overlay(app: &AppHandle) {
    let next = {
        let state = app.state::<AppState>();
        let Ok(mut overlay) = state.overlay.lock() else {
            return;
        };
        if overlay.current.is_some() {
            return;
        }
        let Some(item) = overlay.queue.pop_front() else {
            return;
        };
        overlay.current = Some(item.clone());
        item
    };

    if let Some(window) = app.get_webview_window("overlay") {
        let position = app
            .state::<AppState>()
            .overlay_position
            .lock()
            .map(|value| *value)
            .unwrap_or_default();
        position_overlay(&window, position);
        let _ = window.set_always_on_top(true);
        let _ = window.show();
        let _ = app.emit_to("overlay", "reminder-due", &next);
    }
}

fn enqueue_overlay(app: &AppHandle, state: &AppState, item: Reminder) {
    if let Ok(mut overlay) = state.overlay.lock() {
        overlay.queue.push_back(item);
    }
    show_next_overlay(app);
}

fn check_due(app: &AppHandle) -> Result<(), String> {
    let now = reminder::now_cn();
    let state = app.state::<AppState>();
    let due = {
        let mut reminders = state
            .reminders
            .lock()
            .map_err(|_| "提醒状态暂时不可用".to_string())?;
        let mut due = Vec::new();
        let mut changed = false;

        for item in reminders.iter_mut() {
            if !item.enabled {
                continue;
            }
            let next = reminder::parse_cn_datetime(&item.next_trigger_at)?;
            if next > now {
                continue;
            }

            if item.workday_only && !calendar::is_workday(now.date_naive()) {
                reminder::reschedule_skipped(item, now)?;
                changed = true;
                continue;
            }

            due.push(item.clone());
            reminder::advance_after_trigger(item, now)?;
            changed = true;
        }

        if changed {
            persist_reminders(&state.data_file, &reminders)?;
        }
        (due, changed)
    };
    let (due, changed) = due;

    if changed {
        notify_reminders_changed(app);
    }

    if due.is_empty() {
        return Ok(());
    }

    {
        let mut overlay = state
            .overlay
            .lock()
            .map_err(|_| "悬浮提醒状态暂时不可用".to_string())?;
        for item in due {
            overlay.queue.push_back(item);
        }
    }
    show_next_overlay(app);
    Ok(())
}

fn start_scheduler(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(error) = check_due(&app) {
                eprintln!("提醒调度失败：{error}");
            }
        }
    });
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(app, "open", "打开提醒列表", true, None::<&str>)?;
    let new_item = MenuItem::with_id(app, "new", "新建提醒", true, None::<&str>)?;
    let preview_item = MenuItem::with_id(app, "preview", "测试悬浮提醒", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出提醒猫", true, None::<&str>)?;
    let menu = Menu::new(app)?;
    menu.append(&open_item)?;
    menu.append(&new_item)?;
    menu.append(&preview_item)?;
    menu.append(&separator)?;
    menu.append(&quit_item)?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("提醒猫")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "new" => {
                show_main_window(app);
                let _ = app.emit_to("main", "request-new-reminder", ());
            }
            "preview" => {
                let state = app.state::<AppState>();
                let preview = reminder::preview_copy(None);
                enqueue_overlay(app, state.inner(), preview);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

#[tauri::command]
fn list_reminders(state: State<AppState>) -> Result<Vec<Reminder>, String> {
    state
        .reminders
        .lock()
        .map(|items| items.clone())
        .map_err(|_| "提醒状态暂时不可用".to_string())
}

#[tauri::command]
fn calendar_info() -> calendar::CalendarInfo {
    let today: NaiveDate = reminder::now_cn().date_naive();
    calendar::info(today)
}

#[tauri::command]
fn get_overlay_position(state: State<AppState>) -> Result<OverlayPosition, String> {
    state
        .overlay_position
        .lock()
        .map(|value| *value)
        .map_err(|_| "显示位置设置暂时不可用".to_string())
}

#[tauri::command]
fn set_overlay_position(
    position: OverlayPosition,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    persist_settings(
        &state.settings_file,
        &SettingsFile {
            overlay_position: position,
        },
    )?;
    {
        let mut current = state
            .overlay_position
            .lock()
            .map_err(|_| "显示位置设置暂时不可用".to_string())?;
        *current = position;
    }
    if let Some(window) = app.get_webview_window("overlay") {
        position_overlay(&window, position);
    }
    Ok(())
}

#[tauri::command]
fn save_reminder(input: ReminderInput, state: State<AppState>) -> Result<Reminder, String> {
    let now = reminder::now_cn();
    let mut reminders = state
        .reminders
        .lock()
        .map_err(|_| "提醒状态暂时不可用".to_string())?;

    if let Some(id) = input.id.as_deref() {
        let index = reminders.iter().position(|item| item.id == id);
        let Some(index) = index else {
            return Err("要编辑的提醒不存在".to_string());
        };
        let existing = reminders[index].clone();
        let updated = reminder::from_input(&input, Some(&existing), now)?;
        reminders[index] = updated.clone();
        persist_reminders(&state.data_file, &reminders)?;
        Ok(updated)
    } else {
        let created = reminder::from_input(&input, None, now)?;
        reminders.push(created.clone());
        persist_reminders(&state.data_file, &reminders)?;
        Ok(created)
    }
}

#[tauri::command]
fn set_reminder_enabled(
    id: String,
    enabled: bool,
    state: State<AppState>,
) -> Result<Reminder, String> {
    let now = reminder::now_cn();
    let mut reminders = state
        .reminders
        .lock()
        .map_err(|_| "提醒状态暂时不可用".to_string())?;
    let item = reminders
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or_else(|| "提醒不存在".to_string())?;

    if enabled && !item.enabled {
        reminder::refresh_if_past(item, now)?;
    }
    item.enabled = enabled;
    let updated = item.clone();
    persist_reminders(&state.data_file, &reminders)?;
    Ok(updated)
}

#[tauri::command]
fn reorder_reminders(
    ids: Vec<String>,
    state: State<AppState>,
) -> Result<Vec<Reminder>, String> {
    let mut reminders = state
        .reminders
        .lock()
        .map_err(|_| "提醒状态暂时不可用".to_string())?;
    let mut reordered: Vec<Reminder> = Vec::with_capacity(reminders.len());
    for id in ids {
        if let Some(index) = reminders.iter().position(|item| item.id == id) {
            reordered.push(reminders.remove(index));
        }
    }
    reordered.append(&mut reminders);
    persist_reminders(&state.data_file, &reordered)?;
    *reminders = reordered.clone();
    Ok(reordered)
}

#[tauri::command]
fn delete_reminder(id: String, app: AppHandle, state: State<AppState>) -> Result<(), String> {
    {
        let mut reminders = state
            .reminders
            .lock()
            .map_err(|_| "提醒状态暂时不可用".to_string())?;
        let before = reminders.len();
        reminders.retain(|item| item.id != id);
        if reminders.len() != before {
            persist_reminders(&state.data_file, &reminders)?;
        }
    }
    notify_reminders_changed(&app);

    let should_show_next = {
        let mut overlay = state
            .overlay
            .lock()
            .map_err(|_| "悬浮提醒状态暂时不可用".to_string())?;
        overlay.queue.retain(|item| item.id != id);
        if overlay.current.as_ref().map(|item| item.id.as_str()) == Some(id.as_str()) {
            overlay.current = None;
            let should_show_next = !overlay.queue.is_empty();
            drop(overlay);
            (true, should_show_next)
        } else {
            (false, false)
        }
    };

    if should_show_next.0 {
        if should_show_next.1 {
            show_next_overlay(&app);
        } else {
            hide_overlay(&app);
        }
    }
    Ok(())
}

#[tauri::command]
fn preview_reminder(
    id: Option<String>,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    let source = if let Some(id) = id.as_deref() {
        let reminders = state
            .reminders
            .lock()
            .map_err(|_| "提醒状态暂时不可用".to_string())?;
        Some(
            reminders
                .iter()
                .find(|item| item.id == id)
                .cloned()
                .ok_or_else(|| "提醒不存在".to_string())?,
        )
    } else {
        None
    };
    let preview = reminder::preview_copy(source.as_ref());
    enqueue_overlay(&app, state.inner(), preview);
    Ok(())
}

#[tauri::command]
fn get_active_overlay(state: State<AppState>) -> Result<Option<Reminder>, String> {
    let overlay = state
        .overlay
        .lock()
        .map_err(|_| "悬浮提醒状态暂时不可用".to_string())?;
    Ok(overlay.current.clone())
}

#[tauri::command]
fn dismiss_overlay(id: String, app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let (should_show_next, current) = {
        let mut overlay = state
            .overlay
            .lock()
            .map_err(|_| "悬浮提醒状态暂时不可用".to_string())?;
        let Some(current) = overlay.current.as_ref() else {
            return Ok(());
        };
        if current.id != id {
            return Ok(());
        }
        let current = overlay.current.take().expect("checked current");
        let should_show_next = !overlay.queue.is_empty();
        (should_show_next, current)
    };

    if current.auto_delete && !current.id.starts_with("preview-") {
        let mut reminders = state
            .reminders
            .lock()
            .map_err(|_| "提醒状态暂时不可用".to_string())?;
        let before = reminders.len();
        reminders.retain(|item| item.id != current.id);
        if reminders.len() != before {
            persist_reminders(&state.data_file, &reminders)?;
        }
    }
    notify_reminders_changed(&app);

    if should_show_next {
        show_next_overlay(&app);
    } else {
        hide_overlay(&app);
    }
    Ok(())
}

#[tauri::command]
fn list_calendar_overrides() -> Vec<calendar::CalendarOverride> {
    calendar::list_overrides()
}

#[tauri::command]
fn calendar_day(date: String) -> Result<calendar::DayStatus, String> {
    calendar::day_status(&date)
}

fn refresh_reminders_for_calendar(state: &AppState) -> Result<(), String> {
    let now = reminder::now_cn();
    let mut reminders = state
        .reminders
        .lock()
        .map_err(|_| "提醒状态暂时不可用".to_string())?;
    let mut changed = false;
    for item in reminders.iter_mut() {
        if reminder::refresh_for_calendar(item, now)? {
            changed = true;
        }
    }
    if changed {
        persist_reminders(&state.data_file, &reminders)?;
    }
    Ok(())
}

fn persist_and_apply_overrides(
    entries: Vec<calendar::CalendarOverride>,
    state: &AppState,
) -> Result<Vec<calendar::CalendarOverride>, String> {
    persist_calendar_overrides(&state.overrides_file, &entries)?;
    calendar::set_overrides(entries.clone());
    refresh_reminders_for_calendar(state)?;
    Ok(entries)
}

#[tauri::command]
fn save_calendar_override(
    date: String,
    kind: calendar::OverrideKind,
    state: State<AppState>,
) -> Result<Vec<calendar::CalendarOverride>, String> {
    let date = calendar::normalize_date(&date)?;
    let mut entries = calendar::list_overrides();
    entries.retain(|entry| entry.date != date);
    entries.push(calendar::CalendarOverride { date, kind });
    entries.sort_by(|left, right| left.date.cmp(&right.date));
    persist_and_apply_overrides(entries, state.inner())
}

#[tauri::command]
fn delete_calendar_override(
    date: String,
    state: State<AppState>,
) -> Result<Vec<calendar::CalendarOverride>, String> {
    let date = calendar::normalize_date(&date)?;
    let mut entries = calendar::list_overrides();
    entries.retain(|entry| entry.date != date);
    persist_and_apply_overrides(entries, state.inner())
}

#[tauri::command]
fn import_calendar_overrides(
    holidays: String,
    workdays: String,
    state: State<AppState>,
) -> Result<CalendarImportResult, String> {
    let holiday_dates = calendar::parse_date_lines(&holidays)?;
    let workday_dates = calendar::parse_date_lines(&workdays)?;
    if holiday_dates.is_empty() && workday_dates.is_empty() {
        return Err("两栏都是空的，请先粘贴日期".to_string());
    }
    for date in &holiday_dates {
        if workday_dates.contains(date) {
            return Err(format!("{date} 同时出现在放假和调休里，请检查一下"));
        }
    }

    let mut entries = calendar::list_overrides();
    entries.retain(|entry| {
        !holiday_dates.contains(&entry.date) && !workday_dates.contains(&entry.date)
    });
    let holiday_count = holiday_dates.len();
    let workday_count = workday_dates.len();
    for date in holiday_dates {
        entries.push(calendar::CalendarOverride {
            date,
            kind: calendar::OverrideKind::Holiday,
        });
    }
    for date in workday_dates {
        entries.push(calendar::CalendarOverride {
            date,
            kind: calendar::OverrideKind::Workday,
        });
    }
    entries.sort_by(|left, right| left.date.cmp(&right.date));
    let entries = persist_and_apply_overrides(entries, state.inner())?;
    Ok(CalendarImportResult {
        entries,
        holiday_count,
        workday_count,
    })
}

pub fn run() {
    let data_file = app_data_file();
    if let Some(parent) = data_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let reminders = load_reminders(&data_file).unwrap_or_else(|error| {
        eprintln!("无法读取提醒数据：{error}");
        let backup = data_file.with_extension("json.broken");
        let _ = std::fs::rename(&data_file, backup);
        Vec::new()
    });
    let settings_file = data_file.with_file_name("settings.json");
    let settings = load_settings(&settings_file).unwrap_or_else(|error| {
        eprintln!("无法读取设置：{error}");
        let backup = settings_file.with_extension("json.broken");
        let _ = std::fs::rename(&settings_file, backup);
        SettingsFile::default()
    });
    let overrides_file = data_file.with_file_name("calendar-overrides.json");
    let overrides = load_calendar_overrides(&overrides_file).unwrap_or_else(|error| {
        eprintln!("无法读取假期设置：{error}");
        let backup = overrides_file.with_extension("json.broken");
        let _ = std::fs::rename(&overrides_file, backup);
        Vec::new()
    });
    calendar::set_overrides(overrides);

    tauri::Builder::default()
        .manage(AppState {
            reminders: Mutex::new(reminders),
            overlay: Mutex::new(OverlayState::default()),
            data_file,
            settings_file,
            overrides_file,
            overlay_position: Mutex::new(settings.overlay_position),
        })
        .setup(|app| {
            setup_tray(app.handle())?;

            if let Some(main_window) = app.get_webview_window("main") {
                let window_to_hide = main_window.clone();
                main_window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_to_hide.hide();
                    }
                });
            }

            start_scheduler(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_reminders,
            calendar_info,
            list_calendar_overrides,
            calendar_day,
            save_calendar_override,
            delete_calendar_override,
            import_calendar_overrides,
            get_overlay_position,
            set_overlay_position,
            save_reminder,
            set_reminder_enabled,
            reorder_reminders,
            delete_reminder,
            preview_reminder,
            get_active_overlay,
            dismiss_overlay
        ])
        .run(tauri::generate_context!())
        .expect("error while running 提醒猫");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reminders_keep_user_order_on_save() {
        let dir = std::env::temp_dir().join(format!(
            "tixingmao-order-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir should be writable");
        let path = dir.join("reminders.json");
        let make = |id: &str, title: &str| Reminder {
            id: id.to_string(),
            title: title.to_string(),
            message: String::new(),
            enabled: true,
            repeat: reminder::RepeatKind::Daily,
            scheduled_at: None,
            time_of_day: Some("09:00".to_string()),
            interval_minutes: None,
            workday_only: false,
            next_trigger_at: "2026-10-27T09:00:00+08:00".to_string(),
            last_triggered_at: None,
            created_at: "2026-10-26T09:00:00+08:00".to_string(),
            auto_delete: false,
        };
        let items = vec![make("b", "第二个"), make("a", "第一个")];
        persist_reminders(&path, &items).expect("reminders should be saved");
        let loaded = load_reminders(&path).expect("reminders should be loaded");
        assert_eq!(
            loaded.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
            vec!["b", "a"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn calendar_override_file_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "tixingmao-overrides-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir should be writable");
        let path = dir.join("calendar-overrides.json");
        let entries = vec![
            calendar::CalendarOverride {
                date: "2027-01-01".to_string(),
                kind: calendar::OverrideKind::Holiday,
            },
            calendar::CalendarOverride {
                date: "2027-01-09".to_string(),
                kind: calendar::OverrideKind::Workday,
            },
        ];
        persist_calendar_overrides(&path, &entries).expect("overrides should be saved");
        let loaded = load_calendar_overrides(&path).expect("overrides should be loaded");
        assert_eq!(loaded, entries);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
