import { Plus, Trash2 } from "lucide-react";
import { Input } from "./Input";

export interface KeyValue {
  key: string;
  value: string;
}

interface KeyValueEditorProps {
  entries: KeyValue[];
  onChange: (entries: KeyValue[]) => void;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
}

/**
 * Editor de pares clave/valor (para `env`). En Fase 3 los valores van en
 * texto plano al config; el vault en keychain llega en Fase 4.
 */
export function KeyValueEditor({
  entries,
  onChange,
  keyPlaceholder = "CLAVE",
  valuePlaceholder = "valor",
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
              value={e.value}
              placeholder={valuePlaceholder}
              onChange={(ev) => update(i, { value: ev.currentTarget.value })}
            />
          </div>
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
