import { create } from "zustand";
import type { Screen } from "../types/nav";

interface UIState {
  screen: Screen;
  setScreen: (screen: Screen) => void;
}

/** Navegación por view-state (sin router): la sidebar cambia `screen`. */
export const useUI = create<UIState>((set) => ({
  screen: "mcps",
  setScreen: (screen) => set({ screen }),
}));
