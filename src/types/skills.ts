import type { Scope } from "./inventory";

/** Espejo del DTO `Skill` de Rust (src-tauri/src/skills.rs). */
export interface Skill {
  name: string;
  description: string;
  version?: string | null;
  scope: Scope;
  projectPath?: string | null;
  /** Ruta absoluta de la carpeta de la skill. */
  path: string;
  enabled: boolean;
}

/** Identifica una skill para las mutaciones (espejo de SkillTarget en Rust). */
export interface SkillTarget {
  scope: Scope;
  projectPath?: string | null;
  name: string;
}

export function skillTargetOf(s: Skill): SkillTarget {
  return { scope: s.scope, projectPath: s.projectPath ?? null, name: s.name };
}

/** Payload para crear/editar una skill (espejo de SkillInput en Rust). */
export interface SkillInput {
  scope: Scope;
  projectPath?: string | null;
  name: string;
  description: string;
  version?: string | null;
  /** Cuerpo markdown tras el frontmatter. Vacío al editar = preserva el actual. */
  body: string;
}
