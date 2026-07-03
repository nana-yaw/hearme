// The accent picker feeds --color-logo-primary directly, but the STRONG
// action color (--color-background-ui: checked toggles, primary buttons) is a
// separate brand token that stayed pink whatever accent was chosen. Derive it
// from the accent instead — same hue, scaled to the brand's own
// strong-color relationship so white knobs and button text keep their
// contrast on any accent, including the light pastels in the preset row.

// Constants reverse-engineered from the brand pair itself: #faa2ca (accent)
// maps to #da5893 (strong) under exactly S×0.72, L clamped to 0.60 — so the
// default accent keeps today's look and every custom accent gets the same
// visual relationship.
const UI_MAX_LIGHTNESS = 0.6;
const UI_SATURATION_FACTOR = 0.72;
const UI_MAX_SATURATION = 0.75;

interface Hsl {
  h: number; // 0..360
  s: number; // 0..1
  l: number; // 0..1
}

const hexToHsl = (hex: string): Hsl | null => {
  const match = /^#([0-9a-f]{6})$/i.exec(hex.trim());
  if (!match) return null;
  const value = parseInt(match[1], 16);
  const r = ((value >> 16) & 0xff) / 255;
  const g = ((value >> 8) & 0xff) / 255;
  const b = (value & 0xff) / 255;

  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  const d = max - min;
  if (d === 0) return { h: 0, s: 0, l };

  const s = d / (1 - Math.abs(2 * l - 1));
  let h: number;
  if (max === r) h = ((g - b) / d) % 6;
  else if (max === g) h = (b - r) / d + 2;
  else h = (r - g) / d + 4;
  h *= 60;
  if (h < 0) h += 360;
  return { h, s, l };
};

const hslToHex = ({ h, s, l }: Hsl): string => {
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;
  let rgb: [number, number, number];
  if (h < 60) rgb = [c, x, 0];
  else if (h < 120) rgb = [x, c, 0];
  else if (h < 180) rgb = [0, c, x];
  else if (h < 240) rgb = [0, x, c];
  else if (h < 300) rgb = [x, 0, c];
  else rgb = [c, 0, x];
  const toByte = (channel: number) =>
    Math.round((channel + m) * 255)
      .toString(16)
      .padStart(2, "0");
  return `#${toByte(rgb[0])}${toByte(rgb[1])}${toByte(rgb[2])}`;
};

/**
 * Strong-action companion of the chosen accent. Falls back to the input when
 * it isn't a #rrggbb color (the Rust side validates before persisting, so
 * this is belt-and-braces only).
 */
export const deriveAccentUiColor = (accentHex: string): string => {
  const hsl = hexToHsl(accentHex);
  if (!hsl) return accentHex;
  return hslToHex({
    h: hsl.h,
    s: Math.min(hsl.s * UI_SATURATION_FACTOR, UI_MAX_SATURATION),
    l: Math.min(hsl.l, UI_MAX_LIGHTNESS),
  });
};
