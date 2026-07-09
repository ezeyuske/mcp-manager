import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * Fase 0: shell mínimo. Invoca `detect_platform` una vez al montar para
 * confirmar que el pipeline IPC (Tauri commands + plugins) levanta OK.
 * En Fase 1 este componente se reemplaza por el shell con Sidebar + screens.
 */
function App() {
  const [platform, setPlatform] = useState<string>("…");

  useEffect(() => {
    invoke<string>("detect_platform")
      .then(setPlatform)
      .catch((err) => setPlatform(`error: ${String(err)}`));
  }, []);

  return (
    <main className="flex h-full flex-col items-center justify-center gap-3 p-10 text-center">
      <h1 className="text-2xl font-semibold tracking-tight">MCP Manager</h1>
      <p className="text-sm text-slate-400">
        Fase 0 — Tailwind v4 activo · plugins Tauri wireados
      </p>
      <p className="text-xs text-slate-500">
        platform: <span className="font-mono text-slate-300">{platform}</span>
      </p>
    </main>
  );
}

export default App;
