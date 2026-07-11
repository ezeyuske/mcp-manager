// Compila el binario `mcp-server` y lo copia a
// `src-tauri/binaries/mcp-server-<TARGET_TRIPLE>` con el sufijo de triple
// que Tauri exige para `externalBin`.
//
// Uso:
//   node scripts/stage-sidecar.mjs            # release (para `tauri build`)
//   node scripts/stage-sidecar.mjs --debug    # debug (para `tauri dev`)
//
// En dev, además, el binario debug ya queda en `target/debug/mcp-server`,
// que es el que resuelve `resolve_sidecar_path` (hermano del ejecutable).
import { execSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";

const debug = process.argv.includes("--debug");
const profile = debug ? "debug" : "release";

function hostTriple() {
  const out = execSync("rustc -vV").toString();
  const line = out.split("\n").find((l) => l.startsWith("host:"));
  if (!line) throw new Error("no se pudo determinar el host triple con `rustc -vV`");
  return line.split(":")[1].trim();
}

const triple = hostTriple();
const ext = process.platform === "win32" ? ".exe" : "";

console.log(`[stage-sidecar] compilando mcp-server (${profile}) para ${triple}...`);
execSync(`cargo build -p mcp-server${debug ? "" : " --release"}`, {
  cwd: "src-tauri",
  stdio: "inherit",
});

const src = join("src-tauri", "target", profile, `mcp-server${ext}`);
const outDir = join("src-tauri", "binaries");
mkdirSync(outDir, { recursive: true });
const dst = join(outDir, `mcp-server-${triple}${ext}`);
copyFileSync(src, dst);
console.log(`[stage-sidecar] sidecar listo: ${dst}`);
