import type { Inventory } from "../types/inventory";

const DESKTOP_CFG =
  "~/Library/Application Support/Claude/claude_desktop_config.json";
const CODE_CFG = "~/.claude.json";

/**
 * Mock SOLO para desarrollo en browser (`pnpm dev`), donde no hay backend
 * Tauri. La app real (`pnpm tauri dev`) siempre usa `get_inventory`. Sirve
 * para revisar el diseño del inventario sin la ventana nativa.
 */
export const MOCK_INVENTORY: Inventory = {
  apps: [
    {
      id: "claude-desktop",
      label: "Claude Desktop",
      installed: true,
      configPath: DESKTOP_CFG,
    },
    {
      id: "claude-code",
      label: "Claude Code",
      installed: true,
      configPath: CODE_CFG,
    },
  ],
  installations: [
    {
      name: "context7",
      app: "claude-desktop",
      scope: "user",
      transport: "stdio",
      command: "npx",
      args: ["-y", "@upstash/context7-mcp"],
      envKeys: [],
      status: "ok",
      configPath: DESKTOP_CFG,
      enabled: true,
    },
    {
      name: "context7",
      app: "claude-code",
      scope: "user",
      transport: "stdio",
      command: "npx",
      args: ["-y", "@upstash/context7-mcp"],
      envKeys: [],
      status: "ok",
      configPath: CODE_CFG,
      enabled: true,
    },
    {
      name: "playwright",
      app: "claude-code",
      scope: "user",
      transport: "stdio",
      command: "npx",
      args: ["-y", "@playwright/mcp@latest"],
      envKeys: [],
      status: "ok",
      configPath: CODE_CFG,
      enabled: true,
    },
    {
      name: "markitdown",
      app: "claude-desktop",
      scope: "user",
      transport: "stdio",
      command: "markitdown-mcp",
      args: [],
      envKeys: ["MARKITDOWN_ENABLE_PLUGINS"],
      status: "command_not_found",
      configPath: DESKTOP_CFG,
      enabled: true,
    },
    {
      name: "notion",
      app: "claude-desktop",
      scope: "user",
      transport: "http",
      url: "https://mcp.notion.com/mcp",
      args: [],
      envKeys: ["NOTION_TOKEN"],
      status: "ok",
      configPath: DESKTOP_CFG,
      enabled: true,
    },
    {
      name: "local-tool",
      app: "claude-code",
      scope: "project",
      projectPath: "~/www/ia-tools/others/mcp-manager",
      transport: "stdio",
      command: "./bin/local-tool",
      args: [],
      envKeys: ["TOKEN"],
      status: "disabled",
      configPath: "~/www/ia-tools/others/mcp-manager/.mcp.json",
      enabled: false,
    },
  ],
};
