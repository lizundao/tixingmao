export type RepeatKind = "once" | "daily" | "interval";

export type OverlayPosition =
  | "topLeft"
  | "topCenter"
  | "topRight"
  | "middleLeft"
  | "center"
  | "middleRight"
  | "bottomLeft"
  | "bottomCenter"
  | "bottomRight";

export type DayOverrideKind = "holiday" | "workday";

export interface CalendarOverride {
  date: string;
  kind: DayOverrideKind;
}

export interface CalendarImportResult {
  entries: CalendarOverride[];
  holidayCount: number;
  workdayCount: number;
}

export interface Reminder {
  id: string;
  title: string;
  message: string;
  enabled: boolean;
  repeat: RepeatKind;
  scheduledAt: string | null;
  timeOfDay: string | null;
  intervalMinutes: number | null;
  workdayOnly: boolean;
  nextTriggerAt: string;
  lastTriggeredAt: string | null;
  createdAt: string;
  autoDelete: boolean;
}

export interface ReminderDraft {
  id?: string;
  title: string;
  message: string;
  repeat: RepeatKind;
  scheduledAt: string | null;
  timeOfDay: string | null;
  intervalMinutes: number | null;
  workdayOnly: boolean;
  autoDelete: boolean;
}

export interface DayStatus {
  date: string;
  isWorkday: boolean;
  kind: "workday" | "holiday" | "weekend";
  label: string;
  dataAvailable: boolean;
}

export interface CalendarInfo {
  today: DayStatus;
  coverageYears: number[];
  note: string;
}

