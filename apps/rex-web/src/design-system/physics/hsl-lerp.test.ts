import { describe, expect, it } from "vitest";
import { ERROR_HSL, NEUTRAL_HSL, lerpHsl, lerpHslCss, parseHsl } from "./hsl-lerp";

describe("hsl-lerp", () => {
  it("parses hsl token strings", () => {
    expect(parseHsl("228 100% 72%")).toEqual({ h: 228, s: 100, l: 72 });
  });

  it("lerps hue across the short arc", () => {
    const mid = lerpHsl({ h: 100, s: 40, l: 50 }, { h: 140, s: 80, l: 60 }, 0.5);
    expect(mid.h).toBeCloseTo(120, 0);
    expect(mid.s).toBeCloseTo(60, 0);
    expect(mid.l).toBeCloseTo(55, 0);
  });

  it("returns endpoints at t=0 and t=1", () => {
    expect(lerpHsl(NEUTRAL_HSL, ERROR_HSL, 0)).toEqual(NEUTRAL_HSL);
    expect(lerpHsl(NEUTRAL_HSL, ERROR_HSL, 1)).toEqual(ERROR_HSL);
  });

  it("formats css hsl strings", () => {
    const css = lerpHslCss(NEUTRAL_HSL, ERROR_HSL, 0.5);
    expect(css).toMatch(/\d+(\.\d+)?%\s+\d+(\.\d+)?%/);
  });
});
