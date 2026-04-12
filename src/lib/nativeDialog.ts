import { ask, message } from "@tauri-apps/plugin-dialog";

export async function confirmDestructive(body: string, title = "确认"): Promise<boolean> {
  return ask(body, { title, kind: "warning" });
}

export async function showErrorDetail(detail: string, title = "操作失败"): Promise<void> {
  await message(detail, { title, kind: "error" });
}
