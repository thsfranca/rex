import { useEffect, useRef } from "react";
import { createParticleRenderer } from "../design-system/canvas/particle-renderer";
import {
  createParticlePool,
  spawnBurst,
  stepParticles,
} from "../design-system/physics/particle-euler";
import type { Particle2D } from "../design-system/physics/types";
import { useDecorativeMotionEnabled, useMotionOrchestrator } from "../design-system/motion/orchestrator";
import type { TurnPhase } from "../types";

interface Props {
  phase: TurnPhase;
}

const POOL_SIZE = 160;

function phaseActive(phase: TurnPhase): boolean {
  return (
    phase === "generating" ||
    phase === "tool_running" ||
    phase === "tool_approval" ||
    phase === "terminal"
  );
}

export function ParticleField({ phase }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const poolRef = useRef<Particle2D[]>(createParticlePool(POOL_SIZE));
  const orchestrator = useMotionOrchestrator();
  const orchestratorRef = useRef(orchestrator);
  orchestratorRef.current = orchestrator;
  const lastTickRef = useRef(orchestrator.streamTick);

  const enabled = useDecorativeMotionEnabled();
  const active = phaseActive(phase) && enabled;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    const renderer = createParticleRenderer(canvas, POOL_SIZE);
    let frame = 0;
    let running = true;
    let last = performance.now();

    const resize = () => renderer.poll();
    resize();
    window.addEventListener("resize", resize);

    const loop = (now: number) => {
      if (!running) return;
      const dt = Math.min(0.032, (now - last) / 1000);
      last = now;

      const pool = poolRef.current;
      const orch = orchestratorRef.current;

      if (orch.streamTick !== lastTickRef.current) {
        lastTickRef.current = orch.streamTick;
        const { width, height } = canvas.getBoundingClientRect();
        spawnBurst(pool, width * 0.5, height * 0.32, 16, Math.PI * 0.55, orch.flowAngle);
      }

      if (active) {
        stepParticles(pool, dt, 2.8, 18);
      }

      renderer.updatePool(pool);
      renderer.draw(active);
      frame = requestAnimationFrame(loop);
    };
    frame = requestAnimationFrame(loop);

    return () => {
      running = false;
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", resize);
      renderer.destroy();
    };
  }, [phase, active]);

  return (
    <canvas
      id="particles"
      ref={canvasRef}
      data-testid="particles"
      data-renderer="regl"
      data-motion-tier={active ? "cinematic" : "idle"}
      aria-hidden
    />
  );
}
