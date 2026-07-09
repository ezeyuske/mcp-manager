import { ScrollText } from "lucide-react";
import { ScreenShell } from "./ScreenShell";
import { EmptyState } from "./EmptyState";

export function ActivityScreen() {
  return (
    <ScreenShell
      title="Actividad"
      subtitle="Log de cambios: qué se modificó, cuándo y en qué archivo, con restore."
    >
      <EmptyState
        icon={ScrollText}
        title="Log de cambios y backups"
        description="Historial de escrituras a configs externos con backup timestampeado y botón de restore por punto."
        phase="Llega en Fase 3"
      />
    </ScreenShell>
  );
}
