import { useEffect, useMemo, useState, type ReactNode } from "react";
import { Sparkles, FolderOpen, Trash2, AlertTriangle, Plus, Pencil } from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { ScreenShell } from "./ScreenShell";
import { SkillFormModal } from "./skills/SkillFormModal";
import { SegmentedControl, Toggle, Modal, Button } from "../components";
import { useSkills } from "../store/skills";
import { isTauri } from "../lib/tauri";
import { useToast } from "../store/toast";
import { skillTargetOf, type Skill } from "../types/skills";
import type { Scope } from "../types/inventory";

export function SkillsScreen() {
  const { status, skills, error, mocked, load } = useSkills();
  const [filter, setFilter] = useState<"all" | Scope>("all");
  const [toDelete, setToDelete] = useState<Skill | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [toEdit, setToEdit] = useState<Skill | null>(null);

  useEffect(() => {
    if (status === "idle") load();
  }, [status, load]);

  function openCreate() {
    setToEdit(null);
    setFormOpen(true);
  }

  function openEdit(skill: Skill) {
    setToEdit(skill);
    setFormOpen(true);
  }

  const visible = useMemo(
    () => (filter === "all" ? skills : skills.filter((s) => s.scope === filter)),
    [skills, filter],
  );

  return (
    <ScreenShell
      title="Skills"
      subtitle="Carpetas con SKILL.md por scope. Deshabilitar o eliminar mueve la carpeta a un backup recuperable."
      actions={
        <Button onClick={openCreate}>
          <Plus size={16} strokeWidth={2.5} />
          Nueva skill
        </Button>
      }
    >
      {status === "loading" && (
        <div className="flex flex-col gap-2.5">
          {[0, 1, 2].map((i) => (
            <div
              key={i}
              className="ds-card h-[68px] animate-pulse !rounded-[var(--radius-md)]"
              style={{ opacity: 1 - i * 0.2 }}
            />
          ))}
        </div>
      )}

      {status === "error" && (
        <div className="ds-card flex flex-col items-center gap-3 px-6 py-12 text-center">
          <AlertTriangle size={26} style={{ color: "var(--state-danger-text)" }} />
          <h2 className="text-[16px] font-semibold text-ink">
            No se pudieron leer las skills
          </h2>
          <p className="max-w-md font-mono text-[12px] text-muted">
            {error ?? "Error desconocido"}
          </p>
        </div>
      )}

      {status === "ready" && skills.length === 0 && (
        <div className="ds-card flex flex-col items-center gap-3 px-6 py-14 text-center">
          <div
            className="flex h-14 w-14 items-center justify-center rounded-[18px] border border-[var(--line)]"
            style={{ background: "var(--accent-soft)" }}
          >
            <Sparkles size={26} style={{ color: "var(--accent-strong)" }} />
          </div>
          <h2 className="text-[16px] font-semibold text-ink">Sin skills</h2>
          <p className="max-w-sm text-[13px] leading-relaxed text-muted">
            No se encontraron skills en ~/.claude/skills ni en los proyectos
            registrados.
          </p>
        </div>
      )}

      {status === "ready" && skills.length > 0 && (
        <>
          <div className="mb-5 flex items-center justify-between gap-4">
            <SegmentedControl
              label="Filtrar por scope"
              value={filter}
              onChange={setFilter}
              options={[
                { value: "all" as const, label: "Todas" },
                { value: "user" as Scope, label: "Global (user)" },
                { value: "project" as Scope, label: "Proyecto" },
              ]}
            />
            <span className="text-[12px] text-faint">
              {skills.length} skill{skills.length === 1 ? "" : "s"}
            </span>
          </div>

          <div className="flex flex-col gap-2.5">
            {visible.map((s) => (
              <SkillRow
                key={`${s.scope}:${s.projectPath ?? ""}:${s.name}`}
                skill={s}
                onEdit={() => openEdit(s)}
                onDelete={() => setToDelete(s)}
              />
            ))}
          </div>
        </>
      )}

      {mocked && (
        <p className="mt-6 text-center text-[12px] text-faint">
          Datos de muestra (modo browser).
        </p>
      )}

      {toDelete && (
        <DeleteSkillModal skill={toDelete} onClose={() => setToDelete(null)} />
      )}

      <SkillFormModal
        open={formOpen}
        onClose={() => setFormOpen(false)}
        initial={toEdit}
      />
    </ScreenShell>
  );
}

function SkillRow({
  skill,
  onEdit,
  onDelete,
}: {
  skill: Skill;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const setEnabled = useSkills((s) => s.setEnabled);
  const pushToast = useToast((s) => s.push);

  async function openFolder() {
    if (!isTauri()) {
      pushToast("info", "Abrir carpeta requiere la app (no browser).");
      return;
    }
    try {
      await revealItemInDir(skill.path);
    } catch (err) {
      pushToast("error", String(err));
    }
  }

  return (
    <div className="ds-card flex items-center gap-4 !rounded-[var(--radius-md)] px-4 py-3.5">
      <div
        className="flex h-10 w-10 shrink-0 items-center justify-center rounded-[12px] border border-[var(--line)]"
        style={{ background: "var(--color-surface-2)" }}
      >
        <Sparkles size={17} className="text-ink-soft" />
      </div>

      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-[14px] font-semibold text-ink">{skill.name}</span>
          <Tag>{skill.scope === "user" ? "user" : "proyecto"}</Tag>
          {skill.version && <Tag>v{skill.version}</Tag>}
          {!skill.enabled && <Tag>deshabilitada</Tag>}
        </div>
        <p className="mt-0.5 line-clamp-1 text-[12.5px] text-muted">
          {skill.description || "—"}
        </p>
      </div>

      <button
        onClick={onEdit}
        aria-label="Editar skill"
        title="Editar"
        className="flex h-8 w-8 items-center justify-center rounded-[10px] text-muted transition-colors duration-[var(--dur)] hover:text-ink"
        onMouseEnter={(e) =>
          (e.currentTarget.style.background = "rgba(255,255,255,0.06)")
        }
        onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
      >
        <Pencil size={15} />
      </button>
      <button
        onClick={openFolder}
        aria-label="Abrir carpeta"
        title={skill.path}
        className="flex h-8 w-8 items-center justify-center rounded-[10px] text-muted transition-colors duration-[var(--dur)] hover:text-ink"
        onMouseEnter={(e) =>
          (e.currentTarget.style.background = "rgba(255,255,255,0.06)")
        }
        onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
      >
        <FolderOpen size={15} />
      </button>
      <button
        onClick={onDelete}
        aria-label="Eliminar skill"
        title="Eliminar"
        className="flex h-8 w-8 items-center justify-center rounded-[10px] transition-colors duration-[var(--dur)]"
        style={{ color: "var(--state-danger-text)" }}
        onMouseEnter={(e) =>
          (e.currentTarget.style.background = "var(--state-danger-soft)")
        }
        onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
      >
        <Trash2 size={15} />
      </button>
      <Toggle
        checked={skill.enabled}
        onChange={(v) => setEnabled(skillTargetOf(skill), v)}
        label={`${skill.enabled ? "Deshabilitar" : "Habilitar"} ${skill.name}`}
      />
    </div>
  );
}

function DeleteSkillModal({
  skill,
  onClose,
}: {
  skill: Skill;
  onClose: () => void;
}) {
  const remove = useSkills((s) => s.remove);
  const busy = useSkills((s) => s.busy);
  return (
    <Modal
      open
      onClose={onClose}
      title={`Eliminar ${skill.name}`}
      subtitle={skill.scope === "user" ? "Global (user)" : "Proyecto"}
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
              const ok = await remove(skillTargetOf(skill));
              if (ok) onClose();
            }}
          >
            Eliminar
          </Button>
        </>
      }
    >
      <p className="text-[13px] leading-relaxed text-muted">
        La carpeta se mueve a un backup recuperable en{" "}
        <span className="font-mono text-ink-soft">
          ~/.mcp-manager/deleted-skills/
        </span>
        , no se borra del disco.
      </p>
    </Modal>
  );
}

function Tag({ children }: { children: ReactNode }) {
  return (
    <span
      className="rounded-full px-2 py-0.5 text-[10.5px] font-medium"
      style={{
        background: "var(--color-surface-3)",
        color: "var(--color-ink-soft)",
      }}
    >
      {children}
    </span>
  );
}
