import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "../lib/tauri";
import { useToast } from "./toast";
import type { Skill, SkillInput, SkillTarget } from "../types/skills";

type Status = "idle" | "loading" | "ready" | "error";

const MOCK_SKILLS: Skill[] = [
  {
    name: "frontend-design",
    description:
      "Create distinctive, production-grade frontend interfaces with high design quality.",
    version: null,
    scope: "user",
    path: "~/.claude/skills/frontend-design",
    enabled: true,
  },
  {
    name: "mcp-sentinel",
    description:
      "Security monitoring agent for Claude Skills and MCP servers. Blocks malicious tool calls in real time.",
    version: "2.0.0",
    scope: "user",
    path: "~/.claude/skills/mcp-sentinel",
    enabled: true,
  },
  {
    name: "obsidian-cli",
    description:
      "Interact with Obsidian vaults using the Obsidian CLI to read, create, search and manage notes.",
    version: null,
    scope: "user",
    path: "~/.claude/skills/obsidian-cli",
    enabled: false,
  },
  {
    name: "mcp-manager-design",
    description:
      "Design system de MCP Manager (dark glassmórfico premium, acento configurable en runtime).",
    version: null,
    scope: "project",
    projectPath: "~/www/ia-tools/others/mcp-manager",
    path: "~/www/ia-tools/others/mcp-manager/.claude/skills/mcp-manager-design",
    enabled: true,
  },
];

interface SkillsState {
  status: Status;
  skills: Skill[];
  error: string | null;
  mocked: boolean;
  busy: boolean;
  load: () => Promise<void>;
  setEnabled: (target: SkillTarget, enabled: boolean) => Promise<boolean>;
  remove: (target: SkillTarget) => Promise<boolean>;
  rename: (target: SkillTarget, newName: string) => Promise<boolean>;
  upsert: (input: SkillInput) => Promise<boolean>;
}

export const useSkills = create<SkillsState>((set) => ({
  status: "idle",
  skills: [],
  error: null,
  mocked: false,
  busy: false,

  load: async () => {
    set({ status: "loading", error: null });
    if (!isTauri()) {
      set({ status: "ready", skills: MOCK_SKILLS, mocked: true });
      return;
    }
    try {
      const skills = await invoke<Skill[]>("get_skills");
      set({ status: "ready", skills, mocked: false });
    } catch (err) {
      set({ status: "error", error: String(err), skills: [] });
    }
  },

  setEnabled: (target, enabled) =>
    runSkill(
      set,
      `Skill "${target.name}" ${enabled ? "habilitada" : "deshabilitada"}.`,
      "set_skill_enabled",
      { target, enabled },
    ),

  remove: (target) =>
    runSkill(set, `Skill "${target.name}" eliminada.`, "delete_skill", {
      target,
    }),

  rename: (target, newName) =>
    runSkill(
      set,
      `Skill "${target.name}" renombrada a "${newName}".`,
      "rename_skill",
      { target, newName },
    ),

  upsert: (input) =>
    runSkill(set, `Skill "${input.name}" guardada.`, "upsert_skill", {
      input,
    }),
}));

async function runSkill(
  set: (p: Partial<SkillsState>) => void,
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
    await useSkills.getState().load();
    toast.push("success", okMessage);
    return true;
  } catch (err) {
    toast.push("error", String(err));
    return false;
  } finally {
    set({ busy: false });
  }
}
