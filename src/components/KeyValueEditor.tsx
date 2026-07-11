import { Plus, Trash2, Lock, LockOpen } from "lucide-react";
import { Input } from "./Input";

export interface KeyValue {
  key: string;
  value: string;
  /** Si es secreto, se guarda en el vault (keychain) y se enmascara. */
  secret?: boolean;
}

interface KeyValueEditorProps {
  entries: KeyValue[];
  onChange: (entries: KeyValue[]) => void;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
  /** Habilita el toggle de "secreto" por fila (vault). */
  allowSecret?: boolean;
}

/**
 * Editor de pares clave/valor (para `env`). Las filas marcadas como secreto
 * se guardan en el vault del keychain al guardar el MCP (no en el estado ni en
 * el config en texto plano si es gestionado por el vault).
 */
export function KeyValueEditor({
  entries,
  onChange,
  keyPlaceholder = "CLAVE",
  valuePlaceholder = "valor",
  allowSecret = false,
}: KeyValueEditorProps) {
  const update = (i: number, patch: Partial<KeyValue>) =>
    onChange(entries.map((e, idx) => (idx === i ? { ...e, ...patch } : e)));

  const remove = (i: number) =>
    onChange(entries.filter((_, idx) => idx !== i));

  const add = () => onChange([...entries, { key: "", value: "" }]);

  return (
    <div className="flex flex-col gap-2">
      {entries.length === 0 && (
        <p className="text-[12px] text-faint">Sin variables de entorno.</p>
      )}
      {entries.map((e, i) => (
        <div key={i} className="flex items-center gap-2">
          <div className="w-2/5">
            <Input
              mono
              value={e.key}
              placeholder={keyPlaceholder}
              onChange={(ev) => update(i, { key: ev.currentTarget.value })}
            />
          </div>
          <div className="flex-1">
            <Input
              mono
              type={e.secret ? "password" : "text"}
              value={e.value}
              placeholder={e.secret ? "•••• (al keychain)" : valuePlaceholder}
              onChange={(ev) => update(i, { value: ev.currentTarget.value })}
            />
          </div>
          {allowSecret && (
            <button
              onClick={() => update(i, { secret: !e.secret })}
              aria-label={e.secret ? "Quitar secreto" : "Marcar secreto"}
              title={
                e.secret
                  ? "Secreto (se guarda en el keychain)"
                  : "Marcar como secreto"
              }
              className="flex h-9 w-9 shrink-0 items-center justify-center rounded-[10px] transition-colors duration-[var(--dur)]"
              style={
                e.secret
                  ? { color: "var(--accent-strong)", background: "var(--accent-soft)" }
                  : { color: "var(--color-muted)" }
              }
            >
              {e.secret ? <Lock size={15} /> : <LockOpen size={15} />}
            </button>
          )}
          <button
            onClick={() => remove(i)}
            aria-label="Quitar variable"
            className="flex h-9 w-9 shrink-0 items-center justify-center rounded-[10px] text-muted transition-colors duration-[var(--dur)] hover:bg-[var(--state-danger-soft)] hover:text-[var(--state-danger-text)]"
          >
            <Trash2 size={15} />
          </button>
        </div>
      ))}
      <button
        onClick={add}
        className="mt-1 inline-flex w-fit items-center gap-1.5 rounded-[10px] border border-[var(--line)] px-2.5 py-1.5 text-[12px] font-medium text-muted transition-colors duration-[var(--dur)] hover:border-[var(--line-strong)] hover:text-ink"
        style={{ background: "var(--color-surface-2)" }}
      >
        <Plus size={14} /> Agregar variable
      </button>
    </div>
  );
}
