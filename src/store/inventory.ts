import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { Inventory } from "../types/inventory";
import { isTauri } from "../lib/tauri";
import { MOCK_INVENTORY } from "../mocks/inventory";

type Status = "idle" | "loading" | "ready" | "error";

interface InventoryState {
  status: Status;
  inventory: Inventory | null;
  error: string | null;
  /** True si los datos vienen del mock de browser (sin backend). */
  mocked: boolean;
  load: () => Promise<void>;
}

export const useInventory = create<InventoryState>((set) => ({
  status: "idle",
  inventory: null,
  error: null,
  mocked: false,

  load: async () => {
    set({ status: "loading", error: null });

    // En browser (pnpm dev) no hay backend: usamos el mock para poder ver la UI.
    if (!isTauri()) {
      set({ status: "ready", inventory: MOCK_INVENTORY, mocked: true });
      return;
    }

    try {
      const inventory = await invoke<Inventory>("get_inventory");
      set({ status: "ready", inventory, mocked: false });
    } catch (err) {
      set({ status: "error", error: String(err), inventory: null });
    }
  },
}));
