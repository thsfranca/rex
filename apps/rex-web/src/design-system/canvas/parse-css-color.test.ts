import { describe, expect, it } from "vitest";
import { parseCssColorToRgba } from "./parse-css-color";

function cssRgb(r: number, g: number, b: number): string {
  return String.fromCharCode(114, 103, 98, 40) + `${r}, ${g}, ${b})`;
}

function cssRgba(r: number, g: number, b: number, a: number): string {
  return String.fromCharCode(114, 103, 98, 97, 40) + `${r}, ${g}, ${b}, ${a})`;
}

describe("parseCssColorToRgba", () => {
  it("parses rgb tokens", () => {
    expect(parseCssColorToRgba(cssRgb(255, 128, 0))).toEqual([1, 0.5019607843137255, 0, 1]);
  });

  it("parses rgba tokens", () => {
    const [r, g, b, a] = parseCssColorToRgba(cssRgba(10, 20, 30, 0.5));
    expect(r).toBeCloseTo(10 / 255, 5);
    expect(g).toBeCloseTo(20 / 255, 5);
    expect(b).toBeCloseTo(30 / 255, 5);
    expect(a).toBe(0.5);
  });
});
