export function parseCssColorToRgba(css: string): [number, number, number, number] {
  const value = css.trim();
  if (!value) return [0.55, 0.62, 1, 0.75];

  const rgbMatch = value.match(/rgba?\(([^)]+)\)/i);
  if (rgbMatch) {
    const parts = rgbMatch[1].split(",").map((part) => part.trim());
    const r = Number(parts[0]) / 255;
    const g = Number(parts[1]) / 255;
    const b = Number(parts[2]) / 255;
    const a = parts[3] !== undefined ? Number(parts[3]) : 1;
    return [r, g, b, Number.isFinite(a) ? a : 1];
  }

  const hslMatch = value.match(/hsla?\(([^)]+)\)/i);
  if (hslMatch) {
    const parts = hslMatch[1].split(/[\s,/]+/).filter(Boolean);
    const h = Number(parts[0]) / 360;
    const s = Number(parts[1].replace("%", "")) / 100;
    const l = Number(parts[2].replace("%", "")) / 100;
    const a = parts[3] !== undefined ? Number(parts[3]) : 1;
    const rgb = hslToRgb(h, s, l);
    return [...rgb, Number.isFinite(a) ? a : 1];
  }

  return [0.55, 0.62, 1, 0.75];
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  if (s === 0) return [l, l, l];
  const hue = ((h % 1) + 1) % 1;
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  return [hueChannel(p, q, hue + 1 / 3), hueChannel(p, q, hue), hueChannel(p, q, hue - 1 / 3)];
}

function hueChannel(p: number, q: number, t: number): number {
  let channel = t;
  if (channel < 0) channel += 1;
  if (channel > 1) channel -= 1;
  if (channel < 1 / 6) return p + (q - p) * 6 * channel;
  if (channel < 1 / 2) return q;
  if (channel < 2 / 3) return p + (q - p) * (2 / 3 - channel) * 6;
  return p;
}
