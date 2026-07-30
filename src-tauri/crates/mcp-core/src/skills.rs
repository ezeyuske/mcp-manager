//! Lector + gestión de Skills (carpetas con `SKILL.md`).
//!
//! Scope user: `~/.claude/skills/<name>/SKILL.md`.
//! Scope project: `<repo>/.claude/skills/<name>/SKILL.md`.
//!
//! La gestión (enable/disable/delete) NUNCA borra de forma destructiva:
//! deshabilitar mueve la carpeta a un sidecar recuperable
//! (`~/.mcp-manager/disabled-skills/`), y eliminar la mueve a
//! `~/.mcp-manager/deleted-skills/<name>.<ts>/`. Ambos son reversibles a mano.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::domain::Scope;
use crate::error::WriteError;
use crate::{changelog, paths, projects};

static UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub scope: Scope,
    pub project_path: Option<String>,
    /// Ruta absoluta de la carpeta de la skill (su ubicación ACTUAL: si está
    /// deshabilitada, apunta al sidecar).
    pub path: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillTarget {
    pub scope: Scope,
    pub project_path: Option<String>,
    pub name: String,
}

/// Datos para crear o editar una skill (`SKILL.md`).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInput {
    pub scope: Scope,
    pub project_path: Option<String>,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub version: Option<String>,
    /// Cuerpo markdown que va DESPUÉS del frontmatter. Si viene vacío al
    /// editar una skill existente, se PRESERVA el body actual (misma
    /// disciplina que `mutations::merge_entry` con `env`).
    #[serde(default)]
    pub body: String,
}

/// Frontmatter que serializamos al escribir un SKILL.md. Solo estos campos;
/// `serde_yaml_ng` garantiza YAML válido y escapado.
#[derive(Debug, Serialize)]
struct FrontmatterOut {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
}

/// Frontmatter YAML de un SKILL.md. Solo nos interesan estos campos; el
/// resto (license, etc.) se ignora.
#[derive(Debug, Default, Deserialize)]
struct Frontmatter {
    name: Option<String>,
    description: Option<String>,
    version: Option<String>,
}

/// Entrada del manifest de skills deshabilitadas (`disabled-skills.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DisabledSkill {
    scope: Scope,
    project_path: Option<String>,
    name: String,
    /// Dónde vivía originalmente (para restaurarla al habilitar).
    original_path: String,
    /// Dónde está ahora (sidecar).
    sidecar_path: String,
}

// ---------------------------------------------------------------------
// Parseo de frontmatter
// ---------------------------------------------------------------------

fn parse_frontmatter(content: &str) -> Frontmatter {
    let trimmed = content.trim_start_matches('\u{feff}').trim_start();
    if let Some(after) = trimmed.strip_prefix("---") {
        // `after` empieza justo tras el primer `---`; el cierre es la
        // siguiente línea `---`.
        if let Some(end) = after.find("\n---") {
            let yaml = &after[..end];
            if let Ok(fm) = serde_yaml_ng::from_str::<Frontmatter>(yaml) {
                return fm;
            }
        }
    }
    Frontmatter::default()
}

/// Lee una carpeta de skill (si contiene SKILL.md). `read_to_string`/`exists`
/// siguen symlinks, así que un SKILL.md symlinkeado se lee bien.
fn read_skill_dir(
    dir: &Path,
    scope: Scope,
    project_path: Option<String>,
    enabled: bool,
) -> Option<Skill> {
    let skill_md = dir.join("SKILL.md");
    if !skill_md.exists() {
        return None;
    }
    let folder_name = dir.file_name()?.to_string_lossy().to_string();
    let content = std::fs::read_to_string(&skill_md).unwrap_or_default();
    let fm = parse_frontmatter(&content);

    Some(Skill {
        name: fm.name.filter(|s| !s.trim().is_empty()).unwrap_or(folder_name),
        description: fm.description.unwrap_or_default().trim().to_string(),
        version: fm.version,
        scope,
        project_path,
        path: dir.display().to_string(),
        enabled,
    })
}

fn scan_skills_dir(base: &Path, scope: Scope, project_path: Option<String>) -> Vec<Skill> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(base) else {
        return out;
    };
    for entry in entries.flatten() {
        if let Some(skill) = read_skill_dir(&entry.path(), scope, project_path.clone(), true) {
            out.push(skill);
        }
    }
    out
}

// ---------------------------------------------------------------------
// Manifest de deshabilitadas
// ---------------------------------------------------------------------

fn disabled_manifest_path(app_data: &Path) -> PathBuf {
    app_data.join("disabled-skills.json")
}

fn read_disabled(app_data: &Path) -> Vec<DisabledSkill> {
    let path = disabled_manifest_path(app_data);
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    if raw.trim().is_empty() {
        return Vec::new();
    }
    serde_json::from_str(&raw).unwrap_or_default()
}

fn write_disabled(app_data: &Path, entries: &[DisabledSkill]) -> Result<(), WriteError> {
    let path = disabled_manifest_path(app_data);
    let serialized = serde_json::to_string_pretty(entries).map_err(|e| WriteError::Serialize {
        message: e.to_string(),
    })?;
    std::fs::write(&path, serialized).map_err(|source| WriteError::Io {
        path: path.display().to_string(),
        source,
    })
}

// ---------------------------------------------------------------------
// Lectura
// ---------------------------------------------------------------------

fn user_skills_dir() -> Result<PathBuf, WriteError> {
    // Con el sandbox de dev activo resuelve bajo `<root>/home` — ver
    // `paths::home_dir`.
    let home = crate::paths::home_dir().ok_or_else(|| WriteError::NotSupported {
        message: "no se pudo resolver el directorio home del usuario".to_string(),
    })?;
    Ok(home.join(".claude").join("skills"))
}

fn project_skills_dir(project_path: &str) -> PathBuf {
    Path::new(project_path).join(".claude").join("skills")
}

/// Variante testeable: recibe los roots en vez de resolverlos del sistema.
fn read_skills_at(user_dir: &Path, app_data: &Path, project_paths: &[String]) -> Vec<Skill> {
    let mut skills = scan_skills_dir(user_dir, Scope::User, None);

    for project_path in project_paths {
        let base = project_skills_dir(project_path);
        skills.extend(scan_skills_dir(
            &base,
            Scope::Project,
            Some(project_path.clone()),
        ));
    }

    // Deshabilitadas (viven en el sidecar): se leen de ahí, enabled=false.
    for d in read_disabled(app_data) {
        if let Some(skill) = read_skill_dir(
            Path::new(&d.sidecar_path),
            d.scope,
            d.project_path.clone(),
            false,
        ) {
            skills.push(skill);
        }
    }

    skills.sort_by_key(|s| s.name.to_lowercase());
    skills
}

pub fn read_skills() -> Result<Vec<Skill>, WriteError> {
    let user_dir = user_skills_dir()?;
    let app_data = paths::app_data_dir()?;
    let project_paths = projects::list().unwrap_or_default();
    Ok(read_skills_at(&user_dir, &app_data, &project_paths))
}

// ---------------------------------------------------------------------
// Mover carpetas (recuperable)
// ---------------------------------------------------------------------

fn unique_suffix() -> String {
    let n = UNIQUE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}", chrono::Utc::now().timestamp_millis(), n)
}

/// Copia `src` a `dst` de forma SYMLINK-SAFE: un symlink se RECREA (no se
/// sigue su target — así no copiamos ni tocamos, p. ej., ~/.agents/skills al
/// que apunta un SKILL.md symlinkeado). Directorios se copian recursivo.
fn copy_path(src: &Path, dst: &Path) -> std::io::Result<()> {
    let meta = std::fs::symlink_metadata(src)?;
    let ft = meta.file_type();
    if ft.is_symlink() {
        let target = std::fs::read_link(src)?;
        std::os::unix::fs::symlink(target, dst)?;
    } else if ft.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            copy_path(&entry.path(), &dst.join(entry.file_name()))?;
        }
    } else {
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

/// Borra `p` de forma SYMLINK-SAFE: un symlink (aunque apunte a un dir) se
/// borra con `remove_file` (solo el link), NUNCA con `remove_dir_all` (que
/// en macOS puede colgarse/loopear sobre un symlink-a-directorio).
fn remove_path(p: &Path) -> std::io::Result<()> {
    let meta = std::fs::symlink_metadata(p)?;
    if meta.file_type().is_symlink() || meta.is_file() {
        std::fs::remove_file(p)
    } else {
        std::fs::remove_dir_all(p)
    }
}

/// Mueve `from` a `to`. `rename` no sigue symlinks (mueve el link mismo).
/// Fallback SYMLINK-SAFE a copia + remove si el destino está en otro
/// filesystem (EXDEV).
fn move_dir(from: &Path, to: &Path) -> Result<(), WriteError> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|source| WriteError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(18) => {
            // EXDEV: cross-device. Copiar (symlink-safe) + borrar el origen.
            copy_path(from, to).map_err(|source| WriteError::Io {
                path: from.display().to_string(),
                source,
            })?;
            remove_path(from).map_err(|source| WriteError::Io {
                path: from.display().to_string(),
                source,
            })
        }
        Err(source) => Err(WriteError::Io {
            path: from.display().to_string(),
            source,
        }),
    }
}

/// El `name` de una skill debe ser un ÚNICO componente de path (nombre de
/// carpeta), sin separadores ni `..`: evita traversal si llegara un
/// `SkillTarget` malicioso/roto desde el frontend.
fn valid_skill_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && name != "."
        && name != ".."
        && !name.contains("..")
}

fn skill_original_dir(target: &SkillTarget, user_dir: &Path) -> Result<PathBuf, WriteError> {
    if !valid_skill_name(&target.name) {
        return Err(WriteError::TargetNotFound {
            message: format!("nombre de skill inválido: '{}'", target.name),
        });
    }
    match target.scope {
        Scope::User => Ok(user_dir.join(&target.name)),
        Scope::Project => {
            let project_path = target.project_path.as_ref().ok_or_else(|| {
                WriteError::TargetNotFound {
                    message: "scope Project requiere projectPath".to_string(),
                }
            })?;
            Ok(project_skills_dir(project_path).join(&target.name))
        }
    }
}

// ---------------------------------------------------------------------
// enable / disable
// ---------------------------------------------------------------------

fn set_skill_enabled_at(
    target: &SkillTarget,
    enabled: bool,
    user_dir: &Path,
    app_data: &Path,
) -> Result<(), WriteError> {
    let mut disabled = read_disabled(app_data);

    if enabled {
        // Habilitar: buscar en el manifest y mover de vuelta al original.
        let idx = disabled
            .iter()
            .position(|d| {
                d.scope == target.scope
                    && d.project_path == target.project_path
                    && d.name == target.name
            })
            .ok_or_else(|| WriteError::TargetNotFound {
                message: format!("la skill '{}' no está deshabilitada", target.name),
            })?;
        let entry = disabled.remove(idx);
        // Colisión: si ya hay algo en el destino (el usuario recreó una skill
        // con ese nombre mientras estaba deshabilitada), abortamos con un
        // mensaje claro SIN escribir el manifest — el estado en disco queda
        // consistente (la entrada sigue listada como deshabilitada).
        if Path::new(&entry.original_path).exists() {
            return Err(WriteError::NotSupported {
                message: format!(
                    "ya existe una skill en {}; resolvé el conflicto antes de habilitar '{}'",
                    entry.original_path, target.name
                ),
            });
        }
        move_dir(
            Path::new(&entry.sidecar_path),
            Path::new(&entry.original_path),
        )?;
        write_disabled(app_data, &disabled)?;
        Ok(())
    } else {
        // Deshabilitar: mover el original al sidecar y registrar en manifest.
        let original = skill_original_dir(target, user_dir)?;
        if !original.exists() {
            return Err(WriteError::TargetNotFound {
                message: format!("no se encontró la skill '{}'", target.name),
            });
        }
        let sidecar_root = app_data.join("disabled-skills");
        let sidecar_path = sidecar_root.join(unique_suffix());
        move_dir(&original, &sidecar_path)?;
        disabled.push(DisabledSkill {
            scope: target.scope,
            project_path: target.project_path.clone(),
            name: target.name.clone(),
            original_path: original.display().to_string(),
            sidecar_path: sidecar_path.display().to_string(),
        });
        write_disabled(app_data, &disabled)?;
        Ok(())
    }
}

pub fn set_skill_enabled(target: &SkillTarget, enabled: bool) -> Result<(), WriteError> {
    let user_dir = user_skills_dir()?;
    let app_data = paths::app_data_dir()?;
    set_skill_enabled_at(target, enabled, &user_dir, &app_data)
}

// ---------------------------------------------------------------------
// delete (a papelera recuperable)
// ---------------------------------------------------------------------

fn delete_skill_at(
    target: &SkillTarget,
    user_dir: &Path,
    app_data: &Path,
) -> Result<(), WriteError> {
    // Si está deshabilitada, la fuente es el sidecar; si no, el original.
    let mut disabled = read_disabled(app_data);
    let disabled_idx = disabled.iter().position(|d| {
        d.scope == target.scope && d.project_path == target.project_path && d.name == target.name
    });

    let source = match disabled_idx {
        Some(i) => PathBuf::from(disabled[i].sidecar_path.clone()),
        None => skill_original_dir(target, user_dir)?,
    };

    if !source.exists() {
        return Err(WriteError::TargetNotFound {
            message: format!("no se encontró la skill '{}'", target.name),
        });
    }

    let trash = app_data
        .join("deleted-skills")
        .join(format!("{}.{}", target.name, unique_suffix()));
    move_dir(&source, &trash)?;

    if let Some(i) = disabled_idx {
        disabled.remove(i);
        write_disabled(app_data, &disabled)?;
    }
    Ok(())
}

pub fn delete_skill(target: &SkillTarget) -> Result<(), WriteError> {
    let user_dir = user_skills_dir()?;
    let app_data = paths::app_data_dir()?;
    delete_skill_at(target, &user_dir, &app_data)
}

// ---------------------------------------------------------------------
// upsert (crear / editar SKILL.md)
// ---------------------------------------------------------------------

/// Separa un SKILL.md en `(frontmatter_yaml, body)`. Si no hay
/// frontmatter, el contenido entero es body.
fn split_frontmatter_body(content: &str) -> (Option<&str>, &str) {
    let t = content.trim_start_matches('\u{feff}');
    let leading_ws = t.len() - t.trim_start().len();
    let start = &t[leading_ws..];
    if let Some(after) = start.strip_prefix("---") {
        if let Some(end) = after.find("\n---") {
            let yaml = &after[..end];
            // `rest` arranca en la línea de cierre `---`.
            let rest = &after[end + 1..];
            if let Some(nl) = rest.find('\n') {
                return (Some(yaml), &rest[nl + 1..]);
            }
            return (Some(yaml), "");
        }
    }
    (None, content)
}

/// Renderiza el contenido completo de un SKILL.md a partir del input y un
/// body ya resuelto.
fn render_skill_md(input: &SkillInput, body: &str) -> Result<String, WriteError> {
    let fm = FrontmatterOut {
        name: input.name.clone(),
        description: input.description.trim().to_string(),
        version: input
            .version
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty()),
    };
    let yaml = serde_yaml_ng::to_string(&fm).map_err(|e| WriteError::Serialize {
        message: e.to_string(),
    })?;

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&yaml);
    out.push_str("---\n");
    let body = body.trim_matches('\n');
    if !body.is_empty() {
        out.push('\n');
        out.push_str(body);
        out.push('\n');
    }
    Ok(out)
}

/// Backup recuperable del SKILL.md previo (si existe) antes de pisarlo, en
/// `~/.mcp-manager/backups/skills/<name>.<ts>.md`.
fn backup_existing_skill_md(
    skill_md: &Path,
    app_data: &Path,
    name: &str,
) -> Result<(), WriteError> {
    if !skill_md.exists() {
        return Ok(());
    }
    let dir = app_data.join("backups").join("skills");
    std::fs::create_dir_all(&dir).map_err(|source| WriteError::Backup {
        path: dir.display().to_string(),
        source,
    })?;
    let dst = dir.join(format!("{}.{}.md", name, unique_suffix()));
    std::fs::copy(skill_md, &dst).map_err(|source| WriteError::Backup {
        path: dst.display().to_string(),
        source,
    })?;
    Ok(())
}

fn upsert_skill_at(
    input: &SkillInput,
    user_dir: &Path,
    app_data: &Path,
) -> Result<(), WriteError> {
    if input.description.trim().is_empty() {
        return Err(WriteError::NotSupported {
            message: "la skill requiere una descripción no vacía".to_string(),
        });
    }

    let target = SkillTarget {
        scope: input.scope,
        project_path: input.project_path.clone(),
        name: input.name.clone(),
    };
    // Valida el nombre (rechaza traversal) y resuelve la carpeta destino.
    let dir = skill_original_dir(&target, user_dir)?;
    let skill_md = dir.join("SKILL.md");

    // No escribir a través de un symlink: si la carpeta de la skill es un
    // symlink, podría apuntar fuera del árbol de skills.
    if let Ok(meta) = std::fs::symlink_metadata(&dir) {
        if meta.file_type().is_symlink() {
            return Err(WriteError::NotSupported {
                message: format!(
                    "la carpeta de la skill '{}' es un symlink; resolvelo a mano",
                    input.name
                ),
            });
        }
    }

    // Al editar con body vacío, preservar el body existente.
    let body = if input.body.trim().is_empty() {
        std::fs::read_to_string(&skill_md)
            .map(|existing| split_frontmatter_body(&existing).1.to_string())
            .unwrap_or_default()
    } else {
        input.body.clone()
    };

    let existed = skill_md.exists();
    let content = render_skill_md(input, &body)?;

    std::fs::create_dir_all(&dir).map_err(|source| WriteError::Io {
        path: dir.display().to_string(),
        source,
    })?;

    // Backup del previo (si había) + escritura atómica del ÚNICO SKILL.md.
    // Nunca tocamos otros archivos de la carpeta.
    backup_existing_skill_md(&skill_md, app_data, &input.name)?;
    crate::safe_write::atomic_write(&skill_md, &content)?;

    // Changelog best-effort (no rompe el upsert si falla).
    let action = if existed {
        changelog::MutationAction::Edit
    } else {
        changelog::MutationAction::Add
    };
    let _ = changelog::append(changelog::MutationLog::new(
        crate::domain::AppId::ClaudeCode,
        input.scope,
        skill_md.display().to_string(),
        action,
        input.name.clone(),
        None,
    ));

    Ok(())
}

/// Crea o edita una skill escribiendo su `SKILL.md` de forma segura.
pub fn upsert_skill(input: &SkillInput) -> Result<(), WriteError> {
    let user_dir = user_skills_dir()?;
    let app_data = paths::app_data_dir()?;
    upsert_skill_at(input, &user_dir, &app_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_skill(base: &Path, name: &str, frontmatter: &str) {
        let dir = base.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), frontmatter).unwrap();
    }

    #[test]
    fn parses_name_description_version() {
        let fm = parse_frontmatter(
            "---\nname: foo\ndescription: hace algo\nversion: \"1.2.0\"\n---\n# body",
        );
        assert_eq!(fm.name.as_deref(), Some("foo"));
        assert_eq!(fm.description.as_deref(), Some("hace algo"));
        assert_eq!(fm.version.as_deref(), Some("1.2.0"));
    }

    #[test]
    fn parses_folded_multiline_description() {
        let fm = parse_frontmatter(
            "---\nname: bar\ndescription: >-\n  linea uno\n  linea dos\n---\n",
        );
        assert_eq!(fm.name.as_deref(), Some("bar"));
        assert_eq!(fm.description.as_deref(), Some("linea uno linea dos"));
    }

    #[test]
    fn missing_frontmatter_falls_back_to_folder_name() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("skills");
        write_skill(&user, "no-fm", "# solo body, sin frontmatter\n");
        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();

        let skills = read_skills_at(&user, &app, &[]);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "no-fm");
        assert_eq!(skills[0].description, "");
    }

    #[test]
    fn reads_user_and_project_scopes() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("home/.claude/skills");
        write_skill(&user, "u1", "---\nname: u1\ndescription: user skill\n---\n");

        let repo = dir.path().join("repo");
        let project_skills = repo.join(".claude/skills");
        write_skill(&project_skills, "p1", "---\nname: p1\ndescription: proj skill\n---\n");

        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();

        let skills = read_skills_at(&user, &app, &[repo.display().to_string()]);
        let names: Vec<_> = skills.iter().map(|s| (s.name.as_str(), s.scope)).collect();
        assert!(names.contains(&("u1", Scope::User)));
        assert!(names.contains(&("p1", Scope::Project)));
    }

    #[test]
    fn reads_symlinked_skill_md() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("skills");
        // target real fuera del dir de skills
        let real = dir.path().join("real/SKILL.md");
        std::fs::create_dir_all(real.parent().unwrap()).unwrap();
        std::fs::write(&real, "---\nname: linked\ndescription: via symlink\n---\n").unwrap();
        let skill_dir = user.join("linked");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::os::unix::fs::symlink(&real, skill_dir.join("SKILL.md")).unwrap();

        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();
        let skills = read_skills_at(&user, &app, &[]);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "linked");
        assert_eq!(skills[0].description, "via symlink");
    }

    #[test]
    fn disable_then_enable_round_trips_folder_intact() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("skills");
        write_skill(&user, "toggle-me", "---\nname: toggle-me\ndescription: x\n---\nBODY");
        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();

        let target = SkillTarget {
            scope: Scope::User,
            project_path: None,
            name: "toggle-me".to_string(),
        };

        // disable: desaparece del dir de user, aparece como enabled=false
        set_skill_enabled_at(&target, false, &user, &app).unwrap();
        assert!(!user.join("toggle-me").exists());
        let skills = read_skills_at(&user, &app, &[]);
        assert_eq!(skills.len(), 1);
        assert!(!skills[0].enabled);

        // enable: vuelve intacta (con su BODY)
        set_skill_enabled_at(&target, true, &user, &app).unwrap();
        let restored = user.join("toggle-me").join("SKILL.md");
        assert!(restored.exists());
        let content = std::fs::read_to_string(&restored).unwrap();
        assert!(content.contains("BODY"));
        let skills = read_skills_at(&user, &app, &[]);
        assert!(skills[0].enabled);
    }

    #[test]
    fn rejects_skill_name_with_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("skills");
        std::fs::create_dir_all(&user).unwrap();
        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();

        let evil = SkillTarget {
            scope: Scope::User,
            project_path: None,
            name: "../../../etc/evil".to_string(),
        };
        assert!(set_skill_enabled_at(&evil, false, &user, &app).is_err());
        assert!(delete_skill_at(&evil, &user, &app).is_err());
    }

    #[test]
    fn disabling_symlinked_skill_does_not_touch_symlink_target() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("skills");
        // Target real del symlink, FUERA del árbol de skills.
        let external = dir.path().join("agents/linked/SKILL.md");
        std::fs::create_dir_all(external.parent().unwrap()).unwrap();
        std::fs::write(&external, "---\nname: linked\ndescription: real\n---\nDO-NOT-DELETE").unwrap();
        // Skill con SKILL.md symlinkeado al target externo.
        let skill_dir = user.join("linked");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::os::unix::fs::symlink(&external, skill_dir.join("SKILL.md")).unwrap();
        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();

        let target = SkillTarget {
            scope: Scope::User,
            project_path: None,
            name: "linked".to_string(),
        };
        set_skill_enabled_at(&target, false, &user, &app).unwrap();

        // El target externo del symlink DEBE seguir intacto.
        assert!(external.exists(), "el target del symlink no debe borrarse");
        assert_eq!(
            std::fs::read_to_string(&external).unwrap(),
            "---\nname: linked\ndescription: real\n---\nDO-NOT-DELETE"
        );
        // Y la skill se puede re-habilitar.
        set_skill_enabled_at(&target, true, &user, &app).unwrap();
        assert!(user.join("linked").join("SKILL.md").exists());
    }

    #[test]
    fn enable_aborts_cleanly_on_destination_collision() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("skills");
        write_skill(&user, "coll", "---\nname: coll\ndescription: x\n---\nORIG");
        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();
        let target = SkillTarget {
            scope: Scope::User,
            project_path: None,
            name: "coll".to_string(),
        };

        set_skill_enabled_at(&target, false, &user, &app).unwrap();
        // El usuario recrea a mano una carpeta con el mismo nombre.
        write_skill(&user, "coll", "---\nname: coll\ndescription: nuevo\n---\nNEW");

        // Habilitar debe fallar limpio (sin pisar lo nuevo), y la deshabilitada
        // sigue listada (recuperable).
        assert!(set_skill_enabled_at(&target, true, &user, &app).is_err());
        let disabled = read_disabled(&app);
        assert_eq!(disabled.len(), 1);
        assert!(std::fs::read_to_string(user.join("coll").join("SKILL.md"))
            .unwrap()
            .contains("NEW"));
    }

    #[test]
    fn delete_moves_to_recoverable_trash_not_destroyed() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("skills");
        write_skill(&user, "kill-me", "---\nname: kill-me\ndescription: x\n---\nKEEP");
        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();

        let target = SkillTarget {
            scope: Scope::User,
            project_path: None,
            name: "kill-me".to_string(),
        };
        delete_skill_at(&target, &user, &app).unwrap();

        assert!(!user.join("kill-me").exists());
        // Sigue existiendo en deleted-skills (recuperable).
        let trash = app.join("deleted-skills");
        let found = std::fs::read_dir(&trash)
            .unwrap()
            .flatten()
            .any(|e| e.path().join("SKILL.md").exists());
        assert!(found, "la skill debe seguir existiendo en deleted-skills");
    }

    fn input(name: &str, description: &str, body: &str) -> SkillInput {
        SkillInput {
            scope: Scope::User,
            project_path: None,
            name: name.to_string(),
            description: description.to_string(),
            version: None,
            body: body.to_string(),
        }
    }

    #[test]
    fn upsert_creates_valid_skill_md_with_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("skills");
        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();

        upsert_skill_at(&input("nueva", "hace algo útil", "# Título\ncontenido"), &user, &app)
            .unwrap();

        let content = std::fs::read_to_string(user.join("nueva").join("SKILL.md")).unwrap();
        let fm = parse_frontmatter(&content);
        assert_eq!(fm.name.as_deref(), Some("nueva"));
        assert_eq!(fm.description.as_deref(), Some("hace algo útil"));
        assert!(content.contains("# Título"));
        assert!(content.contains("contenido"));
    }

    #[test]
    fn upsert_does_not_clobber_sibling_files() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("skills");
        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();
        // Skill con un archivo hermano (script) además del SKILL.md.
        let skill_dir = user.join("con-script");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("run.py"), "print('hola')").unwrap();

        upsert_skill_at(&input("con-script", "desc", "body nuevo"), &user, &app).unwrap();

        // El hermano sigue intacto.
        assert_eq!(
            std::fs::read_to_string(skill_dir.join("run.py")).unwrap(),
            "print('hola')"
        );
        assert!(skill_dir.join("SKILL.md").exists());
    }

    #[test]
    fn upsert_rejects_traversal_name() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("skills");
        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();

        let evil = input("../../etc/evil", "desc", "body");
        assert!(upsert_skill_at(&evil, &user, &app).is_err());
    }

    #[test]
    fn upsert_requires_non_empty_description() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("skills");
        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();

        assert!(upsert_skill_at(&input("x", "   ", "body"), &user, &app).is_err());
    }

    #[test]
    fn upsert_edit_with_empty_body_preserves_existing_body() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("skills");
        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();

        upsert_skill_at(&input("edit-me", "v1", "CUERPO ORIGINAL"), &user, &app).unwrap();
        // Editar solo la descripción, con body vacío.
        upsert_skill_at(&input("edit-me", "v2 desc", ""), &user, &app).unwrap();

        let content = std::fs::read_to_string(user.join("edit-me").join("SKILL.md")).unwrap();
        let fm = parse_frontmatter(&content);
        assert_eq!(fm.description.as_deref(), Some("v2 desc"));
        assert!(content.contains("CUERPO ORIGINAL"), "el body debía preservarse");
    }

    #[test]
    fn upsert_backs_up_previous_skill_md_on_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("skills");
        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();

        upsert_skill_at(&input("bkp", "desc", "PRIMERA"), &user, &app).unwrap();
        upsert_skill_at(&input("bkp", "desc", "SEGUNDA"), &user, &app).unwrap();

        let backups = app.join("backups").join("skills");
        let has_backup = std::fs::read_dir(&backups)
            .unwrap()
            .flatten()
            .any(|e| std::fs::read_to_string(e.path()).map(|c| c.contains("PRIMERA")).unwrap_or(false));
        assert!(has_backup, "debía existir un backup con el contenido previo");
    }

    #[test]
    fn upsert_refuses_symlinked_skill_dir() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("skills");
        std::fs::create_dir_all(&user).unwrap();
        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();
        // La carpeta de la skill es un symlink a un dir externo.
        let external = dir.path().join("external");
        std::fs::create_dir_all(&external).unwrap();
        std::os::unix::fs::symlink(&external, user.join("linked")).unwrap();

        assert!(upsert_skill_at(&input("linked", "desc", "body"), &user, &app).is_err());
    }
}
