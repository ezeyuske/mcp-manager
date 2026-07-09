import { Sparkles } from "lucide-react";
import { ScreenShell } from "./ScreenShell";
import { EmptyState } from "./EmptyState";

export function SkillsScreen() {
  return (
    <ScreenShell
      title="Skills"
      subtitle="Carpetas con SKILL.md por scope, con name y description del frontmatter."
    >
      <EmptyState
        icon={Sparkles}
        title="Inventario de Skills"
        description="Listado de skills en ~/.claude/skills y .claude/skills de cada proyecto, con parseo de frontmatter y scope."
        phase="Llega en Fase 5"
      />
    </ScreenShell>
  );
}
