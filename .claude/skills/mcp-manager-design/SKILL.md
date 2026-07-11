---
name: mcp-manager-design
description: >-
  Design system de MCP Manager (dark glassmórfico premium, acento configurable
  en runtime con glow). Única fuente de verdad del estilo: tokens exactos en CSS
  variables, specs por componente y ejemplos de uso. Usar SIEMPRE al construir o
  revisar UI de este proyecto, y como referencia del subagente ui-reviewer.
---

# MCP Manager — Design System

Estética: **dark glassmórfico premium**, tipo settings panel moderno. Acento
**configurable en runtime** (default coral) que tiñe todos los elementos activos
con **glow**. Tipografía **Inter**.

Implementación: **Tailwind v4 CSS-first** (`@theme`, sin `tailwind.config.js`) +
CSS variables. Fuente: `src/styles/theme.css`. Paleta de acentos:
`src/types/theme.ts`. Componentes: `src/components/`.

## Principio no negociable

**El acento NUNCA se hardcodea en componentes.** Siempre vía las variables
`--accent*`. La pantalla Themes las reescribe en `:root` en caliente
(`src/store/theme.ts`) y toda la app recolorea. Si ves un `#FF6F61` (o cualquier
color de acento) literal dentro de un componente, es un bug.

## Tokens

Definidos en `@theme` (generan utilidades Tailwind `bg-*`, `text-*`, etc.):

| Token | Valor | Uso |
|-------|-------|-----|
| `--color-bg-from` | `#171D2B` | fondo (tope del gradiente) |
| `--color-bg-to` | `#1E2536` | fondo (base del gradiente) |
| `--color-surface` | `#1C2333` | base de tarjetas |
| `--color-surface-2` | `#222B3D` | elevado (inputs, selects, tags) |
| `--color-surface-3` | `#2A3346` | hover elevado / badge inactivo |
| `--color-track` | `#3A4254` | estado OFF (toggle/slider) |
| `--color-ink` | `#F4F6FB` | texto primario |
| `--color-ink-soft` | `#C7CEDD` | texto secundario claro |
| `--color-muted` | `#8A93A6` | subtítulos / secundario |
| `--color-faint` | `#5C6577` | helper / disabled |
| `--font-sans` | `"Inter", …` | toda la tipografía |
| `--radius-sm/md/lg/xl` | `10/14/18/22px` | radios |
| `--ease-out` | `cubic-bezier(0.22,1,0.36,1)` | easing |
| `--dur-fast/dur/dur-slow` | `150/200/250ms` | transiciones |

Runtime (en `:root`, fuera de `@theme` porque se reescriben):

| Token | Default (coral) | Uso |
|-------|-----------------|-----|
| `--accent` | `#FF6F61` | color base del acento |
| `--accent-strong` | `#FF7A6B` | variante clara (hover, texto activo) |
| `--accent-contrast` | `#1A0F0D` | texto sobre botón de acento |
| `--accent-glow` | `rgba(255,111,97,0.5)` | glow (box-shadow) |
| `--accent-soft` | `rgba(255,111,97,0.14)` | fondos tenues (sidebar activo) |
| `--line` | `rgba(255,255,255,0.08)` | bordes glass |
| `--line-strong` | `rgba(255,255,255,0.14)` | borde en hover |
| `--shadow-card` | ver theme.css | sombra difusa de tarjetas |

Los 9 acentos (azul, verde, amarillo, naranja, rojo, magenta, violeta, cian,
coral) viven en `ACCENTS` en `src/types/theme.ts`.

**Colores de estado** (semánticos, FIJOS — NO siguen el acento; en `:root`):

| Token | Uso |
|-------|-----|
| `--state-ok` / `--state-ok-glow` | verde: MCP sano, app detectada, dot OK |
| `--state-warn` | amarillo: advertencia (ej. error leyendo config de una app) |
| `--state-danger` / `--state-danger-glow` | rojo: dot/estado roto (comando no encontrado), relleno de botón danger |
| `--state-danger-text` | texto/ícono danger sobre fondo OSCURO (tags, contadores) |
| `--state-danger-contrast` | texto sobre RELLENO danger (ej. botón danger) — no confundir con `-text` |
| `--state-danger-soft` | fondo tenue de tags/cards danger |
| `--overlay` | scrim de fondo de modales |

Regla: los colores de estado también se usan SIEMPRE vía estos tokens, nunca
como hex literal en un componente (igual que el acento). La única diferencia es
que no cambian en runtime.

## Reglas visuales

- **Fondo**: nunca negro puro. Gradiente navy + dos halos radiales sutiles
  (mesh) definidos en `body` (`background-attachment: fixed`).
- **Tarjetas**: clase `.ds-card` — glass con `--radius-lg`, borde `--line`,
  `--shadow-card`, `backdrop-filter: blur(12px)`.
- **Glow**: TODO elemento activo/primario lleva `box-shadow: 0 0 12–20px
  var(--accent-glow)`: toggle on, item de sidebar activo, thumb de slider,
  opción activa del segmented, botón primario, swatch seleccionado.
- **Motion**: transiciones `--dur`/`--ease-out` en hover/focus/estado. Carga de
  pantalla con `.ds-rise` (reveal escalonado). Respetar
  `prefers-reduced-motion`.
- **Tipografía**: títulos de pantalla ~26px semibold `--color-ink`; títulos de
  card 15px semibold; subtítulos y helper en `--color-muted`/`--color-faint`.
- **Accesibilidad**: `:focus-visible` con outline de acento; contraste de texto
  ≥ 4.5:1.

## Componentes (`src/components/`)

Todos tipados y controlados (`value`/`onChange`). El glow sale de `--accent*`.

- **Sidebar** — 248px, borde derecho `--line`. Marca arriba (cuadro de acento
  con glow). Items: ícono lineal (lucide) + label + badge numérico redondo.
  Activo: fondo `--accent-soft`, texto `--accent-strong`, glow. Hover inactivo:
  `rgba(255,255,255,0.05)`. Navegación vía store `useUI`.
- **Card** — panel `.ds-card` con `title`/`subtitle` opcionales.
- **Toggle** — `role="switch"`. On: fondo `--accent` + glow. Off:
  `--color-track`. Thumb blanco que desliza.
- **Slider** — track 6px (`.ds-slider`), relleno `--accent` vía
  linear-gradient, thumb circular blanco con borde de acento y glow
  (`::-webkit-slider-thumb` / `::-moz-range-thumb` en theme.css).
- **SegmentedControl** — pill (`role="tablist"`), borde `--line`, fondo
  `--surface-2`. Opción activa: `--accent` + `--accent-contrast` + glow.
  Genérico sobre `<T extends string>`.
- **ColorSwatch** — círculo con glow de su propio color; seleccionado añade
  doble anillo (surface + color) y check. Solo pantalla Themes.
- **Select** — nativo estilado: `--surface-2`, borde `--line`, chevron lucide,
  focus borde `--accent`. Genérico sobre `<T extends string>`.

## Ejemplos de uso

```tsx
import { Card, Toggle, SegmentedControl } from "@/components";

<Card title="Seguridad" subtitle="Aplica a todo config externo">
  <Toggle checked={on} onChange={setOn} label="Backup automático" />
</Card>

<SegmentedControl
  value={filter}
  onChange={setFilter}
  options={[
    { value: "all", label: "Todas" },
    { value: "desktop", label: "Claude Desktop" },
  ]}
/>
```

Elemento primario con glow (patrón para botones/estados activos):

```tsx
<button
  style={{
    background: "var(--accent)",
    color: "var(--accent-contrast)",
    boxShadow: "0 0 18px var(--accent-glow)",
  }}
>
  Agregar MCP
</button>
```

## Al construir/revisar UI

1. Reutilizá los componentes de `src/components/`; no dupliques estilos.
2. Layout/spacing/texto con utilidades Tailwind; acento/glow con
   `var(--accent*)` (inline o clases `.ds-*`).
3. Cero colores de acento literales en componentes.
4. Verificá glow en todos los estados activos y foco visible.
5. Al cerrar la tarea de UI, corré el subagente **ui-reviewer** (audita contra
   esta skill).
