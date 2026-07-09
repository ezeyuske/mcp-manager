/** True cuando corremos dentro de la ventana Tauri (no en `pnpm dev` browser). */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
