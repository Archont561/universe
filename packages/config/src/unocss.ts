import lucide from "@iconify-json/lucide/icons.json" with { type: "json" };
import simpleIcons from "@iconify-json/simple-icons/icons.json" with { type: "json" };
import {
  defineConfig,
  mergeConfigs,
  presetIcons,
  presetTypography,
  presetWind3,
  transformerDirectives,
  transformerVariantGroup,
  type UserConfig,
} from "unocss";

export const unocssTheme = {
  colors: {
    accent: "var(--cu-accent, #10b981)",
    surface: {
      DEFAULT: "var(--cu-surface, #0f172a)",
      elevated: "var(--cu-surface-elevated, #1e293b)",
      border: "var(--cu-surface-border, #334155)",
    },
  },
  fontFamily: {
    sans: "Inter, ui-sans-serif, system-ui, -apple-system, sans-serif",
    mono: "'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, monospace",
  },
} as const;

export const unocssShortcuts: Record<string, string> = {
  "cu-badge":
    "inline-flex items-center gap-1.5 rounded-full border border-slate-700 bg-slate-800/80 px-3 py-1 text-xs font-medium text-slate-200",
  "cu-card": "rounded-xl border border-slate-800 bg-slate-900/80 p-6 shadow-lg backdrop-blur-sm",
};

/**
 * Create a self-contained offline UnoCSS configuration shared by SvelteKit SSG packages.
 * All icons compile to inline CSS data URIs with zero external CDN requests.
 */
export function createUnoConfig(overrides: UserConfig = {}): UserConfig {
  const base = defineConfig({
    theme: unocssTheme,
    shortcuts: unocssShortcuts,
    presets: [
      presetWind3(),
      presetIcons({
        scale: 1.1,
        extraProperties: {
          display: "inline-block",
          "vertical-align": "middle",
        },
        collections: {
          lucide: () => lucide,
          "simple-icons": () => simpleIcons,
        },
      }),
      presetTypography(),
    ],
    transformers: [transformerDirectives(), transformerVariantGroup()],
  });

  return mergeConfigs([base, overrides]);
}

export default createUnoConfig;
