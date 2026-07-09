import {
  Plug,
  Sparkles,
  FolderGit2,
  KeyRound,
  ScrollText,
  Palette,
  Settings,
  type LucideIcon,
} from "lucide-react";
import { useUI } from "../store/ui";
import type { Screen } from "../types/nav";

interface NavItem {
  screen: Screen;
  label: string;
  icon: LucideIcon;
  /** Badge numérico (0 u omitido = sin badge). Placeholder hasta Fase 2. */
  badge?: number;
}

const NAV: NavItem[] = [
  { screen: "mcps", label: "MCP Servers", icon: Plug, badge: 12 },
  { screen: "skills", label: "Skills", icon: Sparkles, badge: 5 },
  { screen: "projects", label: "Proyectos", icon: FolderGit2, badge: 3 },
  { screen: "env", label: "Env & Secrets", icon: KeyRound },
  { screen: "activity", label: "Actividad", icon: ScrollText },
  { screen: "themes", label: "Themes", icon: Palette },
  { screen: "settings", label: "Ajustes", icon: Settings },
];

export function Sidebar() {
  const screen = useUI((s) => s.screen);
  const setScreen = useUI((s) => s.setScreen);

  return (
    <aside className="flex h-full w-[248px] shrink-0 flex-col gap-1 border-r border-[var(--line)] bg-[rgba(255,255,255,0.02)] p-3">
      {/* Marca */}
      <div className="mb-4 flex items-center gap-3 px-2 pt-2">
        <div
          className="flex h-9 w-9 items-center justify-center rounded-[12px] text-[15px] font-bold"
          style={{
            background: "var(--accent)",
            color: "var(--accent-contrast)",
            boxShadow: "0 0 16px var(--accent-glow)",
          }}
        >
          M
        </div>
        <div className="leading-tight">
          <div className="text-[14px] font-semibold text-ink">MCP Manager</div>
          <div className="text-[11px] text-faint">v0.1.0</div>
        </div>
      </div>

      <nav className="flex flex-col gap-0.5">
        {NAV.map((item) => {
          const active = item.screen === screen;
          const Icon = item.icon;
          return (
            <button
              key={item.screen}
              onClick={() => setScreen(item.screen)}
              aria-current={active ? "page" : undefined}
              className="group flex items-center gap-3 rounded-[12px] px-3 py-2.5 text-[13.5px] font-medium transition-colors duration-[var(--dur)] ease-[var(--ease-out)]"
              style={
                active
                  ? {
                      background: "var(--accent-soft)",
                      color: "var(--accent-strong)",
                      boxShadow: "0 0 18px -2px var(--accent-glow)",
                    }
                  : { color: "var(--color-muted)" }
              }
              onMouseEnter={(e) => {
                if (!active)
                  e.currentTarget.style.background = "rgba(255,255,255,0.05)";
              }}
              onMouseLeave={(e) => {
                if (!active) e.currentTarget.style.background = "transparent";
              }}
            >
              <Icon
                size={18}
                strokeWidth={active ? 2.4 : 2}
                className="shrink-0"
              />
              <span className="flex-1 text-left">{item.label}</span>
              {item.badge ? (
                <span
                  className="flex h-5 min-w-5 items-center justify-center rounded-full px-1.5 text-[11px] font-semibold tabular-nums"
                  style={
                    active
                      ? {
                          background: "var(--accent)",
                          color: "var(--accent-contrast)",
                        }
                      : {
                          background: "var(--color-surface-3)",
                          color: "var(--color-ink-soft)",
                        }
                  }
                >
                  {item.badge}
                </span>
              ) : null}
            </button>
          );
        })}
      </nav>

      <div className="mt-auto px-3 pb-1 text-[11px] leading-relaxed text-faint">
        Administrá MCPs y Skills de Claude Desktop y Claude Code.
      </div>
    </aside>
  );
}
