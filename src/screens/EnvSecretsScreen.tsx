import { useEffect, useState } from "react";
import { KeyRound, Plus, Eye, EyeOff, Pencil, Trash2, AlertTriangle } from "lucide-react";
import { ScreenShell } from "./ScreenShell";
import { Card, Button, Input, Modal } from "../components";
import { useVault } from "../store/vault";
import type { VaultSecretInfo } from "../types/inventory";

type Dialog =
  | { kind: "create" }
  | { kind: "edit"; secret: VaultSecretInfo }
  | { kind: "delete"; secret: VaultSecretInfo }
  | null;

export function EnvSecretsScreen() {
  const { status, secrets, error, mocked, load } = useVault();
  const [dialog, setDialog] = useState<Dialog>(null);

  useEffect(() => {
    if (status === "idle") load();
  }, [status, load]);

  return (
    <ScreenShell
      title="Env & Secrets"
      subtitle="Vault de variables compartidas en el keychain del OS. Se inyectan a los MCPs vinculados al escribir; nunca se guardan en texto plano fuera del config."
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

      {status === "ready" && secrets.length === 0 && (
        <div className="ds-card flex flex-col items-center gap-3 px-6 py-14 text-center">
          <div
            className="flex h-14 w-14 items-center justify-center rounded-[18px] border border-[var(--line)]"
            style={{ background: "var(--accent-soft)" }}
          >
            <KeyRound size={26} style={{ color: "var(--accent-strong)" }} />
          </div>
          <h2 className="text-[16px] font-semibold text-ink">Vault vacío</h2>
          <p className="max-w-sm text-[13px] leading-relaxed text-muted">
            Creá un secreto compartido (ej. NOTION_TOKEN) y vinculalo a los MCPs
            que lo necesiten desde el formulario del MCP.
          </p>
        </div>
      )}

      {status === "ready" && secrets.length > 0 && (
        <Card>
          <div className="flex flex-col divide-y divide-[var(--line)]">
            {secrets.map((s) => (
              <SecretRow
                key={s.name}
                secret={s}
                onEdit={() => setDialog({ kind: "edit", secret: s })}
                onDelete={() => setDialog({ kind: "delete", secret: s })}
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

      {(dialog?.kind === "create" || dialog?.kind === "edit") && (
        <SecretFormModal
          initial={dialog.kind === "edit" ? dialog.secret : null}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog?.kind === "delete" && (
        <DeleteSecretModal secret={dialog.secret} onClose={() => setDialog(null)} />
      )}
    </ScreenShell>
  );
}

function SecretRow({
  secret,
  onEdit,
  onDelete,
}: {
  secret: VaultSecretInfo;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const reveal = useVault((s) => s.reveal);
  const [shown, setShown] = useState<string | null>(null);

  async function toggle() {
    if (shown !== null) {
      setShown(null);
      return;
    }
    const v = await reveal(secret.name);
    if (v !== null) setShown(v);
  }

  return (
    <div className="flex items-center gap-3 py-3 first:pt-0 last:pb-0">
      <div
        className="flex h-9 w-9 shrink-0 items-center justify-center rounded-[10px] border border-[var(--line)]"
        style={{ background: "var(--color-surface-2)" }}
      >
        <KeyRound size={15} className="text-ink-soft" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="font-mono text-[13px] font-semibold text-ink">
            {secret.name}
          </span>
          <span
            className="rounded-full px-2 py-0.5 text-[10.5px] font-medium text-ink-soft"
            style={{ background: "var(--color-surface-3)" }}
          >
            {secret.usedBy} MCP{secret.usedBy === 1 ? "" : "s"}
          </span>
        </div>
        <div className="mt-0.5 font-mono text-[12px] text-faint">
          {shown ?? "••••••••••••"}
        </div>
      </div>
      <RowIcon label={shown ? "Ocultar" : "Revelar"} onClick={toggle}>
        {shown ? <EyeOff size={15} /> : <Eye size={15} />}
      </RowIcon>
      <RowIcon label="Editar valor" onClick={onEdit}>
        <Pencil size={15} />
      </RowIcon>
      <RowIcon label="Eliminar" danger onClick={onDelete}>
        <Trash2 size={15} />
      </RowIcon>
    </div>
  );
}

function SecretFormModal({
  initial,
  onClose,
}: {
  initial: VaultSecretInfo | null;
  onClose: () => void;
}) {
  const editing = !!initial;
  const setSecret = useVault((s) => s.setSecret);
  const busy = useVault((s) => s.busy);
  const [name, setName] = useState(initial?.name ?? "");
  const [value, setValue] = useState("");
  const [show, setShow] = useState(false);

  return (
    <Modal
      open
      onClose={onClose}
      title={editing ? `Actualizar ${initial?.name}` : "Nuevo secreto"}
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
  secret,
  onClose,
}: {
  secret: VaultSecretInfo;
  onClose: () => void;
}) {
  const deleteSecret = useVault((s) => s.deleteSecret);
  const busy = useVault((s) => s.busy);
  return (
    <Modal
      open
      onClose={onClose}
      title={`Eliminar ${secret.name}`}
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
              const ok = await deleteSecret(secret.name);
              if (ok) onClose();
            }}
          >
            Eliminar
          </Button>
        </>
      }
    >
      <p className="text-[13px] leading-relaxed text-muted">
        Se borra del keychain y se quita de los {secret.usedBy} MCP
        {secret.usedBy === 1 ? "" : "s"} vinculado
        {secret.usedBy === 1 ? "" : "s"} (se limpia esa variable de cada config,
        con backup).
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
