import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "../lib/tauri";
import { useInventory } from "./inventory";
import { useToast } from "./toast";
import type { MutationLog } from "../types/inventory";

type Status = "idle" | "loading" | "ready" | "error";

const MOCK_LOG: MutationLog[] = [
  {
    id: "3",
    timestamp: "2026-07-09T14:20:11.000+00:00",
    app: "claude-code",
    scope: "project",
    filePath: "~/www/ia-tools/others/mcp-manager/.mcp.json",
    action: "add",
    mcpName: "local-tool",
    backupPath: null,
  },
  {
    id: "2",
    timestamp: "2026-07-09T14:18:02.000+00:00",
    app: "claude-desktop",
    scope: "user",
    filePath: "~/Library/Application Support/Claude/claude_desktop_config.json",
    action: "edit",
    mcpName: "context7",
    backupPath:
      "~/.mcp-manager/backups/claude-desktop/claude_desktop_config.json.20260709-141802.json",
  },
  {
    id: "1",
    timestamp: "2026-07-09T14:10:45.000+00:00",
    app: "claude-desktop",
    scope: "user",
    filePath: "~/Library/Application Support/Claude/claude_desktop_config.json",
    action: "disable",
    mcpName: "notion",
    backupPath:
      "~/.mcp-manager/backups/claude-desktop/claude_desktop_config.json.20260709-141045.json",
  },
];

interface ChangelogState {
  status: Status;
  entries: MutationLog[];
  error: string | null;
  mocked: boolean;
  load: () => Promise<void>;
  restore: (backupPath: string, targetPath: string) => Promise<void>;
}

export const useChangelog = create<ChangelogState>((set) => ({
  status: "idle",
  entries: [],
  error: null,
  mocked: false,

  load: async () => {
    set({ status: "loading", error: null });
    if (!isTauri()) {
      set({ status: "ready", entries: MOCK_LOG, mocked: true });
      return;
    }
    try {
      const entries = await invoke<MutationLog[]>("list_changelog");
      set({ status: "ready", entries, mocked: false });
    } catch (err) {
      set({ status: "error", error: String(err), entries: [] });
    }
  },

  restore: async (backupPath, targetPath) => {
    const toast = useToast.getState();
    if (!isTauri()) {
      toast.push("info", "No disponible en modo browser (sin backend).");
      return;
    }
    try {
      await invoke("restore_backup", { backupPath, targetPath });
      toast.push("success", "Backup restaurado.");
      await useInventory.getState().load();
      await useChangelog.getState().load();
    } catch (err) {
      toast.push("error", String(err));
    }
  },
}));
