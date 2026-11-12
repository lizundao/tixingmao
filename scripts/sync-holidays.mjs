import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const years = [2024, 2025, 2026, 2027];
const outputYears = {};

function toNamedDates(value) {
  const result = {};
  for (const [date, raw] of Object.entries(value ?? {})) {
    const parts = String(raw).split(",");
    result[date] = parts.length > 1 ? parts[1].trim() : String(raw);
  }
  return result;
}

for (const year of years) {
  const source = join("node_modules", "chinese-days", "dist", "years", String(year) + ".json");
  const raw = JSON.parse(readFileSync(source, "utf8"));
  outputYears[String(year)] = {
    holidays: toNamedDates(raw.holidays),
    workdays: toNamedDates(raw.workdays)
  };
}

const output = {
  source: "https://github.com/vsme/chinese-days",
  packageVersion: "1.5.10",
  license: "MIT",
  generatedAt: "2026-11-12",
  years: outputYears
};

mkdirSync(join("src-tauri", "data"), { recursive: true });
writeFileSync(
  join("src-tauri", "data", "china-holidays.json"),
  JSON.stringify(output, null, 2) + "\n",
  "utf8"
);

console.log("已生成 " + years.length + " 年的中国法定节假日/调休数据");

