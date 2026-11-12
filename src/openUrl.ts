import { openUrl } from "@tauri-apps/plugin-opener";

function isTauri(): boolean {
  return (
    typeof window !== "undefined" &&
    ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)
  );
}

/** 用系统默认浏览器打开外部链接；非 Tauri 环境或插件失败时回退到 window.open。 */
export async function openExternalUrl(url: string): Promise<void> {
  if (isTauri()) {
    try {
      await openUrl(url);
      return;
    } catch {
      // 插件异常时回退到系统浏览器
    }
  }
  window.open(url, "_blank", "noopener,noreferrer");
}
