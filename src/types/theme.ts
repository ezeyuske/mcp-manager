/**
 * Paleta de acentos configurables en runtime (pantalla Themes — dogfooding).
 * Cada acento define las variables que el store escribe en :root.
 * El default del proyecto es `coral`.
 */
export type AccentKey =
  | "coral"
  | "blue"
  | "green"
  | "yellow"
  | "orange"
  | "red"
  | "magenta"
  | "violet"
  | "cyan";

export interface Accent {
  key: AccentKey;
  label: string;
  /** color base */
  accent: string;
  /** variante clara (hover/gradiente) */
  strong: string;
  /** texto sobre el color (botón primario) */
  contrast: string;
  /** rgba para el glow (~0.5 alpha) */
  glow: string;
  /** rgba tenue para fondos (sidebar activo) */
  soft: string;
}

export const ACCENTS: Record<AccentKey, Accent> = {
  coral: {
    key: "coral",
    label: "Coral",
    accent: "#ff6f61",
    strong: "#ff7a6b",
    contrast: "#1a0f0d",
    glow: "rgba(255, 111, 97, 0.5)",
    soft: "rgba(255, 111, 97, 0.14)",
  },
  blue: {
    key: "blue",
    label: "Azul",
    accent: "#4c8dff",
    strong: "#63a0ff",
    contrast: "#08111f",
    glow: "rgba(76, 141, 255, 0.5)",
    soft: "rgba(76, 141, 255, 0.14)",
  },
  green: {
    key: "green",
    label: "Verde",
    accent: "#33c98a",
    strong: "#49d99b",
    contrast: "#06160f",
    glow: "rgba(51, 201, 138, 0.5)",
    soft: "rgba(51, 201, 138, 0.14)",
  },
  yellow: {
    key: "yellow",
    label: "Amarillo",
    accent: "#f2c14e",
    strong: "#f7cf6b",
    contrast: "#1c1503",
    glow: "rgba(242, 193, 78, 0.5)",
    soft: "rgba(242, 193, 78, 0.14)",
  },
  orange: {
    key: "orange",
    label: "Naranja",
    accent: "#ff9040",
    strong: "#ffa25e",
    contrast: "#1d1005",
    glow: "rgba(255, 144, 64, 0.5)",
    soft: "rgba(255, 144, 64, 0.14)",
  },
  red: {
    key: "red",
    label: "Rojo",
    accent: "#f2555a",
    strong: "#f76d72",
    contrast: "#1e0708",
    glow: "rgba(242, 85, 90, 0.5)",
    soft: "rgba(242, 85, 90, 0.14)",
  },
  magenta: {
    key: "magenta",
    label: "Magenta",
    accent: "#e666cf",
    strong: "#ee7fda",
    contrast: "#1c0819",
    glow: "rgba(230, 102, 207, 0.5)",
    soft: "rgba(230, 102, 207, 0.14)",
  },
  violet: {
    key: "violet",
    label: "Violeta",
    accent: "#9b7bff",
    strong: "#ac90ff",
    contrast: "#100a24",
    glow: "rgba(155, 123, 255, 0.5)",
    soft: "rgba(155, 123, 255, 0.14)",
  },
  cyan: {
    key: "cyan",
    label: "Cian",
    accent: "#2fd4d4",
    strong: "#4fe0e0",
    contrast: "#041717",
    glow: "rgba(47, 212, 212, 0.5)",
    soft: "rgba(47, 212, 212, 0.14)",
  },
};

/** Orden de presentación en la grilla de swatches. */
export const ACCENT_ORDER: AccentKey[] = [
  "blue",
  "green",
  "yellow",
  "orange",
  "red",
  "magenta",
  "violet",
  "cyan",
  "coral",
];
