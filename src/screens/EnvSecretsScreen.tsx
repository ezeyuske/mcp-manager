import { KeyRound } from "lucide-react";
import { ScreenShell } from "./ScreenShell";
import { EmptyState } from "./EmptyState";

export function EnvSecretsScreen() {
  return (
    <ScreenShell
      title="Env & Secrets"
      subtitle="Variables de ambiente y vault de secrets en el keychain del OS."
    >
      <EmptyState
        icon={KeyRound}
        title="Variables y secrets"
        description="Editor clave/valor del bloque env, variables compartidas y secrets cifrados en el keychain (con máscara + revelar)."
        phase="Llega en Fase 4"
      />
    </ScreenShell>
  );
}
