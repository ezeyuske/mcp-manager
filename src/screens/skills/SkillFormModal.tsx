import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Modal,
  Button,
  Input,
  Textarea,
  SegmentedControl,
} from "../../components";
import { useSkills } from "../../store/skills";
import { isTauri } from "../../lib/tauri";
import { useToast } from "../../store/toast";
import type { Skill, SkillInput } from "../../types/skills";
import type { Scope } from "../../types/inventory";

interface SkillFormModalProps {
  open: boolean;
  onClose: () => void;
  /** Si viene, el modal está en modo edición (scope/projectPath/name fijos). */
  initial?: Skill | null;
}

export function SkillFormModal({
  open: isOpen,
  onClose,
  initial,
}: SkillFormModalProps) {
  const editing = !!initial;
  const upsert = useSkills((s) => s.upsert);
  const busy = useSkills((s) => s.busy);
  const pushToast = useToast((s) => s.push);

  const [name, setName] = useState(initial?.name ?? "");
  const [description, setDescription] = useState(initial?.description ?? "");
  const [version, setVersion] = useState(initial?.version ?? "");
  const [scope, setScope] = useState<Scope>(initial?.scope ?? "user");
  const [projectPath, setProjectPath] = useState(initial?.projectPath ?? "");
  // El body no viaja al frontend; en edición se deja vacío para preservarlo.
  const [body, setBody] = useState("");

  const canSave =
    name.trim() !== "" &&
    description.trim() !== "" &&
    (scope === "user" || projectPath.trim() !== "");

  async function pickFolder() {
    if (!isTauri()) {
      pushToast("info", "El selector de carpetas requiere la app (no browser).");
      return;
    }
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === "string") setProjectPath(picked);
  }

  async function save() {
    const input: SkillInput = {
      scope,
      projectPath: scope === "project" ? projectPath.trim() : null,
      name: name.trim(),
      description: description.trim(),
      version: version.trim() || null,
      body,
    };
    const ok = await upsert(input);
    if (ok) onClose();
  }

  return (
    <Modal
      open={isOpen}
      onClose={onClose}
      title={editing ? `Editar ${initial?.name}` : "Nueva skill"}
      subtitle={
        editing
          ? "Editás metadatos y opcionalmente el contenido. La identidad de scope y nombre no cambia."
          : "Creá una skill (carpeta con SKILL.md) en el scope elegido."
      }
      width={560}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            Cancelar
          </Button>
          <Button onClick={save} disabled={!canSave || busy}>
            {editing ? "Guardar cambios" : "Crear skill"}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <Input
          label="Nombre"
          value={name}
          disabled={editing}
          placeholder="mi-skill"
          mono
          onChange={(e) => setName(e.currentTarget.value)}
          help={
            editing
              ? "El nombre se cambia con la acción Renombrar de la lista."
              : undefined
          }
        />

        <Textarea
          label="Descripción"
          value={description}
          rows={3}
          placeholder="Qué hace la skill y cuándo usarla."
          onChange={(e) => setDescription(e.currentTarget.value)}
        />

        <Input
          label="Versión (opcional)"
          value={version ?? ""}
          mono
          placeholder="1.0.0"
          onChange={(e) => setVersion(e.currentTarget.value)}
        />

        <div className="flex flex-col gap-1.5">
          <span className="text-[12.5px] font-medium text-ink-soft">Scope</span>
          {editing ? (
            <span
              className="inline-flex w-fit items-center rounded-full px-3 py-1 text-[12.5px] font-medium text-ink-soft"
              style={{ background: "var(--color-surface-3)" }}
            >
              {scope === "user" ? "Global (user)" : "Proyecto"}
            </span>
          ) : (
            <SegmentedControl
              value={scope}
              onChange={(v) => setScope(v)}
              options={[
                { value: "user" as Scope, label: "Global (user)" },
                { value: "project" as Scope, label: "Proyecto" },
              ]}
            />
          )}
        </div>

        {scope === "project" && (
          <div className="flex flex-col gap-1.5">
            <span className="text-[12.5px] font-medium text-ink-soft">
              Repositorio
            </span>
            {editing ? (
              <span className="truncate font-mono text-[12px] text-faint">
                {projectPath || "—"}
              </span>
            ) : (
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
            )}
          </div>
        )}

        <Textarea
          label="Contenido (markdown, opcional)"
          value={body}
          rows={6}
          placeholder={
            editing
              ? "Dejar vacío preserva el contenido actual."
              : "# Mi skill\n\nInstrucciones en markdown…"
          }
          onChange={(e) => setBody(e.currentTarget.value)}
          help={
            editing
              ? "Dejar vacío preserva el contenido actual del SKILL.md."
              : undefined
          }
        />
      </div>
    </Modal>
  );
}
