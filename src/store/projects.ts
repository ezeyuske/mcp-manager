import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { isTauri } from "../lib/tauri";
import { useInventory } from "./inventory";
import { useSkills } from "./skills";
import { useToast } from "./toast";

type Status = "idle" | "loading" | "ready" | "error";

const MOCK_PROJECTS = ["~/www/ia-tools/others/mcp-manager"];

interface ProjectsState {
  status: Status;
  projects: string[];
  error: string | null;
  mocked: boolean;
  load: () => Promise<void>;
  /** Abre el file picker, registra el repo elegido. */
  registerPicked: () => Promise<void>;
  unregister: (path: string) => Promise<void>;
}

async function refreshAll() {
  await useInventory.getState().load();
  await useSkills.getState().load();
  await useProjects.getState().load();
}

export const useProjects = create<ProjectsState>((set) => ({
  status: "idle",
  projects: [],
  error: null,
  mocked: false,

  load: async () => {
    set({ status: "loading", error: null });
    if (!isTauri()) {
      set({ status: "ready", projects: MOCK_PROJECTS, mocked: true });
      return;
    }
    try {
      const projects = await invoke<string[]>("list_projects");
      set({ status: "ready", projects, mocked: false });
    } catch (err) {
      set({ status: "error", error: String(err), projects: [] });
    }
  },

  registerPicked: async () => {
    const toast = useToast.getState();
    if (!isTauri()) {
      toast.push("info", "El selector de carpetas requiere la app (no browser).");
      return;
    }
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked !== "string") return;
    try {
      await invoke("register_project_dir", { path: picked });
      await refreshAll();
      toast.push("success", "Proyecto registrado.");
    } catch (err) {
      toast.push("error", String(err));
    }
  },

  unregister: async (path) => {
    const toast = useToast.getState();
    if (!isTauri()) {
      toast.push("info", "No disponible en modo browser (sin backend).");
      return;
    }
    try {
      await invoke("unregister_project", { path });
      await refreshAll();
      toast.push("success", "Proyecto quitado.");
    } catch (err) {
      toast.push("error", String(err));
    }
  },
}));
