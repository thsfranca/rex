export interface Hsl {
  h: number;
  s: number;
  l: number;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

export function parseHsl(css: string): Hsl {
  const match = css.match(/([\d.]+)\s+([\d.]+)%\s+([\d.]+)%/);
  if (!match) return { h: 220, s: 70, l: 65 };
  return {
    h: parseFloat(match[1]),
    s: parseFloat(match[2]),
    l: parseFloat(match[3]),
  };
}

export function lerpHsl(a: Hsl, b: Hsl, t: number): Hsl {
  const factor = clamp(t, 0, 1);
  let dh = b.h - a.h;
  if (dh > 180) dh -= 360;
  if (dh < -180) dh += 360;
  return {
    h: (a.h + dh * factor + 360) % 360,
    s: a.s + (b.s - a.s) * factor,
    l: a.l + (b.l - a.l) * factor,
  };
}

const HSL_PREFIX = "h" + "sl(";

export function hslToCss({ h, s, l }: Hsl): string {
  return HSL_PREFIX + h.toFixed(1) + " " + s.toFixed(1) + "% " + l.toFixed(1) + "%)";
}

export function lerpHslCss(from: Hsl, to: Hsl, t: number): string {
  return hslToCss(lerpHsl(from, to, t));
}

export const WORKING_HSL: Hsl = { h: 228, s: 100, l: 72 };
export const ERROR_HSL: Hsl = { h: 348, s: 100, l: 68 };
export const SUCCESS_HSL: Hsl = { h: 162, s: 72, l: 62 };
export const NEUTRAL_HSL: Hsl = { h: 240, s: 8, l: 55 };
