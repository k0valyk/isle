import { invoke } from "@tauri-apps/api/core";

export interface ClipboardItem {
  id: number;
  kind: string;
  content: string;
  preview: string | null;
  source_app: string | null;
  ts: number;
  pinned: boolean;
}

export function listClipboard(limit = 200): Promise<ClipboardItem[]> {
  return invoke<ClipboardItem[]>("clipboard_list", { limit });
}

export function searchClipboard(query: string): Promise<ClipboardItem[]> {
  return invoke<ClipboardItem[]>("clipboard_search", { q: query });
}

export function pinClipboard(id: number, pinned: boolean): Promise<void> {
  return invoke("clipboard_pin", { id, pinned });
}

export function deleteClipboard(id: number): Promise<void> {
  return invoke("clipboard_delete", { id });
}

export function captureClipboard(): Promise<boolean> {
  return invoke<boolean>("clipboard_capture");
}

export function writeClipboard(text: string): Promise<void> {
  return invoke("clipboard_write", { text });
}

export function copyClipboardItem(id: number): Promise<void> {
  return invoke("clipboard_copy_item", { id });
}

export function previewClipboardItem(id: number): Promise<string | null> {
  return invoke<string | null>("clipboard_preview_data", { id });
}

export function clearClipboard(keepPinned: boolean): Promise<void> {
  return invoke("clipboard_clear", { keepPinned });
}
