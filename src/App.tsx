import { useEffect, type ComponentType } from "react";
import { Sidebar } from "./components";
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

  // Aplica el acento guardado al montar.
  useEffect(() => hydrate(), [hydrate]);

  const Active = SCREENS[screen];

  return (
    <div className="flex h-screen w-screen overflow-hidden">
      <Sidebar />
      <main className="min-w-0 flex-1 overflow-hidden">
        <Active key={screen} />
      </main>
    </div>
  );
}

export default App;
