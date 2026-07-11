import { create } from "zustand";
import { ACCENTS, type Accent, type AccentKey } from "../types/theme";

const STORAGE_KEY = "mcp-manager.accent";

/** Escribe las variables del acento en :root (theming en caliente). */
function applyAccent(accent: Accent): void {
  const root = document.documentElement;
  root.style.setProperty("--accent", accent.accent);
  root.style.setProperty("--accent-strong", accent.strong);
  root.style.setProperty("--accent-contrast", accent.contrast);
  root.style.setProperty("--accent-glow", accent.glow);
  root.style.setProperty("--accent-soft", accent.soft);
}

function initialAccent(): AccentKey {
  const saved = localStorage.getItem(STORAGE_KEY) as AccentKey | null;
  return saved && saved in ACCENTS ? saved : "coral";
}

interface ThemeState {
  accent: AccentKey;
  setAccent: (key: AccentKey) => void;
  /** Aplica el acento actual al DOM (llamar una vez al montar la app). */
  hydrate: () => void;
}

export const useTheme = create<ThemeState>((set, get) => ({
  accent: initialAccent(),
  setAccent: (key) => {
    applyAccent(ACCENTS[key]);
    localStorage.setItem(STORAGE_KEY, key);
    set({ accent: key });
  },
  hydrate: () => applyAccent(ACCENTS[get().accent]),
}));
