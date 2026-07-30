import { useEffect, useMemo, useState } from "react";
import {
  KeyRound,
  Plus,
  Eye,
  EyeOff,
  Pencil,
  Trash2,
  Lock,
  LockOpen,
  ChevronRight,
  AlertTriangle,
} from "lucide-react";
import { ScreenShell } from "./ScreenShell";
import { Card, Button, Input, Modal } from "../components";
import { useVault } from "../store/vault";
import { useInventory } from "../store/inventory";
import { useMutations } from "../store/mutations";
import { aggregateEnvs, type EnvAggregate, type EnvUsage } from "../lib/envAggregate";
import { applyEnvEdits, lockCollisions, seedRow } from "../lib/envEditing";
import { APP_LABEL } from "../types/inventory";

type Dialog =
  | { kind: "create" }
  | { kind: "editSecret"; name: string }
  | { kind: "editUsage"; envKey: string; usage: EnvUsage; isVault: boolean }
  | { kind: "deleteSecret"; name: string; usedBy: number }
  | null;

export function EnvSecretsScreen() {
  const { status, secrets, error, mocked, load } = useVault();
  const inventory = useInventory((s) => s.inventory);
  const loadInventory = useInventory((s) => s.load);
  const inventoryStatus = useInventory((s) => s.status);
  const [dialog, setDialog] = useState<Dialog>(null);

  useEffect(() => {
    if (status === "idle") load();
    if (inventoryStatus === "idle") loadInventory();
  }, [status, load, inventoryStatus, loadInventory]);

  const aggregates = useMemo(
    () => aggregateEnvs(inventory?.installations ?? [], secrets),
    [inventory, secrets],
  );

  return (
    <ScreenShell
      title="Env & Secrets"
      subtitle="Todas las variables de entorno de tus MCPs. Los secretos (candado) viven en el keychain del OS y se inyectan al escribir; nunca en texto plano fuera del config."
      actions={
        <Button onClick={() => setDialog({ kind: "create" })}>
          <Plus size={16} strokeWidth={2.5} />
          Nuevo secreto
        </Button>
      }
    >
      {status === "loading" && (
        <div className="flex flex-col gap-2.5">
          {[0, 1].map((i) => (
            <div
              key={i}
              className="ds-card h-16 animate-pulse !rounded-[var(--radius-md)]"
              style={{ opacity: 1 - i * 0.2 }}
            />
          ))}
        </div>
      )}

      {status === "error" && (
        <div className="ds-card flex flex-col items-center gap-3 px-6 py-12 text-center">
          <AlertTriangle size={26} style={{ color: "var(--state-danger-text)" }} />
          <h2 className="text-[16px] font-semibold text-ink">
            No se pudo acceder al vault
          </h2>
          <p className="max-w-md font-mono text-[12px] text-muted">
            {error ?? "El keychain del OS no está disponible."}
          </p>
        </div>
      )}

      {status === "ready" && aggregates.length === 0 && (
        <div className="ds-card flex flex-col items-center gap-3 px-6 py-14 text-center">
          <div
            className="flex h-14 w-14 items-center justify-center rounded-[18px] border border-[var(--line)]"
            style={{ background: "var(--accent-soft)" }}
          >
            <KeyRound size={26} style={{ color: "var(--accent-strong)" }} />
          </div>
          <h2 className="text-[16px] font-semibold text-ink">Sin variables</h2>
          <p className="max-w-sm text-[13px] leading-relaxed text-muted">
            Agregá variables de entorno desde el formulario de un MCP, o creá un
            secreto compartido acá y vinculalo donde lo necesites.
          </p>
        </div>
      )}

      {status === "ready" && aggregates.length > 0 && (
        <Card>
          <div className="flex flex-col divide-y divide-[var(--line)]">
            {aggregates.map((agg) => (
              <EnvGroupRow
                key={agg.key}
                agg={agg}
                onEditUsage={(usage, isVault) =>
                  setDialog({
                    kind: "editUsage",
                    envKey: agg.key,
                    usage,
                    isVault,
                  })
                }
                onEditSecret={() =>
                  setDialog({ kind: "editSecret", name: agg.key })
                }
                onDeleteSecret={() =>
                  setDialog({ kind: "deleteSecret", name: agg.key, usedBy: 0 })
                }
              />
            ))}
          </div>
        </Card>
      )}

      {mocked && (
        <p className="mt-6 text-center text-[12px] text-faint">
          Datos de muestra (modo browser). El vault real usa el keychain del OS.
        </p>
      )}

      {dialog?.kind === "create" && (
        <SecretFormModal initial={null} onClose={() => setDialog(null)} />
      )}
      {dialog?.kind === "editSecret" && (
        <SecretFormModal initial={dialog.name} onClose={() => setDialog(null)} />
      )}
      {dialog?.kind === "editUsage" && (
        <EnvUsageEditModal
          envKey={dialog.envKey}
          usage={dialog.usage}
          isVault={dialog.isVault}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog?.kind === "deleteSecret" && (
        <DeleteSecretModal
          name={dialog.name}
          usedBy={dialog.usedBy}
          onClose={() => setDialog(null)}
        />
      )}
    </ScreenShell>
  );
}

function Badge({
  children,
  accent = false,
}: {
  children: React.ReactNode;
  accent?: boolean;
}) {
  return (
    <span
      className="rounded-full px-2 py-0.5 text-[10.5px] font-medium"
      style={
        accent
          ? { background: "var(--accent-soft)", color: "var(--accent-strong)" }
          : { background: "var(--color-surface-3)", color: "var(--color-ink-soft)" }
      }
    >
      {children}
    </span>
  );
}

function EnvGroupRow({
  agg,
  onEditUsage,
  onEditSecret,
  onDeleteSecret,
}: {
  agg: EnvAggregate;
  onEditUsage: (usage: EnvUsage, isVault: boolean) => void;
  onEditSecret: () => void;
  onDeleteSecret: () => void;
}) {
  const [expanded, setExpanded] = useState(false);

  const badge = agg.unused ? (
    <Badge>Sin usar</Badge>
  ) : agg.shared ? (
    <Badge accent>Compartido · {agg.usages.length} MCPs</Badge>
  ) : (
    <Badge>Privado · 1 MCP</Badge>
  );

  return (
    <div className="py-3 first:pt-0 last:pb-0">
      <div className="flex items-center gap-3">
        <button
          onClick={() => !agg.unused && setExpanded((e) => !e)}
          disabled={agg.unused}
          className="flex h-9 w-9 shrink-0 items-center justify-center rounded-[10px] border border-[var(--line)] transition-colors duration-[var(--dur)] disabled:opacity-60"
          style={{ background: "var(--color-surface-2)" }}
          aria-label={expanded ? "Colapsar" : "Expandir"}
        >
          {agg.isSecret ? (
            <Lock size={14} style={{ color: "var(--accent-strong)" }} />
          ) : (
            <LockOpen size={14} className="text-ink-soft" />
          )}
        </button>

        <button
          onClick={() => !agg.unused && setExpanded((e) => !e)}
          disabled={agg.unused}
          className="flex min-w-0 flex-1 items-center gap-2 text-left"
        >
          <span className="truncate font-mono text-[13px] font-semibold text-ink">
            {agg.key}
          </span>
          {badge}
          {!agg.unused && (
            <ChevronRight
              size={15}
              className="ml-auto shrink-0 text-faint transition-transform duration-[var(--dur)]"
              style={{ transform: expanded ? "rotate(90deg)" : "none" }}
            />
          )}
        </button>

        {agg.unused && (
          <>
            <RowIcon label="Editar valor" onClick={onEditSecret}>
              <Pencil size={15} />
            </RowIcon>
            <RowIcon label="Eliminar" danger onClick={onDeleteSecret}>
              <Trash2 size={15} />
            </RowIcon>
          </>
        )}
      </div>

      {expanded && !agg.unused && (
        <div className="mt-2 flex flex-col gap-1 pl-12">
          {agg.usages.map((u, i) => (
            <div
              key={`${u.mcpName}-${u.app}-${u.scope}-${i}`}
              className="flex items-center gap-2 rounded-[10px] px-2.5 py-1.5"
              style={{ background: "var(--color-surface-2)" }}
            >
              <span className="min-w-0 flex-1 truncate text-[12px] text-ink-soft">
                <span className="font-medium text-ink">{u.mcpName}</span>
                <span className="text-faint">
                  {" · "}
                  {APP_LABEL[u.app]} · {u.scope === "project" ? "proyecto" : "global"}
                </span>
              </span>
              {u.isVault ? (
                <Lock size={13} style={{ color: "var(--accent-strong)" }} />
              ) : (
                <LockOpen size={13} className="text-faint" />
              )}
              <RowIcon label="Editar" onClick={() => onEditUsage(u, u.isVault)}>
                <Pencil size={14} />
              </RowIcon>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

/** Edita una env var puntual en UN MCP: valor, candado (vault) y quitar. */
function EnvUsageEditModal({
  envKey,
  usage,
  isVault,
  onClose,
}: {
  envKey: string;
  usage: EnvUsage;
  isVault: boolean;
  onClose: () => void;
}) {
  const reveal = useVault((s) => s.reveal);
  const secrets = useVault((s) => s.secrets);
  const readEnvValue = useMutations((s) => s.readEnvValue);
  const [value, setValue] = useState("");
  const [loaded, setLoaded] = useState(false);
  const [secret, setSecret] = useState(isVault);
  const [show, setShow] = useState(false);
  const [busy, setBusy] = useState(false);

  async function doReveal() {
    const v = isVault
      ? await reveal(envKey)
      : await readEnvValue(usage.target, envKey);
    if (v !== null) {
      setValue(v);
      setLoaded(true);
      setShow(true);
    }
  }

  const editedRow = {
    key: envKey,
    value,
    secret,
    originalKey: envKey,
    originalSecret: isVault,
    loaded,
    dirty: loaded,
  };
  const collisions = lockCollisions([editedRow], secrets);

  async function save() {
    setBusy(true);
    const ok = await applyEnvEdits(
      usage.target,
      [seedRow(envKey, isVault)],
      [editedRow],
    );
    setBusy(false);
    if (ok) onClose();
  }

  async function removeFromMcp() {
    setBusy(true);
    const ok = await applyEnvEdits(usage.target, [seedRow(envKey, isVault)], []);
    setBusy(false);
    if (ok) onClose();
  }

  return (
    <Modal
      open
      onClose={onClose}
      title={envKey}
      subtitle={`En ${usage.mcpName} · ${APP_LABEL[usage.app]} · ${
        usage.scope === "project" ? "proyecto" : "global"
      }`}
      width={480}
      footer={
        <>
          <Button
            variant="danger"
            onClick={removeFromMcp}
            disabled={busy}
            style={{ marginRight: "auto" }}
          >
            Quitar de este MCP
          </Button>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            Cancelar
          </Button>
          <Button onClick={save} disabled={busy || (!loaded && secret === isVault)}>
            Guardar
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <div className="flex items-end gap-2">
          <div className="flex-1">
            <Input
              label="Valor"
              value={loaded ? value : ""}
              mono
              type={show && !secret ? "text" : "password"}
              disabled={!loaded}
              placeholder={loaded ? "" : "•••••••• (revelar para editar)"}
              onChange={(e) => {
                setValue(e.currentTarget.value);
                setLoaded(true);
              }}
            />
          </div>
          {loaded ? (
            <Button variant="ghost" onClick={() => setShow((s) => !s)}>
              {show ? <EyeOff size={15} /> : <Eye size={15} />}
            </Button>
          ) : (
            <Button variant="ghost" onClick={doReveal}>
              <Eye size={15} />
            </Button>
          )}
        </div>

        <button
          onClick={() => setSecret((s) => !s)}
          className="flex items-center gap-2 self-start rounded-[10px] px-2.5 py-1.5 text-[12.5px] font-medium transition-colors duration-[var(--dur)]"
          style={
            secret
              ? { color: "var(--accent-strong)", background: "var(--accent-soft)" }
              : { color: "var(--color-muted)", background: "var(--color-surface-2)" }
          }
        >
          {secret ? <Lock size={15} /> : <LockOpen size={15} />}
          {secret ? "Secreto (en el keychain)" : "Inline (texto plano en el config)"}
        </button>

        {collisions.length > 0 && (
          <div
            className="flex items-start gap-2 rounded-[var(--radius-sm)] px-2.5 py-2"
            style={{ background: "var(--state-warn-soft)" }}
          >
            <AlertTriangle
              size={14}
              className="mt-0.5 shrink-0"
              style={{ color: "var(--state-warn-text)" }}
            />
            <span className="text-[11.5px] leading-relaxed text-ink-soft">
              {envKey} ya existe en el vault (usado por {collisions[0].usedBy} MCP
              {collisions[0].usedBy === 1 ? "" : "s"}). Guardar rotará ese secreto
              compartido para todos.
            </span>
          </div>
        )}
      </div>
    </Modal>
  );
}

function SecretFormModal({
  initial,
  onClose,
}: {
  initial: string | null;
  onClose: () => void;
}) {
  const editing = !!initial;
  const setSecret = useVault((s) => s.setSecret);
  const busy = useVault((s) => s.busy);
  const [name, setName] = useState(initial ?? "");
  const [value, setValue] = useState("");
  const [show, setShow] = useState(false);

  return (
    <Modal
      open
      onClose={onClose}
      title={editing ? `Actualizar ${initial}` : "Nuevo secreto"}
      subtitle={
        editing
          ? "El nuevo valor se re-inyecta en los MCPs vinculados a este secreto."
          : "Se guarda en el keychain del OS. Vinculalo a un MCP desde su formulario."
      }
      width={460}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            Cancelar
          </Button>
          <Button
            disabled={busy || name.trim() === "" || value === ""}
            onClick={async () => {
              const ok = await setSecret(name.trim(), value);
              if (ok) onClose();
            }}
          >
            {editing ? "Guardar valor" : "Crear secreto"}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <Input
          label="Nombre"
          value={name}
          disabled={editing}
          mono
          placeholder="NOTION_TOKEN"
          onChange={(e) => setName(e.currentTarget.value)}
          help={editing ? undefined : "Se usa como clave de env al vincular."}
        />
        <div className="flex items-end gap-2">
          <div className="flex-1">
            <Input
              label="Valor"
              value={value}
              mono
              type={show ? "text" : "password"}
              placeholder="••••••••"
              onChange={(e) => setValue(e.currentTarget.value)}
            />
          </div>
          <Button variant="ghost" onClick={() => setShow((s) => !s)}>
            {show ? <EyeOff size={15} /> : <Eye size={15} />}
          </Button>
        </div>
      </div>
    </Modal>
  );
}

function DeleteSecretModal({
  name,
  usedBy,
  onClose,
}: {
  name: string;
  usedBy: number;
  onClose: () => void;
}) {
  const deleteSecret = useVault((s) => s.deleteSecret);
  const busy = useVault((s) => s.busy);
  return (
    <Modal
      open
      onClose={onClose}
      title={`Eliminar ${name}`}
      width={440}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            Cancelar
          </Button>
          <Button
            variant="danger"
            disabled={busy}
            onClick={async () => {
              const ok = await deleteSecret(name);
              if (ok) onClose();
            }}
          >
            Eliminar
          </Button>
        </>
      }
    >
      <p className="text-[13px] leading-relaxed text-muted">
        Se borra del keychain
        {usedBy > 0
          ? ` y se quita de los ${usedBy} MCP${usedBy === 1 ? "" : "s"} vinculado${
              usedBy === 1 ? "" : "s"
            } (se limpia esa variable de cada config, con backup).`
          : " (no está vinculado a ningún MCP)."}
      </p>
    </Modal>
  );
}

function RowIcon({
  children,
  onClick,
  label,
  danger = false,
}: {
  children: React.ReactNode;
  onClick: () => void;
  label: string;
  danger?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      aria-label={label}
      title={label}
      className="flex h-8 w-8 items-center justify-center rounded-[10px] text-muted transition-colors duration-[var(--dur)] hover:text-ink"
      style={danger ? { color: "var(--state-danger-text)" } : undefined}
      onMouseEnter={(e) =>
        (e.currentTarget.style.background = danger
          ? "var(--state-danger-soft)"
          : "rgba(255,255,255,0.06)")
      }
      onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
    >
      {children}
    </button>
  );
}
