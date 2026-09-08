//! E2E del rename de skills bajo el SANDBOX de desarrollo
//! (`MCP_MANAGER_CONFIG_ROOT`): verifica que se muevan la carpeta Y el
//! `name:` del frontmatter (los dos lugares donde vive la identidad de
//! una skill), preservando byte a byte el resto del SKILL.md.
//!
//! Un solo `#[test]` por binario: `set_var` es process-global.

use mcp_core::domain::Scope;
use mcp_core::skills::{self, SkillTarget};

/// Frontmatter con claves que el parser NO modela (`allowed-tools`,
/// `license`, `metadata`), un `name:` anidado, y un bloque literal que
/// contiene una línea `---` INDENTADA — las tres trampas de una
/// reescritura ingenua del frontmatter.
const SKILL_MD: &str = "---\nname: old-name\ndescription: |\n  hace algo útil\n  ---\n  y sigue\nversion: 1.2.3\nallowed-tools: Read, Grep\nlicense: MIT\nmetadata:\n  name: no-me-toques\n  author: ezequiel\n---\n\n# Cuerpo\n\nTexto con `código` y\n\n---\n\nguiones en columna 0.\n";

#[test]
fn rename_skill_under_sandbox() {
    let root = tempfile::tempdir().expect("tempdir");
    std::env::set_var(mcp_core::paths::CONFIG_ROOT_ENV, root.path());
    assert!(mcp_core::paths::sandbox_active());

    let skills_dir = root.path().join("home").join(".claude").join("skills");
    let old_dir = skills_dir.join("old-name");
    std::fs::create_dir_all(&old_dir).unwrap();
    std::fs::write(old_dir.join("SKILL.md"), SKILL_MD).unwrap();
    // Un archivo extra en la carpeta: el rename mueve todo, no solo el md.
    std::fs::write(old_dir.join("reference.md"), "no me pierdas").unwrap();

    // Una segunda skill, para probar la colisión.
    let taken_dir = skills_dir.join("taken");
    std::fs::create_dir_all(&taken_dir).unwrap();
    std::fs::write(
        taken_dir.join("SKILL.md"),
        "---\nname: taken\ndescription: ya existe\n---\n",
    )
    .unwrap();

    let target = SkillTarget {
        scope: Scope::User,
        project_path: None,
        name: "old-name".to_string(),
    };

    // -----------------------------------------------------------------
    // 1. Colisión y nombres inválidos: fallan sin tocar nada.
    // -----------------------------------------------------------------
    assert!(skills::rename_skill(&target, "taken").is_err());
    for bad in ["", "con espacio", "a/b", "..", "acentué", "a:b"] {
        assert!(
            skills::rename_skill(&target, bad).is_err(),
            "'{bad}' debería rechazarse"
        );
    }
    assert_eq!(
        std::fs::read_to_string(old_dir.join("SKILL.md")).unwrap(),
        SKILL_MD,
        "un rename fallido no debe modificar el SKILL.md"
    );
    assert!(old_dir.exists());

    // -----------------------------------------------------------------
    // 2. Rename efectivo.
    // -----------------------------------------------------------------
    skills::rename_skill(&target, "new-name").expect("rename");

    let new_dir = skills_dir.join("new-name");
    assert!(new_dir.exists(), "la carpeta se movió");
    assert!(!old_dir.exists(), "la carpeta vieja no queda atrás");
    assert_eq!(
        std::fs::read_to_string(new_dir.join("reference.md")).unwrap(),
        "no me pierdas",
        "los demás archivos de la carpeta viajan con ella"
    );

    // El frontmatter cambió SOLO en la línea `name:`.
    let written = std::fs::read_to_string(new_dir.join("SKILL.md")).unwrap();
    assert_eq!(
        written,
        SKILL_MD.replace("name: old-name", "name: new-name"),
        "solo la línea `name:` top-level debería diferir"
    );
    // Explícito, porque es el punto del diseño: claves no modeladas y el
    // `name` anidado sobreviven.
    assert!(written.contains("allowed-tools: Read, Grep"));
    assert!(written.contains("license: MIT"));
    assert!(written.contains("  name: no-me-toques"));
    // El `---` indentado del bloque literal no se tomó como cierre del
    // frontmatter, así que no hay una segunda clave `name:` top-level.
    assert_eq!(
        written.matches("\nname:").count(),
        1,
        "una sola clave `name:` top-level: {written}"
    );
    assert!(written.contains("  ---\n  y sigue"));
    assert!(written.contains("guiones en columna 0."));

    // Backup del SKILL.md previo, recuperable.
    let backups = root
        .path()
        .join("home")
        .join(".mcp-manager")
        .join("backups")
        .join("skills");
    assert!(backups.read_dir().unwrap().count() > 0);

    // -----------------------------------------------------------------
    // 3. El inventario reporta el nombre nuevo (una sola vez).
    // -----------------------------------------------------------------
    let listed = skills::read_skills().expect("read_skills");
    assert_eq!(
        listed.iter().filter(|s| s.name == "new-name").count(),
        1,
        "la skill aparece una vez con el nombre nuevo: {listed:?}"
    );
    assert!(!listed.iter().any(|s| s.name == "old-name"));
    let renamed = listed.iter().find(|s| s.name == "new-name").unwrap();
    // El bloque literal se sigue parseando entero: el rename no lo cortó.
    assert_eq!(renamed.description, "hace algo útil\n---\ny sigue");
    assert_eq!(renamed.version.as_deref(), Some("1.2.3"));

    // -----------------------------------------------------------------
    // 4. Rename de una skill DESHABILITADA: la carpeta vive en el
    //    sidecar, así que solo se reescribe el frontmatter y el manifest
    //    tiene que quedar apuntando al destino de restauración nuevo.
    // -----------------------------------------------------------------
    let new_target = SkillTarget {
        name: "new-name".to_string(),
        ..target.clone()
    };
    skills::set_skill_enabled(&new_target, false).expect("disable");
    assert!(!new_dir.exists(), "deshabilitar la mueve al sidecar");

    skills::rename_skill(&new_target, "final-name").expect("rename disabled");

    let final_target = SkillTarget {
        name: "final-name".to_string(),
        ..target.clone()
    };
    let listed = skills::read_skills().expect("read_skills");
    let disabled_skill = listed
        .iter()
        .find(|s| s.name == "final-name")
        .expect("la skill deshabilitada se lista con el nombre nuevo");
    assert!(!disabled_skill.enabled);

    // Y al habilitarla vuelve a ~/.claude/skills con el nombre nuevo.
    skills::set_skill_enabled(&final_target, true).expect("enable");
    let final_dir = skills_dir.join("final-name");
    assert!(final_dir.exists(), "vuelve a la carpeta del nombre nuevo");
    let written = std::fs::read_to_string(final_dir.join("SKILL.md")).unwrap();
    assert!(written.contains("name: final-name"));
    assert!(written.contains("license: MIT"));
}
