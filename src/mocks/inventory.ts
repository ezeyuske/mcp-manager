import type { Inventory } from "../types/inventory";

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
      configPath:
        "~/Library/Application Support/Claude/claude_desktop_config.json",
    },
    {
      id: "claude-code",
      label: "Claude Code",
      installed: true,
      configPath: "~/.claude.json",
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
    },
    {
      name: "markitdown",
      app: "claude-code",
      scope: "user",
      transport: "stdio",
      command: "markitdown-mcp",
      args: [],
      envKeys: ["MARKITDOWN_ENABLE_PLUGINS"],
      status: "command_not_found",
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
    },
    {
      name: "linear",
      app: "claude-code",
      scope: "project",
      projectPath: "~/www/ia-tools/others/mcp-manager",
      transport: "sse",
      url: "https://mcp.linear.app/sse",
      args: [],
      envKeys: [],
      status: "ok",
    },
  ],
};
