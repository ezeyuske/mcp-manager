import { useEffect, useMemo, useState } from "react";
import {
  FolderGit2,
  FolderPlus,
  X,
  Plug,
  Sparkles,
  ArrowUpRight,
} from "lucide-react";
import { ScreenShell } from "./ScreenShell";
import { Button, Card } from "../components";
import { useProjects } from "../store/projects";
import { useInventory } from "../store/inventory";
import { useSkills } from "../store/skills";

function basename(path: string): string {
  const parts = path.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export function ProjectsScreen() {
  const { status, projects, mocked, load, registerPicked, unregister } =
    useProjects();
  const inventory = useInventory((s) => s.inventory);
  const loadInventory = useInventory((s) => s.load);
  const skills = useSkills((s) => s.skills);
  const loadSkills = useSkills((s) => s.load);
  const skillsStatus = useSkills((s) => s.status);

  const [selected, setSelected] = useState<string | null>(null);

  useEffect(() => {
    if (status === "idle") load();
    if (skillsStatus === "idle") loadSkills();
    if (!inventory) loadInventory();
  }, [status, load, skillsStatus, loadSkills, inventory, loadInventory]);

  useEffect(() => {
    if (!selected && projects.length > 0) setSelected(projects[0]);
  }, [projects, selected]);

  return (
    <ScreenShell
      title="Proyectos"
      subtitle="Repos registrados con sus MCPs y skills de scope proyecto, y el diff contra el global."
      actions={
        <Button onClick={registerPicked}>
          <FolderPlus size={16} strokeWidth={2.5} />
          Registrar repo
        </Button>
      }
    >
      {status === "ready" && projects.length === 0 && (
        <div className="ds-card flex flex-col items-center gap-3 px-6 py-14 text-center">
          <div
            className="flex h-14 w-14 items-center justify-center rounded-[18px] border border-[var(--line)]"
            style={{ background: "var(--accent-soft)" }}
          >
            <FolderGit2 size={26} style={{ color: "var(--accent-strong)" }} />
          </div>
          <h2 className="text-[16px] font-semibold text-ink">Sin proyectos</h2>
          <p className="max-w-sm text-[13px] leading-relaxed text-muted">
            Registrá un repo para gestionar su <span className="font-mono">.mcp.json</span>{" "}
            y ver sus skills de proyecto.
          </p>
        </div>
      )}

      {projects.length > 0 && (
        <div className="flex gap-4">
          {/* Lista de proyectos */}
          <div className="flex w-64 shrink-0 flex-col gap-1.5">
            {projects.map((p) => {
              const active = p === selected;
              return (
                <button
                  key={p}
                  onClick={() => setSelected(p)}
                  className="group flex items-center gap-2.5 rounded-[12px] border px-3 py-2.5 text-left transition-colors duration-[var(--dur)]"
                  style={
                    active
                      ? {
                          background: "var(--accent-soft)",
                          borderColor: "transparent",
                          boxShadow: "0 0 16px -4px var(--accent-glow)",
                        }
                      : {
                          background: "var(--color-surface-2)",
                          borderColor: "var(--line)",
                        }
                  }
                >
                  <FolderGit2
                    size={15}
                    style={{
                      color: active ? "var(--accent-strong)" : "var(--color-muted)",
                    }}
                  />
                  <span
                    className="min-w-0 flex-1 truncate text-[13px] font-medium"
                    style={{ color: active ? "var(--accent-strong)" : "var(--color-ink)" }}
                    title={p}
                  >
                    {basename(p)}
                  </span>
                </button>
              );
            })}
          </div>

          {/* Detalle del proyecto seleccionado */}
          <div className="min-w-0 flex-1">
            {selected && (
              <ProjectDetail
                path={selected}
                installations={inventory?.installations ?? []}
                skills={skills}
                onUnregister={() => {
                  unregister(selected);
                  setSelected(null);
                }}
              />
            )}
          </div>
        </div>
      )}

      {mocked && (
        <p className="mt-6 text-center text-[12px] text-faint">
          Datos de muestra (modo browser).
        </p>
      )}
    </ScreenShell>
  );
}

function ProjectDetail({
  path,
  installations,
  skills,
  onUnregister,
}: {
  path: string;
  installations: import("../types/inventory").McpInstallation[];
  skills: import("../types/skills").Skill[];
  onUnregister: () => void;
}) {
  const projectMcps = useMemo(
    () =>
      installations.filter(
        (i) => i.scope === "project" && i.projectPath === path,
      ),
    [installations, path],
  );
  const projectSkills = useMemo(
    () => skills.filter((s) => s.scope === "project" && s.projectPath === path),
    [skills, path],
  );

  // Diff: MCPs de scope user que NO están en este proyecto.
  const missingFromProject = useMemo(() => {
    const projectNames = new Set(projectMcps.map((m) => m.name));
    const userNames = new Set(
      installations.filter((i) => i.scope === "user").map((i) => i.name),
    );
    return [...userNames].filter((n) => !projectNames.has(n)).sort();
  }, [installations, projectMcps]);

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0">
            <h3 className="text-[15px] font-semibold text-ink">
              {basename(path)}
            </h3>
            <p className="mt-0.5 truncate font-mono text-[12px] text-faint">
              {path}
            </p>
          </div>
          <button
            onClick={onUnregister}
            className="flex items-center gap-1.5 rounded-[10px] border border-[var(--line)] px-2.5 py-1.5 text-[12px] font-medium text-muted transition-colors duration-[var(--dur)] hover:border-[var(--line-strong)] hover:text-ink"
            style={{ background: "var(--color-surface-2)" }}
          >
            <X size={13} /> Quitar
          </button>
        </div>
      </Card>

      <Card title="MCPs del proyecto" subtitle=".mcp.json en la raíz del repo">
        {projectMcps.length === 0 ? (
          <p className="text-[13px] text-faint">Sin MCPs de proyecto.</p>
        ) : (
          <div className="flex flex-col gap-2">
            {projectMcps.map((m) => (
              <div key={m.name} className="flex items-center gap-2.5">
                <Plug size={14} className="text-ink-soft" />
                <span className="text-[13px] font-medium text-ink">{m.name}</span>
                <span className="truncate font-mono text-[11.5px] text-faint">
                  {m.command ?? m.url ?? ""}
                </span>
              </div>
            ))}
          </div>
        )}
      </Card>

      <Card title="Skills del proyecto" subtitle=".claude/skills/ del repo">
        {projectSkills.length === 0 ? (
          <p className="text-[13px] text-faint">Sin skills de proyecto.</p>
        ) : (
          <div className="flex flex-col gap-2">
            {projectSkills.map((s) => (
              <div key={s.name} className="flex items-center gap-2.5">
                <Sparkles size={14} className="text-ink-soft" />
                <span className="text-[13px] font-medium text-ink">{s.name}</span>
                {!s.enabled && (
                  <span className="text-[11px] text-faint">(deshabilitada)</span>
                )}
              </div>
            ))}
          </div>
        )}
      </Card>

      <Card
        title="En el global, no en este proyecto"
        subtitle="MCPs de scope user que podrías querer instalar acá también"
      >
        {missingFromProject.length === 0 ? (
          <p className="text-[13px] text-faint">
            El proyecto tiene todos los MCPs del global.
          </p>
        ) : (
          <div className="flex flex-wrap gap-2">
            {missingFromProject.map((n) => (
              <span
                key={n}
                className="inline-flex items-center gap-1 rounded-full border border-[var(--line)] px-2.5 py-1 text-[12px] text-ink-soft"
                style={{ background: "var(--color-surface-2)" }}
              >
                <ArrowUpRight size={12} className="text-faint" />
                {n}
              </span>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
