import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "../lib/tauri";
import { useInventory } from "./inventory";
import { useToast } from "./toast";
import type { McpTarget, VaultSecretInfo } from "../types/inventory";

type Status = "idle" | "loading" | "ready" | "error";

const MOCK_SECRETS: VaultSecretInfo[] = [
  { name: "NOTION_TOKEN", usedBy: 1 },
  { name: "GITHUB_PAT", usedBy: 0 },
];

interface VaultState {
  status: Status;
  secrets: VaultSecretInfo[];
  error: string | null;
  mocked: boolean;
  busy: boolean;
  load: () => Promise<void>;
  setSecret: (name: string, value: string) => Promise<boolean>;
  deleteSecret: (name: string) => Promise<boolean>;
  reveal: (name: string) => Promise<string | null>;
  bind: (target: McpTarget, envKey: string, secretName: string) => Promise<boolean>;
  unbind: (target: McpTarget, envKey: string) => Promise<boolean>;
}

async function afterMutation() {
  await useInventory.getState().load();
  await useVault.getState().load();
}

export const useVault = create<VaultState>((set) => ({
  status: "idle",
  secrets: [],
  error: null,
  mocked: false,
  busy: false,

  load: async () => {
    set({ status: "loading", error: null });
    if (!isTauri()) {
      set({ status: "ready", secrets: MOCK_SECRETS, mocked: true });
      return;
    }
    try {
      const secrets = await invoke<VaultSecretInfo[]>("vault_list");
      set({ status: "ready", secrets, mocked: false });
    } catch (err) {
      set({ status: "error", error: String(err), secrets: [] });
    }
  },

  setSecret: (name, value) =>
    runVault(set, `Secreto "${name}" guardado.`, "vault_set_secret", {
      name,
      value,
    }),

  deleteSecret: (name) =>
    runVault(set, `Secreto "${name}" eliminado.`, "vault_delete_secret", {
      name,
    }),

  reveal: async (name) => {
    if (!isTauri()) return "•••• (modo browser)";
    try {
      return await invoke<string>("vault_reveal", { name });
    } catch (err) {
      useToast.getState().push("error", String(err));
      return null;
    }
  },

  bind: (target, envKey, secretName) =>
    runVault(set, `Vinculado a "${secretName}".`, "bind_env_secret", {
      target,
      envKey,
      secretName,
    }),

  unbind: (target, envKey) =>
    runVault(set, `Vínculo quitado.`, "unbind_env_secret", { target, envKey }),
}));

async function runVault(
  set: (p: Partial<VaultState>) => void,
  okMessage: string,
  command: string,
  args: Record<string, unknown>,
): Promise<boolean> {
  const toast = useToast.getState();
  if (!isTauri()) {
    toast.push("info", "No disponible en modo browser (sin backend).");
    return false;
  }
  set({ busy: true });
  try {
    await invoke(command, args);
    await afterMutation();
    toast.push("success", okMessage);
    return true;
  } catch (err) {
    toast.push("error", String(err));
    return false;
  } finally {
    set({ busy: false });
  }
}
