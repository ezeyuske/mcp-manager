import { useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Modal,
  Button,
  Input,
  Textarea,
  Select,
  SegmentedControl,
  KeyValueEditor,
  type KeyValue,
} from "../../components";
import { useMutations } from "../../store/mutations";
import { isTauri } from "../../lib/tauri";
import { useToast } from "../../store/toast";
import type {
  AppId,
  McpInstallation,
  McpServerConfigInput,
  Scope,
  TransportKind,
} from "../../types/inventory";

interface McpFormModalProps {
  open: boolean;
  onClose: () => void;
  /** Si viene, el modal está en modo edición (identidad app/scope/name fija). */
  initial?: McpInstallation | null;
}

type FormTransport = "stdio" | "http" | "sse";

export function McpFormModal({ open: isOpen, onClose, initial }: McpFormModalProps) {
  const editing = !!initial;
  const upsert = useMutations((s) => s.upsert);
  const busy = useMutations((s) => s.busy);
  const registerProject = useMutations((s) => s.registerProject);
  const pushToast = useToast((s) => s.push);

  const [name, setName] = useState(initial?.name ?? "");
  const [scope, setScope] = useState<Scope>(initial?.scope ?? "user");
  const [app, setApp] = useState<AppId>(initial?.app ?? "claude-desktop");
  const [projectPath, setProjectPath] = useState(initial?.projectPath ?? "");
  const [transport, setTransport] = useState<FormTransport>(
    (initial?.transport as FormTransport) ?? "stdio",
  );
  const [command, setCommand] = useState(initial?.command ?? "");
  const [argsText, setArgsText] = useState((initial?.args ?? []).join("\n"));
  const [url, setUrl] = useState(initial?.url ?? "");
  const [env, setEnv] = useState<KeyValue[]>([]);

  // Restricciones de dominio:
  // - scope proyecto ⇒ el destino es un .mcp.json (lo tratamos como Claude Code).
  // - Claude Desktop solo soporta stdio.
  const effectiveApp: AppId = scope === "project" ? "claude-code" : app;
  const desktopOnlyStdio = effectiveApp === "claude-desktop";
  const transportOptions = useMemo<{ value: FormTransport; label: string }[]>(
    () =>
      desktopOnlyStdio
        ? [{ value: "stdio", label: "stdio" }]
        : [
            { value: "stdio", label: "stdio" },
            { value: "http", label: "HTTP" },
            { value: "sse", label: "SSE" },
          ],
    [desktopOnlyStdio],
  );
  const effectiveTransport: FormTransport = desktopOnlyStdio ? "stdio" : transport;
  const isStdio = effectiveTransport === "stdio";

  const canSave =
    name.trim() !== "" &&
    (scope === "user" || projectPath.trim() !== "") &&
    (isStdio ? command.trim() !== "" : url.trim() !== "");

  async function pickFolder() {
    if (!isTauri()) {
      pushToast("info", "El selector de carpetas requiere la app (no browser).");
      return;
    }
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === "string") setProjectPath(picked);
  }

  async function save() {
    const args = argsText
      .split("\n")
      .map((a) => a.trim())
      .filter(Boolean);

    const config: McpServerConfigInput = {
      args: isStdio ? args : [],
      // ADD: env desde el editor. EDIT: vacío ⇒ el backend preserva el env
      // existente (los valores de env no cruzan al frontend; su gestión es Fase 4).
      env:
        !editing && isStdio
          ? Object.fromEntries(
              env
                .filter((e) => e.key.trim() !== "")
                .map((e) => [e.key.trim(), e.value]),
            )
          : {},
    };

    if (isStdio) {
      config.command = command.trim();
      if (effectiveApp === "claude-code") config.type = "stdio";
    } else {
      config.type = effectiveTransport;
      config.url = url.trim();
    }

    if (scope === "project" && projectPath.trim()) {
      await registerProject(projectPath.trim());
    }

    const ok = await upsert(
      {
        app: effectiveApp,
        scope,
        projectPath: scope === "project" ? projectPath.trim() : null,
        name: name.trim(),
      },
      config,
    );
    if (ok) onClose();
  }

  const transportKind: TransportKind = effectiveTransport;

  return (
    <Modal
      open={isOpen}
      onClose={onClose}
      title={editing ? `Editar ${initial?.name}` : "Agregar MCP"}
      subtitle={
        editing
          ? "Editás comando, args y transporte. Los valores de env se gestionan en Fase 4."
          : "Instalá un MCP server en una de tus apps de Claude."
      }
      width={560}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            Cancelar
          </Button>
          <Button onClick={save} disabled={!canSave || busy}>
            {editing ? "Guardar cambios" : "Agregar MCP"}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <Input
          label="Nombre"
          value={name}
          disabled={editing}
          placeholder="context7"
          mono
          onChange={(e) => setName(e.currentTarget.value)}
          help={editing ? "El nombre identifica la entrada y no se cambia acá." : undefined}
        />

        <div className="flex flex-col gap-1.5">
          <span className="text-[12.5px] font-medium text-ink-soft">Scope</span>
          <SegmentedControl
            value={scope}
            onChange={(v) => setScope(v)}
            options={[
              { value: "user" as Scope, label: "Global (user)" },
              { value: "project" as Scope, label: "Proyecto" },
            ]}
          />
        </div>

        {scope === "user" ? (
          <div className="flex flex-col gap-1.5">
            <span className="text-[12.5px] font-medium text-ink-soft">App</span>
            <Select
              value={app}
              onChange={(v) => setApp(v)}
              options={[
                { value: "claude-desktop" as AppId, label: "Claude Desktop" },
                { value: "claude-code" as AppId, label: "Claude Code" },
              ]}
            />
          </div>
        ) : (
          <div className="flex flex-col gap-1.5">
            <span className="text-[12.5px] font-medium text-ink-soft">
              Repositorio (.mcp.json)
            </span>
            <div className="flex gap-2">
              <Input
                value={projectPath}
                mono
                placeholder="/ruta/al/repo"
                onChange={(e) => setProjectPath(e.currentTarget.value)}
              />
              <Button variant="ghost" onClick={pickFolder}>
                Elegir…
              </Button>
            </div>
          </div>
        )}

        <div className="flex flex-col gap-1.5">
          <span className="text-[12.5px] font-medium text-ink-soft">
            Transporte
          </span>
          <SegmentedControl
            value={effectiveTransport}
            onChange={(v) => setTransport(v)}
            options={transportOptions}
          />
          {desktopOnlyStdio && (
            <span className="text-[11.5px] text-faint">
              Claude Desktop solo soporta MCPs stdio.
            </span>
          )}
        </div>

        {transportKind === "stdio" ? (
          <>
            <Input
              label="Comando"
              value={command}
              mono
              placeholder="npx"
              onChange={(e) => setCommand(e.currentTarget.value)}
            />
            <Textarea
              label="Argumentos (uno por línea)"
              value={argsText}
              rows={3}
              placeholder={"-y\n@upstash/context7-mcp"}
              onChange={(e) => setArgsText(e.currentTarget.value)}
            />
            {!editing && (
              <div className="flex flex-col gap-1.5">
                <span className="text-[12.5px] font-medium text-ink-soft">
                  Variables de entorno
                </span>
                <KeyValueEditor entries={env} onChange={setEnv} />
              </div>
            )}
            {editing && initial && initial.envKeys.length > 0 && (
              <div className="flex flex-col gap-1.5">
                <span className="text-[12.5px] font-medium text-ink-soft">
                  Variables de entorno
                </span>
                <div className="flex flex-wrap gap-1.5">
                  {initial.envKeys.map((k) => (
                    <span
                      key={k}
                      className="rounded-full px-2 py-0.5 font-mono text-[11px] text-ink-soft"
                      style={{ background: "var(--color-surface-3)" }}
                    >
                      {k}
                    </span>
                  ))}
                </div>
                <span className="text-[11.5px] text-faint">
                  Se preservan tal cual. La edición de valores llega en Fase 4
                  (keychain).
                </span>
              </div>
            )}
          </>
        ) : (
          <Input
            label="URL"
            value={url}
            mono
            placeholder="https://mcp.ejemplo.com/mcp"
            onChange={(e) => setUrl(e.currentTarget.value)}
          />
        )}
      </div>
    </Modal>
  );
}
