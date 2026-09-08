import { useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { AlertTriangle } from "lucide-react";
import {
  Modal,
  Button,
  Input,
  Textarea,
  Select,
  SegmentedControl,
  KeyValueEditor,
} from "../../components";
import { useMutations } from "../../store/mutations";
import { useVault } from "../../store/vault";
import { isTauri } from "../../lib/tauri";
import { useToast } from "../../store/toast";
import {
  applyEnvEdits,
  lockCollisions,
  seedRow,
  type EnvRow,
} from "../../lib/envEditing";
import {
  targetOf,
  type AppId,
  type McpInstallation,
  type McpServerConfigInput,
  type Scope,
  type TransportKind,
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
  const readEnvValue = useMutations((s) => s.readEnvValue);
  const setSecret = useVault((s) => s.setSecret);
  const bindSecret = useVault((s) => s.bind);
  const reveal = useVault((s) => s.reveal);
  const secrets = useVault((s) => s.secrets);
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

  // Target original (donde vive el env actual): identidad fija en edición,
  // independiente de cambios de scope/app en el form.
  const originalTarget = useMemo(
    () => (initial ? targetOf(initial) : null),
    [initial],
  );
  // Snapshot del env al abrir (para el diff). Filas existentes arrancan
  // enmascaradas (valor on-demand); el candado refleja si está en el vault.
  const initialEnvRows = useMemo<EnvRow[]>(
    () =>
      initial
        ? initial.envKeys.map((k) => seedRow(k, initial.vaultKeys.includes(k)))
        : [],
    [initial],
  );
  const [env, setEnv] = useState<EnvRow[]>(() =>
    initialEnvRows.map((r) => ({ ...r })),
  );

  const collisions = lockCollisions(env, secrets);

  async function onRevealRow(i: number): Promise<string | null> {
    const row = env[i];
    if (!row.originalKey || !originalTarget) return row.value;
    return row.originalSecret
      ? reveal(row.originalKey)
      : readEnvValue(originalTarget, row.originalKey);
  }

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

    const envRows = env.filter((e) => e.key.trim() !== "");

    const config: McpServerConfigInput = {
      args: isStdio ? args : [],
      // ADD: env inline (no secreto) desde el editor. EDIT: vacío ⇒ el backend
      // preserva el env existente y luego lo editamos quirúrgicamente.
      env:
        !editing && isStdio
          ? Object.fromEntries(
              envRows
                .filter((e) => !e.secret)
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

    const target = {
      app: effectiveApp,
      scope,
      projectPath: scope === "project" ? projectPath.trim() : null,
      name: name.trim(),
    };

    const ok = await upsert(target, config);
    if (!ok) return;

    if (editing) {
      // EDIT: aplicar el diff de env quirúrgicamente (add/edit/remove/lock).
      // Usa el target ORIGINAL: ahí vive el env, sin importar cambios de form.
      const envOk = await applyEnvEdits(
        originalTarget ?? target,
        initialEnvRows,
        env,
      );
      if (!envOk) return;
    } else {
      // ADD: los secretos se guardan en el vault y se vinculan tras crear el
      // MCP (mínimo privilegio: binding solo para este target; nombre = clave).
      const secretRows = envRows.filter((e) => e.secret && e.value !== "");
      for (const row of secretRows) {
        const key = row.key.trim();
        const created = await setSecret(key, row.value);
        if (created) await bindSecret(target, key, key);
      }
    }

    onClose();
  }

  const transportKind: TransportKind = effectiveTransport;

  return (
    <Modal
      open={isOpen}
      onClose={onClose}
      title={editing ? `Editar ${initial?.name}` : "Agregar MCP"}
      subtitle={
        editing
          ? "Editás comando, args, transporte y variables de entorno (incluidos secretos)."
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
          help={
            editing
              ? "El nombre se cambia con la acción Renombrar de la lista."
              : undefined
          }
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
            <div className="flex flex-col gap-1.5">
              <span className="text-[12.5px] font-medium text-ink-soft">
                Variables de entorno
              </span>
              <KeyValueEditor
                entries={env}
                onChange={(rows) => setEnv(rows)}
                allowSecret
                onReveal={editing ? onRevealRow : undefined}
              />
              <span className="text-[11.5px] text-faint">
                El candado guarda el valor en el keychain del OS (vault) y lo
                inyecta al escribir; no queda en el estado de la app.
                {editing &&
                  " Los valores existentes se revelan bajo demanda con el ojo."}
              </span>
              {collisions.length > 0 && (
                <div
                  className="mt-1 flex items-start gap-2 rounded-[var(--radius-sm)] px-2.5 py-2"
                  style={{ background: "var(--state-warn-soft)" }}
                >
                  <AlertTriangle
                    size={14}
                    className="mt-0.5 shrink-0"
                    style={{ color: "var(--state-warn-text)" }}
                  />
                  <span className="text-[11.5px] leading-relaxed text-ink-soft">
                    {collisions.map((c) => c.name).join(", ")} ya{" "}
                    {collisions.length === 1 ? "existe" : "existen"} en el vault.
                    Guardar rotará ese secreto compartido para todos sus MCPs (
                    {collisions.map((c) => `${c.name}: ${c.usedBy}`).join(", ")}).
                  </span>
                </div>
              )}
            </div>
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
