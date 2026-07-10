import { useEffect, type ComponentType } from "react";
import { Sidebar, ToastHost } from "./components";
import {
  McpsScreen,
  SkillsScreen,
  ProjectsScreen,
  EnvSecretsScreen,
  ActivityScreen,
  ThemesScreen,
  SettingsScreen,
} from "./screens";
import { useUI } from "./store/ui";
import { useTheme } from "./store/theme";
import { useInventory } from "./store/inventory";
import { useSkills } from "./store/skills";
import { useProjects } from "./store/projects";
import type { Screen } from "./types/nav";

const SCREENS: Record<Screen, ComponentType> = {
  mcps: McpsScreen,
  skills: SkillsScreen,
  projects: ProjectsScreen,
  env: EnvSecretsScreen,
  activity: ActivityScreen,
  themes: ThemesScreen,
  settings: SettingsScreen,
};

function App() {
  const screen = useUI((s) => s.screen);
  const hydrate = useTheme((s) => s.hydrate);
  const loadInventory = useInventory((s) => s.load);
  const loadSkills = useSkills((s) => s.load);
  const loadProjects = useProjects((s) => s.load);

  // Aplica el acento guardado y carga inventario/skills/proyectos al montar
  // (para poblar los badges del sidebar antes de visitar cada pantalla).
  useEffect(() => {
    hydrate();
    loadInventory();
    loadSkills();
    loadProjects();
  }, [hydrate, loadInventory, loadSkills, loadProjects]);

  const Active = SCREENS[screen];

  return (
    <div className="flex h-screen w-screen overflow-hidden">
      <Sidebar />
      <main className="min-w-0 flex-1 overflow-hidden">
        <Active key={screen} />
      </main>
      <ToastHost />
    </div>
  );
}

export default App;
