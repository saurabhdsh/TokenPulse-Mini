import { useCallback } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

const NO_DRAG_SELECTOR = "button, a, input, select, textarea, label, [data-no-drag]";

export function useWindowDrag() {
  return useCallback((e: React.MouseEvent) => {
    if (e.button !== 0) return;
    if ((e.target as HTMLElement).closest(NO_DRAG_SELECTOR)) return;
    void getCurrentWindow().startDragging();
  }, []);
}
