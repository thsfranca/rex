import type { SpringConfig, SpringState } from "./types";

const DEFAULTS: Required<SpringConfig> = {
  mass: 1,
  stiffness: 300,
  damping: 20,
  precision: 0.001,
};

function acceleration(
  position: number,
  velocity: number,
  target: number,
  mass: number,
  stiffness: number,
  damping: number,
): number {
  return (-stiffness * (position - target) - damping * velocity) / mass;
}

function rk4Step(
  position: number,
  velocity: number,
  target: number,
  dt: number,
  mass: number,
  stiffness: number,
  damping: number,
): [number, number] {
  const a = (x: number, v: number) => acceleration(x, v, target, mass, stiffness, damping);

  const v1 = velocity;
  const a1 = a(position, v1);
  const v2 = velocity + 0.5 * a1 * dt;
  const a2 = a(position + 0.5 * v1 * dt, v2);
  const v3 = velocity + 0.5 * a2 * dt;
  const a3 = a(position + 0.5 * v2 * dt, v3);
  const v4 = velocity + a3 * dt;
  const a4 = a(position + v3 * dt, v4);

  const nextPosition = position + (dt / 6) * (v1 + 2 * v2 + 2 * v3 + v4);
  const nextVelocity = velocity + (dt / 6) * (a1 + 2 * a2 + 2 * a3 + a4);
  return [nextPosition, nextVelocity];
}

export function createSpringState(initial = 0): SpringState {
  return { position: initial, velocity: 0, atRest: false };
}

export function stepSpring(
  state: SpringState,
  target: number,
  dt: number,
  config: SpringConfig = {},
): SpringState {
  const { mass, stiffness, damping, precision } = { ...DEFAULTS, ...config };
  const [position, velocity] = rk4Step(
    state.position,
    state.velocity,
    target,
    dt,
    mass,
    stiffness,
    damping,
  );

  const kinetic = 0.5 * mass * velocity * velocity;
  const potential = 0.5 * stiffness * (position - target) ** 2;
  const atRest = kinetic + potential < precision;

  return {
    position: atRest ? target : position,
    velocity: atRest ? 0 : velocity,
    atRest,
  };
}

export function springToValue(
  from: number,
  to: number,
  config: SpringConfig = {},
  maxSteps = 600,
  dt = 1 / 60,
): number[] {
  const values: number[] = [from];
  let state = createSpringState(from);
  for (let i = 0; i < maxSteps; i += 1) {
    state = stepSpring(state, to, dt, config);
    values.push(state.position);
    if (state.atRest) break;
  }
  return values;
}

export function readSpringTokens(): Required<Pick<SpringConfig, "stiffness" | "damping">> {
  if (typeof document === "undefined") {
    return { stiffness: 300, damping: 20 };
  }
  const root = getComputedStyle(document.documentElement);
  const stiffness = parseFloat(root.getPropertyValue("--rex-spring-modal-stiffness")) || 300;
  const damping = parseFloat(root.getPropertyValue("--rex-spring-modal-damping")) || 20;
  return { stiffness, damping };
}
