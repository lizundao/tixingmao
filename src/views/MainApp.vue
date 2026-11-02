<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  CalendarImportResult,
  CalendarInfo,
  CalendarOverride,
  DayOverrideKind,
  DayStatus,
  OverlayPosition,
  Reminder,
  ReminderDraft,
  RepeatKind
} from "../types";

const reminders = ref<Reminder[]>([]);

const calendar = ref<CalendarInfo | null>(null);
const loading = ref(true);
const saving = ref(false);
const error = ref("");
const success = ref("");
const editingId = ref<string | null>(null);
const deletingId = ref<string | null>(null);
const overlayPosition = ref<OverlayPosition>("bottomRight");
const editorOpen = ref(false);
const settingsOpen = ref(false);
const pendingDelete = ref<Reminder | null>(null);
const formError = ref("");
const titleInput = ref<HTMLInputElement | null>(null);
const calendarOverrides = ref<CalendarOverride[]>([]);
const overrideDate = ref(todayValue());
const overrideKind = ref<DayOverrideKind>("holiday");
const overridePreview = ref<DayStatus | null>(null);
const overrideError = ref("");
const overrideNotice = ref("");
const savingOverride = ref(false);
const deletingOverride = ref<string | null>(null);
const importOpen = ref(false);
const dragHandleId = ref<string | null>(null);
const draggingId = ref<string | null>(null);
const reminderListEl = ref<HTMLElement | null>(null);
const importHolidays = ref("");
const importWorkdays = ref("");
const importError = ref("");
const importing = ref(false);
const tick = ref(Date.now());

const positionOptions: Array<{ value: OverlayPosition; label: string }> = [
  { value: "topLeft", label: "左上" },
  { value: "topCenter", label: "上中" },
  { value: "topRight", label: "右上" },
  { value: "middleLeft", label: "左中" },
  { value: "center", label: "居中" },
  { value: "middleRight", label: "右中" },
  { value: "bottomLeft", label: "左下" },
  { value: "bottomCenter", label: "下中" },
  { value: "bottomRight", label: "右下" }
];

const currentPositionLabel = computed(
  () => positionOptions.find((option) => option.value === overlayPosition.value)?.label ?? "右下"
);

const modalOpen = computed(
  () =>
    editorOpen.value ||
    settingsOpen.value ||
    importOpen.value ||
    pendingDelete.value !== null
);

watch(modalOpen, (open) => {
  document.body.classList.toggle("modal-open", open);
});

watch(overrideDate, () => {
  void refreshOverridePreview();
});

let unlisten: UnlistenFn | undefined;
let unlistenChanged: UnlistenFn | undefined;
let clockTimer: number | undefined;
let noticeTimer: number | undefined;

function pad(value: number) {
  return String(value).padStart(2, "0");
}

function toLocalInput(iso: string | null) {
  if (!iso) return defaultOnceValue();
  const date = new Date(iso);
  return (
    date.getFullYear() +
    "-" +
    pad(date.getMonth() + 1) +
    "-" +
    pad(date.getDate()) +
    "T" +
    pad(date.getHours()) +
    ":" +
    pad(date.getMinutes())
  );
}

function dateInputValue(date: Date) {
  return date.getFullYear() + "-" + pad(date.getMonth() + 1) + "-" + pad(date.getDate());
}

function todayValue() {
  return dateInputValue(new Date());
}

function defaultOnceValue() {
  const date = new Date(Date.now() + 60 * 60 * 1000);
  return (
    date.getFullYear() +
    "-" +
    pad(date.getMonth() + 1) +
    "-" +
    pad(date.getDate()) +
    "T" +
    pad(date.getHours()) +
    ":" +
    pad(date.getMinutes())
  );
}

const form = reactive({
  title: "",
  message: "",
  repeat: "daily" as RepeatKind,
  onceAt: defaultOnceValue(),
  dailyTime: "09:00",
  intervalValue: 30,
  intervalUnit: "minutes" as "minutes" | "hours" | "days",
  workdayOnly: true,
  autoDelete: true
});

const activeCount = computed(() => reminders.value.filter((item) => item.enabled).length);
const displayReminders = computed(() => reminders.value);

const todayPillClass = computed(() => {
  if (!calendar.value) return "";
  return calendar.value.today.kind;
});

function showSuccess(message: string) {
  success.value = message;
  window.clearTimeout(noticeTimer);
  noticeTimer = window.setTimeout(() => {
    success.value = "";
  }, 2800);
}

async function load() {
  error.value = "";
  try {
    const [items, info, position, overrides] = await Promise.all([
      invoke<Reminder[]>("list_reminders"),
      invoke<CalendarInfo>("calendar_info"),
      invoke<OverlayPosition>("get_overlay_position"),
      invoke<CalendarOverride[]>("list_calendar_overrides")
    ]);
    reminders.value = items;
    calendar.value = info;
    overlayPosition.value = position;
    calendarOverrides.value = overrides;
    void refreshOverridePreview();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

function resetForm() {
  editingId.value = null;
  form.title = "";
  form.message = "";
  form.repeat = "daily";
  form.onceAt = defaultOnceValue();
  form.dailyTime = "09:00";
  form.intervalValue = 30;
  form.intervalUnit = "minutes";
  form.workdayOnly = true;
  form.autoDelete = true;
}

function editReminder(reminder: Reminder) {
  editingId.value = reminder.id;
  form.title = reminder.title;
  form.message = reminder.message;
  form.repeat = reminder.repeat;
  form.onceAt = toLocalInput(reminder.scheduledAt);
  form.dailyTime = reminder.timeOfDay ?? "09:00";
  const minutes = reminder.intervalMinutes ?? 30;
  if (minutes % 1440 === 0) {
    form.intervalValue = minutes / 1440;
    form.intervalUnit = "days";
  } else if (minutes % 60 === 0) {
    form.intervalValue = minutes / 60;
    form.intervalUnit = "hours";
  } else {
    form.intervalValue = minutes;
    form.intervalUnit = "minutes";
  }
  form.workdayOnly = reminder.workdayOnly;
  form.autoDelete = reminder.autoDelete;
}

function openEditor(reminder: Reminder | null = null) {
  error.value = "";
  formError.value = "";
  if (reminder) {
    editReminder(reminder);
  } else {
    resetForm();
  }
  editorOpen.value = true;
  nextTick(() => titleInput.value?.focus());
}

function closeEditor() {
  if (saving.value) return;
  editorOpen.value = false;
  formError.value = "";
  resetForm();
}

async function save() {
  formError.value = "";
  if (!form.title.trim()) {
    formError.value = "先给这个提醒起个名字吧";
    nextTick(() => titleInput.value?.focus());
    return;
  }

  saving.value = true;
  try {
    const unitMultiplier = form.intervalUnit === "days" ? 1440 : form.intervalUnit === "hours" ? 60 : 1;
    const draft: ReminderDraft = {
      id: editingId.value ?? undefined,
      title: form.title.trim(),
      message: form.message.trim(),
      repeat: form.repeat,
      scheduledAt: form.repeat === "once" ? new Date(form.onceAt).toISOString() : null,
      timeOfDay: form.repeat === "daily" ? form.dailyTime : null,
      intervalMinutes:
        form.repeat === "interval" ? Math.max(1, Math.round(form.intervalValue * unitMultiplier)) : null,
      workdayOnly: form.workdayOnly,
      autoDelete: form.autoDelete
    };
    const wasEditing = Boolean(editingId.value);
    await invoke<Reminder>("save_reminder", { input: draft });
    await load();
    resetForm();
    editorOpen.value = false;
    showSuccess(wasEditing ? "提醒已经更新" : "提醒猫记住了");
  } catch (cause) {
    formError.value = String(cause);
  } finally {
    saving.value = false;
  }
}

async function toggleReminder(reminder: Reminder) {
  error.value = "";
  try {
    await invoke("set_reminder_enabled", {
      id: reminder.id,
      enabled: !reminder.enabled
    });
    await load();
  } catch (cause) {
    error.value = String(cause);
  }
}

async function removeReminder(reminder: Reminder) {
  if (deletingId.value) return;
  error.value = "";
  deletingId.value = reminder.id;
  const previous = reminders.value;
  reminders.value = previous.filter((item) => item.id !== reminder.id);
  if (editingId.value === reminder.id) resetForm();
  try {
    await invoke("delete_reminder", { id: reminder.id });
    await load();
    showSuccess("提醒已删除");
  } catch (cause) {
    reminders.value = previous;
    error.value = String(cause);
  } finally {
    deletingId.value = null;
  }
}

let dragOrigin: { x: number; y: number } | null = null;
let dragSnapshot: Reminder[] | null = null;

function detachDragListeners() {
  window.removeEventListener("pointermove", onPointerMove);
  window.removeEventListener("pointerup", onPointerUp);
  window.removeEventListener("pointercancel", onPointerUp);
  document.body.classList.remove("is-dragging-card");
}

function onHandlePointerDown(reminder: Reminder, event: PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  dragHandleId.value = reminder.id;
  draggingId.value = null;
  dragOrigin = { x: event.clientX, y: event.clientY };
  dragSnapshot = reminders.value;
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp);
  window.addEventListener("pointercancel", onPointerUp);
}

function onPointerMove(event: PointerEvent) {
  const handleId = dragHandleId.value;
  if (!handleId) return;
  event.preventDefault();

  if (!draggingId.value) {
    if (!dragOrigin) return;
    const moved = Math.abs(event.clientX - dragOrigin.x) + Math.abs(event.clientY - dragOrigin.y);
    if (moved < 4) return;
    draggingId.value = handleId;
    document.body.classList.add("is-dragging-card");
  }

  reorderTo(insertIndexAt(event.clientX, event.clientY));
}

function insertIndexAt(clientX: number, clientY: number) {
  const container = reminderListEl.value;
  if (!container) return 0;
  const cards = Array.from(
    container.querySelectorAll<HTMLElement>("[data-reminder-id]")
  ).filter((element) => element.dataset.reminderId !== draggingId.value);

  let index = 0;
  for (const card of cards) {
    const rect = card.getBoundingClientRect();
    const centerY = rect.top + rect.height / 2;
    const centerX = rect.left + rect.width / 2;
    const isBefore =
      centerY < clientY - Math.min(rect.height / 2, 24) ||
      (Math.abs(centerY - clientY) <= rect.height / 2 && centerX < clientX);
    if (!isBefore) break;
    index += 1;
  }
  return index;
}

function reorderTo(index: number) {
  const draggedId = draggingId.value;
  if (!draggedId) return;
  const dragged = reminders.value.find((item) => item.id === draggedId);
  if (!dragged) return;
  const rest = reminders.value.filter((item) => item.id !== draggedId);
  const target = Math.max(0, Math.min(index, rest.length));
  const next = [...rest];
  next.splice(target, 0, dragged);
  if (next.every((item, position) => item.id === reminders.value[position]?.id)) return;
  reminders.value = next;
}

async function onPointerUp() {
  detachDragListeners();
  const wasDragging = draggingId.value !== null;
  const snapshot = dragSnapshot;
  const ids = reminders.value.map((item) => item.id);
  dragHandleId.value = null;
  draggingId.value = null;
  dragOrigin = null;
  dragSnapshot = null;
  if (!wasDragging) return;

  try {
    reminders.value = await invoke<Reminder[]>("reorder_reminders", { ids });
    showSuccess("顺序已保存");
  } catch (cause) {
    if (snapshot) reminders.value = snapshot;
    error.value = String(cause);
  }
}

async function previewReminder(id: string | null = null) {
  error.value = "";
  try {
    await invoke("preview_reminder", { id });
  } catch (cause) {
    error.value = String(cause);
  }
}

async function changeOverlayPosition(position: OverlayPosition) {
  if (position === overlayPosition.value) return;
  const previous = overlayPosition.value;
  overlayPosition.value = position;
  error.value = "";
  try {
    await invoke("set_overlay_position", { position });
    showSuccess("猫咪出现位置已更新");
  } catch (cause) {
    overlayPosition.value = previous;
    error.value = String(cause);
  }
}

async function refreshOverridePreview() {
  if (!overrideDate.value) {
    overridePreview.value = null;
    return;
  }
  try {
    overridePreview.value = await invoke<DayStatus>("calendar_day", { date: overrideDate.value });
  } catch {
    overridePreview.value = null;
  }
}

function overrideDateLabel(value: string) {
  const date = new Date(value + "T00:00:00");
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "long",
    day: "numeric",
    weekday: "short"
  }).format(date);
}

async function saveOverride() {
  overrideError.value = "";
  if (!overrideDate.value) {
    overrideError.value = "先选一个日期";
    return;
  }
  savingOverride.value = true;
  try {
    calendarOverrides.value = await invoke<CalendarOverride[]>("save_calendar_override", {
      date: overrideDate.value,
      kind: overrideKind.value
    });
    const info = await invoke<CalendarInfo>("calendar_info");
    calendar.value = info;
    await load();
    await refreshOverridePreview();
    overrideNotice.value = overrideKind.value === "holiday" ? "已标记为休息日" : "已标记为调休上班日";
    showSuccess(overrideNotice.value);
  } catch (cause) {
    overrideError.value = String(cause);
  } finally {
    savingOverride.value = false;
  }
}

function openImport() {
  importError.value = "";
  importHolidays.value = "";
  importWorkdays.value = "";
  importOpen.value = true;
}

function closeImport() {
  if (importing.value) return;
  importOpen.value = false;
  importError.value = "";
}

async function submitImport() {
  importError.value = "";
  importing.value = true;
  try {
    const result = await invoke<CalendarImportResult>("import_calendar_overrides", {
      holidays: importHolidays.value,
      workdays: importWorkdays.value
    });
    calendarOverrides.value = result.entries;
    importOpen.value = false;
    const info = await invoke<CalendarInfo>("calendar_info");
    calendar.value = info;
    await load();
    await refreshOverridePreview();
    overrideNotice.value =
      "已导入 " + result.holidayCount + " 个休息日、" + result.workdayCount + " 个调休上班日";
    showSuccess(overrideNotice.value);
  } catch (cause) {
    importError.value = String(cause);
  } finally {
    importing.value = false;
  }
}

async function removeOverride(date: string) {
  if (deletingOverride.value) return;
  overrideError.value = "";
  deletingOverride.value = date;
  const previous = calendarOverrides.value;
  calendarOverrides.value = previous.filter((entry) => entry.date !== date);
  try {
    calendarOverrides.value = await invoke<CalendarOverride[]>("delete_calendar_override", { date });
    const info = await invoke<CalendarInfo>("calendar_info");
    calendar.value = info;
    await load();
    await refreshOverridePreview();
    overrideNotice.value = "已恢复默认判断";
    showSuccess(overrideNotice.value);
  } catch (cause) {
    calendarOverrides.value = previous;
    overrideError.value = String(cause);
  } finally {
    deletingOverride.value = null;
  }
}

function requestDelete(reminder: Reminder) {
  pendingDelete.value = reminder;
}

async function confirmDelete() {
  const reminder = pendingDelete.value;
  if (!reminder) return;
  await removeReminder(reminder);
  pendingDelete.value = null;
}

function onKeydown(event: KeyboardEvent) {
  if (event.key !== "Escape") return;
  if (importOpen.value) {
    closeImport();
    return;
  }
  if (pendingDelete.value) {
    pendingDelete.value = null;
    return;
  }
  if (editorOpen.value) {
    closeEditor();
    return;
  }
  if (settingsOpen.value) settingsOpen.value = false;
}

function formatNext(reminder: Reminder) {
  if (!reminder.enabled) return "已暂停";
  const next = new Date(reminder.nextTriggerAt);
  if (Number.isNaN(next.getTime())) return "等待计算";
  if (next.getTime() <= tick.value) return "即将出现";

  const now = new Date(tick.value);
  const sameDay =
    next.getFullYear() === now.getFullYear() &&
    next.getMonth() === now.getMonth() &&
    next.getDate() === now.getDate();
  const tomorrow = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
  const isTomorrow =
    next.getFullYear() === tomorrow.getFullYear() &&
    next.getMonth() === tomorrow.getMonth() &&
    next.getDate() === tomorrow.getDate();
  const time = new Intl.DateTimeFormat("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false
  }).format(next);

  if (sameDay) return "今天 " + time;
  if (isTomorrow) return "明天 " + time;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "numeric",
    day: "numeric",
    weekday: "short",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false
  }).format(next);
}

function repeatLabel(reminder: Reminder) {
  if (reminder.repeat === "once") return "仅一次";
  if (reminder.repeat === "daily") return "每天 " + (reminder.timeOfDay ?? "");
  const minutes = reminder.intervalMinutes ?? 0;
  if (minutes >= 1440 && minutes % 1440 === 0) return "每 " + minutes / 1440 + " 天";
  if (minutes >= 60 && minutes % 60 === 0) return "每 " + minutes / 60 + " 小时";
  return "每 " + minutes + " 分钟";
}

function timeOfNext(reminder: Reminder) {
  return formatNext(reminder);
}

onMounted(async () => {
  await load();
  window.addEventListener("keydown", onKeydown);
  clockTimer = window.setInterval(() => {
    tick.value = Date.now();
  }, 30000);
  unlisten = await listen("request-new-reminder", () => {
    openEditor();
  });
  unlistenChanged = await listen("reminders-changed", () => {
    void load();
  });
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
  detachDragListeners();
  document.body.classList.remove("modal-open");
  if (unlisten) unlisten();
  if (unlistenChanged) unlistenChanged();
  if (clockTimer) window.clearInterval(clockTimer);
  if (noticeTimer) window.clearTimeout(noticeTimer);
});
</script>

<template>
  <div class="app-shell">
    <header class="app-header">
      <div class="brand">
        <div class="brand-mark" aria-hidden="true">
          <svg viewBox="0 0 512 512">
            <rect width="512" height="512" rx="116" fill="#f4e6d2" />
            <path
              d="M132 210 122 88l118 76c11-3 22-5 34-5s23 2 34 5l118-76-10 122c22 29 34 63 34 99 0 86-78 142-176 142S98 395 98 309c0-36 12-70 34-99Z"
              fill="#302c2a"
            />
            <ellipse cx="196" cy="294" rx="24" ry="29" fill="#f4e6d2" />
            <ellipse cx="316" cy="294" rx="24" ry="29" fill="#f4e6d2" />
            <ellipse cx="196" cy="300" rx="10" ry="16" fill="#302c2a" />
            <ellipse cx="316" cy="300" rx="10" ry="16" fill="#302c2a" />
            <path d="M232 350c14 13 34 13 48 0" fill="none" stroke="#f4e6d2" stroke-width="10" stroke-linecap="round" />
            <path d="M256 324l-12 12h24l-12-12Z" fill="#e8aab1" />
            <circle cx="386" cy="356" r="34" fill="#e7b24b" />
          </svg>
        </div>
        <div>
          <p class="eyebrow">提醒猫 · Tixingmao</p>
          <h1>轻轻提醒你一下</h1>
          <p class="brand-subtitle">平时安静待在托盘里，到点让猫咪来叫你。</p>
        </div>
      </div>

      <div class="header-actions">
        <div v-if="calendar" class="day-pill" :class="todayPillClass">
          <span>今天</span>
          <strong>{{ calendar.today.isWorkday ? '工作日' : calendar.today.label }}</strong>
        </div>
        <button type="button" class="ghost-button" @click="previewReminder(null)">预览猫咪</button>
        <button type="button" class="ghost-button" @click="settingsOpen = true">系统设置</button>
        <button type="button" class="primary-button compact" @click="openEditor()">
          <span aria-hidden="true">＋</span>
          新建提醒
        </button>
      </div>
    </header>

    <main class="workspace">
      <section class="reminder-column" aria-labelledby="reminder-heading">
        <div class="section-heading">
          <div class="section-title-row">
            <h2 id="reminder-heading">我的提醒</h2>
            <span class="count-badge">{{ activeCount }} 个进行中 · {{ reminders.length }} 个总计</span>
          </div>
          <button type="button" class="text-button position-link" @click="settingsOpen = true">
            猫咪出现位置 · {{ currentPositionLabel }}
          </button>
        </div>

        <div v-if="error" class="notice error">{{ error }}</div>
        <div v-if="success" class="notice success">{{ success }}</div>

        <div v-if="loading" class="empty-state">
          <div class="empty-state-inner">
            <div class="empty-cat" aria-hidden="true">◡</div>
            <h3>猫咪正在翻找提醒…</h3>
          </div>
        </div>

        <div v-else-if="displayReminders.length === 0" class="empty-state">
          <div class="empty-state-inner">
            <div class="empty-cat" aria-hidden="true">ฅ</div>
            <h3>这里还空空的</h3>
            <p>点右上角的「新建提醒」开始设置。到点后，猫咪会从屏幕{{ currentPositionLabel }}出现。</p>
          </div>
        </div>

        <div v-else ref="reminderListEl" class="reminder-list">
          <TransitionGroup name="card">
          <article
            v-for="reminder in displayReminders"
            :key="reminder.id"
            class="reminder-card"
            :class="{
              'is-disabled': !reminder.enabled,
              'is-dragging': draggingId === reminder.id
            }"
            :data-reminder-id="reminder.id"
          >
            <span
              class="drag-handle"
              :class="{ 'is-active': dragHandleId === reminder.id }"
              title="按住拖动排序"
              aria-hidden="true"
              @pointerdown="onHandlePointerDown(reminder, $event)"
            >
              <i></i><i></i><i></i><i></i><i></i><i></i>
            </span>
            <button
              type="button"
              class="switch"
              :class="{ 'is-on': reminder.enabled }"
              :aria-label="reminder.enabled ? '暂停提醒' : '启用提醒'"
              :aria-pressed="reminder.enabled"
              @click="toggleReminder(reminder)"
            >
              <span></span>
            </button>

            <div class="reminder-content">
              <div class="reminder-title-row">
                <h3>{{ reminder.title }}</h3>
                <span class="repeat-tag">{{ repeatLabel(reminder) }}</span>
              </div>
              <p v-if="reminder.message" class="reminder-message">{{ reminder.message }}</p>
              <div class="reminder-meta">
                <span>{{ timeOfNext(reminder) }}</span>
                <span v-if="reminder.workdayOnly" class="meta-dot">仅法定工作日</span>
                <span v-if="reminder.repeat === 'once' && reminder.autoDelete && !reminder.enabled" class="meta-dot">
                  完成后自动删除
                </span>
              </div>
            </div>

            <div class="card-actions">
              <button type="button" class="icon-button" title="预览" @click="previewReminder(reminder.id)">◉</button>
              <button type="button" class="icon-button" title="编辑" @click="openEditor(reminder)">✎</button>
              <button
                type="button"
                class="icon-button danger"
                title="删除"
                :disabled="deletingId === reminder.id"
                @click="requestDelete(reminder)"
              >
                ×
              </button>
            </div>
          </article>
          </TransitionGroup>
        </div>

        <div v-if="calendar" class="calendar-note">
          <strong>{{ calendar.note }}</strong><br />
          今天：{{ calendar.today.label }}。所有时间按北京时间计算。
        </div>
      </section>
    </main>

    <div v-if="editorOpen" class="modal-backdrop">
      <div class="modal-card" role="dialog" aria-modal="true" aria-labelledby="editor-title">
        <div class="modal-header">
          <div>
            <h2 id="editor-title">{{ editingId ? '编辑提醒' : '新建提醒' }}</h2>
            <p class="modal-subtitle">这些设置只作用于这一条提醒。</p>
          </div>
          <button type="button" class="icon-button modal-close" aria-label="关闭" @click="closeEditor">×</button>
        </div>

        <form class="reminder-form" @submit.prevent="save">
          <label class="field">
            <span class="field-label">提醒名称 <small>必填</small></span>
            <input
              ref="titleInput"
              v-model="form.title"
              class="text-input"
              maxlength="80"
              placeholder="例如：起来喝口水"
            />
          </label>

          <label class="field">
            <span class="field-label">猫咪说的话 <small>可选</small></span>
            <textarea
              v-model="form.message"
              class="text-area"
              maxlength="500"
              placeholder="例如：已经坐了很久啦，站起来伸个懒腰吧。"
            ></textarea>
          </label>

          <div class="field">
            <span class="field-label">提醒方式</span>
            <div class="segmented" role="group" aria-label="提醒方式">
              <button
                type="button"
                :class="{ 'is-active': form.repeat === 'once' }"
                @click="form.repeat = 'once'"
              >
                仅一次
              </button>
              <button
                type="button"
                :class="{ 'is-active': form.repeat === 'daily' }"
                @click="form.repeat = 'daily'"
              >
                每天
              </button>
              <button
                type="button"
                :class="{ 'is-active': form.repeat === 'interval' }"
                @click="form.repeat = 'interval'"
              >
                间隔提醒
              </button>
            </div>
          </div>

          <label v-if="form.repeat === 'once'" class="field">
            <span class="field-label">提醒时间</span>
            <input v-model="form.onceAt" class="text-input" type="datetime-local" />
          </label>

          <label v-else-if="form.repeat === 'daily'" class="field">
            <span class="field-label">每天几点</span>
            <input v-model="form.dailyTime" class="text-input" type="time" />
          </label>

          <div v-else class="field">
            <span class="field-label">提醒间隔</span>
            <div class="inline-fields">
              <input v-model.number="form.intervalValue" class="text-input" type="number" min="1" max="100000" />
              <select v-model="form.intervalUnit" class="select-input" aria-label="间隔单位">
                <option value="minutes">分钟</option>
                <option value="hours">小时</option>
                <option value="days">天</option>
              </select>
            </div>
          </div>

          <label class="checkbox-row">
            <input v-model="form.workdayOnly" type="checkbox" />
            <span class="checkbox-copy">
              <strong>只在法定工作日提醒</strong>
              <span>自动跳过周末和国家法定节假日，包含官方调休上班日。遇到非工作日会顺延。</span>
            </span>
          </label>

          <label v-if="form.repeat === 'once'" class="checkbox-row">
            <input v-model="form.autoDelete" type="checkbox" />
            <span class="checkbox-copy">
              <strong>关闭提醒后自动删除</strong>
              <span>只对一次性提醒生效，列表里不会留下已完成的记录。</span>
            </span>
          </label>

          <div v-if="formError" class="notice error">{{ formError }}</div>

          <div class="modal-actions">
            <button type="button" class="ghost-button" :disabled="saving" @click="closeEditor">取消</button>
            <button class="primary-button compact" type="submit" :disabled="saving">
              <svg class="button-paw" viewBox="0 0 24 24" aria-hidden="true">
                <ellipse cx="6.2" cy="9.2" rx="2.4" ry="3.1" fill="currentColor" transform="rotate(-24 6.2 9.2)" />
                <ellipse cx="10.4" cy="5.8" rx="2.2" ry="3" fill="currentColor" transform="rotate(-8 10.4 5.8)" />
                <ellipse cx="15.2" cy="6.1" rx="2.2" ry="3" fill="currentColor" transform="rotate(9 15.2 6.1)" />
                <ellipse cx="19.1" cy="9.7" rx="2.3" ry="3" fill="currentColor" transform="rotate(25 19.1 9.7)" />
                <path d="M6.8 16.3c0-4.1 3.1-7.2 6.1-7.2s5.7 3.1 5.7 7.1c0 2.5-1.7 3.9-4 3.9-1.4 0-2.1-.7-3.2-.7-1.1 0-1.8.7-3.1.7-2.1 0-3.1-1.4-3.1-3.5Z" fill="currentColor" />
              </svg>
              {{ saving ? '正在保存…' : editingId ? '保存修改' : '让猫咪记住' }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <div v-if="settingsOpen" class="modal-backdrop" @click.self="settingsOpen = false">
      <div class="modal-card modal-settings" role="dialog" aria-modal="true" aria-labelledby="settings-title">
        <div class="modal-header">
          <div>
            <h2 id="settings-title">系统设置</h2>
            <p class="modal-subtitle">这里影响所有提醒，与提醒内容分开保存。</p>
          </div>
          <button type="button" class="icon-button modal-close" aria-label="关闭" @click="settingsOpen = false">×</button>
        </div>

        <div class="settings-section">
          <div class="settings-section-heading">
            <h3>猫咪出现位置</h3>
            <span>当前：{{ currentPositionLabel }}</span>
          </div>
          <div class="position-grid" role="radiogroup" aria-label="猫咪出现位置">
            <button
              v-for="option in positionOptions"
              :key="option.value"
              type="button"
              class="position-option"
              :class="{ 'is-active': overlayPosition === option.value }"
              role="radio"
              :aria-checked="overlayPosition === option.value"
              @click="changeOverlayPosition(option.value)"
            >
              {{ option.label }}
            </button>
          </div>
          <p class="settings-hint">选择后立即生效，之后所有提醒都会出现在这里。</p>
        </div>

        <div class="settings-section">
          <div class="settings-section-heading">
            <h3>法定节假日与调休</h3>
            <div class="settings-section-actions">
              <span>{{ calendarOverrides.length }} 个自定义日期</span>
              <button type="button" class="text-button import-link" @click="openImport">批量导入</button>
            </div>
          </div>
          <p class="settings-hint">
            内置 2024–2026 年官方数据。新一年的安排公布后，可在这里补上休息日和调休上班日，只影响“仅法定工作日”的提醒。
          </p>

          <div class="override-form">
            <input v-model="overrideDate" class="text-input" type="date" aria-label="选择日期" />
            <div class="segmented override-kind" role="group" aria-label="日期类型">
              <button
                type="button"
                :class="{ 'is-active': overrideKind === 'holiday' }"
                @click="overrideKind = 'holiday'"
              >
                休息
              </button>
              <button
                type="button"
                :class="{ 'is-active': overrideKind === 'workday' }"
                @click="overrideKind = 'workday'"
              >
                调休上班
              </button>
            </div>
            <button type="button" class="primary-button compact" :disabled="savingOverride" @click="saveOverride">
              {{ savingOverride ? '保存中…' : '保存' }}
            </button>
          </div>

          <p v-if="overridePreview" class="override-preview">
            当前判断：<strong>{{ overridePreview.label }}</strong> · {{ overridePreview.isWorkday ? '上班' : '休息' }}
          </p>
          <div v-if="overrideError" class="notice error">{{ overrideError }}</div>
          <div v-if="overrideNotice" class="notice success">{{ overrideNotice }}</div>

          <ul v-if="calendarOverrides.length" class="override-list">
            <li v-for="entry in calendarOverrides" :key="entry.date">
              <span class="override-date">{{ overrideDateLabel(entry.date) }}</span>
              <span class="override-tag" :class="entry.kind === 'holiday' ? 'is-rest' : 'is-work'">
                {{ entry.kind === 'holiday' ? '休息' : '调休上班' }}
              </span>
              <button
                type="button"
                class="icon-button danger"
                title="删除"
                :disabled="deletingOverride === entry.date"
                @click="removeOverride(entry.date)"
              >
                ×
              </button>
            </li>
          </ul>
          <p v-else class="settings-hint">还没有自定义日期，默认使用内置节假日数据。</p>
        </div>

        <div class="modal-actions">
          <button type="button" class="ghost-button" @click="previewReminder(null)">预览效果</button>
          <button type="button" class="primary-button compact" @click="settingsOpen = false">完成</button>
        </div>
      </div>
    </div>

    <div v-if="importOpen" class="modal-backdrop">
      <div class="modal-card modal-import" role="dialog" aria-modal="true" aria-labelledby="import-title">
        <div class="modal-header">
          <div>
            <h2 id="import-title">批量导入假期</h2>
            <p class="modal-subtitle">
              粘贴国务院公布的放假和调休日期，一行一个，例如 2026-01-01。重复日期会自动合并，同一天以本次导入为准。
            </p>
          </div>
          <button type="button" class="icon-button modal-close" aria-label="关闭" @click="closeImport">×</button>
        </div>

        <div class="import-grid">
          <label class="field">
            <span class="field-label">放假日 <small>休息</small></span>
            <textarea
              v-model="importHolidays"
              class="text-area import-area"
              spellcheck="false"
              placeholder="2026-01-01&#10;2026-01-02&#10;2026-01-03"
            ></textarea>
          </label>
          <label class="field">
            <span class="field-label">调休上班日 <small>上班</small></span>
            <textarea
              v-model="importWorkdays"
              class="text-area import-area"
              spellcheck="false"
              placeholder="2026-01-04&#10;2026-02-14"
            ></textarea>
          </label>
        </div>

        <div v-if="importError" class="notice error">{{ importError }}</div>

        <div class="modal-actions">
          <button type="button" class="ghost-button" :disabled="importing" @click="closeImport">取消</button>
          <button type="button" class="primary-button compact" :disabled="importing" @click="submitImport">
            {{ importing ? '正在导入…' : '开始导入' }}
          </button>
        </div>
      </div>
    </div>

    <div v-if="pendingDelete" class="modal-backdrop" @click.self="pendingDelete = null">
      <div class="modal-card modal-confirm" role="alertdialog" aria-modal="true" aria-labelledby="delete-title">
        <div class="modal-header">
          <h2 id="delete-title">删除这条提醒？</h2>
          <button type="button" class="icon-button modal-close" aria-label="关闭" @click="pendingDelete = null">×</button>
        </div>
        <p class="confirm-copy">
          「{{ pendingDelete.title }}」删除后无法恢复。之后再想提醒，需要重新新建。
        </p>
        <div class="modal-actions">
          <button type="button" class="ghost-button" @click="pendingDelete = null">留着吧</button>
          <button
            type="button"
            class="danger-button"
            :disabled="deletingId === pendingDelete.id"
            @click="confirmDelete"
          >
            {{ deletingId === pendingDelete.id ? '正在删除…' : '删除提醒' }}
          </button>
        </div>
      </div>
    </div>

    <footer class="footer-note">
      <span>关闭主窗口不会退出，提醒猫会继续在托盘运行。</span>
      <span>按住卡片左侧的圆点可以拖动排序。</span>
      <span>右键托盘图标可以打开列表或退出。</span>
    </footer>
  </div>
</template>

