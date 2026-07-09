import { FolderGit2 } from "lucide-react";
import { ScreenShell } from "./ScreenShell";
import { EmptyState } from "./EmptyState";

export function ProjectsScreen() {
  return (
    <ScreenShell
      title="Proyectos"
      subtitle="Repos conocidos y sus MCPs/skills por scope, con diffs contra el global."
    >
      <EmptyState
        icon={FolderGit2}
        title="Vista por proyecto"
        description="Elegí un repo y gestioná su .mcp.json de scope proyecto, con comparación contra el scope user."
        phase="Llega en Fase 5"
      />
    </ScreenShell>
  );
}
