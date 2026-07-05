import { describe, expect, it } from "vitest";
import { createSpringState, springToValue, stepSpring } from "./rk4-spring";

describe("rk4-spring", () => {
  it("settles near target", () => {
    const values = springToValue(0, 1, { stiffness: 300, damping: 20, precision: 0.001 });
    const last = values[values.length - 1];
    expect(last).toBeCloseTo(1, 2);
  });

  it("steps toward target each frame", () => {
    let state = createSpringState(0);
    state = stepSpring(state, 1, 1 / 60, { stiffness: 300, damping: 20 });
    expect(state.position).toBeGreaterThan(0);
    expect(state.atRest).toBe(false);
  });
});
