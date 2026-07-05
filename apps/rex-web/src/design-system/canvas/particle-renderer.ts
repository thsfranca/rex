import createREGL, { type Regl } from "regl";
import type { Particle2D } from "../physics/types";
import { parseCssColorToRgba } from "./parse-css-color";
import particleFrag from "./shaders/particle.frag?raw";
import particleVert from "./shaders/particle.vert?raw";

const QUAD = [-1, -1, 1, -1, -1, 1, 1, 1];

export interface ParticleRenderer {
  draw(active: boolean): void;
  poll(): void;
  updatePool(pool: Particle2D[]): void;
  destroy(): void;
}

export function createParticleRenderer(
  canvas: HTMLCanvasElement,
  poolSize: number,
): ParticleRenderer {
  const regl: Regl = createREGL({
    canvas,
    attributes: {
      alpha: true,
      premultipliedAlpha: true,
      antialias: true,
      preserveDrawingBuffer: true,
    },
  });

  const offsets = new Float32Array(poolSize * 2);
  const sizes = new Float32Array(poolSize);
  const alphas = new Float32Array(poolSize);

  const offsetBuffer = regl.buffer(offsets);
  const sizeBuffer = regl.buffer(sizes);
  const alphaBuffer = regl.buffer(alphas);

  const draw = regl({
    frag: particleFrag,
    vert: particleVert,
    attributes: {
      position: QUAD,
      offset: { buffer: offsetBuffer, divisor: 1 },
      size: { buffer: sizeBuffer, divisor: 1 },
      alpha: { buffer: alphaBuffer, divisor: 1 },
    },
    uniforms: {
      uResolution: ({ viewportWidth, viewportHeight }) => [viewportWidth, viewportHeight],
      uColor: () => readParticleColor(),
    },
    instances: poolSize,
    count: 4,
    primitive: "triangle strip",
    blend: {
      enable: true,
      func: {
        srcRGB: "src alpha",
        srcAlpha: "one",
        dstRGB: "one minus src alpha",
        dstAlpha: "one minus src alpha",
      },
    },
    depth: { enable: false },
  });

  return {
    poll: () => regl.poll(),
    updatePool(pool: Particle2D[]) {
      for (let i = 0; i < poolSize; i += 1) {
        const particle = pool[i];
        if (!particle?.active) {
          offsets[i * 2] = 0;
          offsets[i * 2 + 1] = 0;
          sizes[i] = 0;
          alphas[i] = 0;
          continue;
        }
        const lifeRatio = particle.life / particle.maxLife;
        offsets[i * 2] = particle.x;
        offsets[i * 2 + 1] = particle.y;
        sizes[i] = 2 + lifeRatio * 2.5;
        alphas[i] = lifeRatio * 0.75;
      }
      offsetBuffer(offsets);
      sizeBuffer(sizes);
      alphaBuffer(alphas);
    },
    draw(active: boolean) {
      if (active) {
        draw();
        return;
      }
      regl.clear({ color: [0, 0, 0, 0], depth: 1 });
    },
    destroy() {
      regl.destroy();
    },
  };
}

function readParticleColor(): [number, number, number, number] {
  if (typeof document === "undefined") return [0.55, 0.62, 1, 0.75];
  const root = getComputedStyle(document.documentElement);
  const fill =
    root.getPropertyValue("--rex-particle-fill").trim() ||
    root.getPropertyValue("--rex-accent-glow").trim();
  return parseCssColorToRgba(fill);
}
