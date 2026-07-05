import { useEffect, useRef } from "react";
import {
  createParticlePool,
  spawnBurst,
  stepParticles,
} from "../design-system/physics/particle-euler";
import type { Particle2D } from "../design-system/physics/types";
import { useMotionOrchestrator } from "../design-system/motion/orchestrator";
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

  const active = phaseActive(phase);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let frame = 0;
    let running = true;
    let last = performance.now();

    const resize = () => {
      canvas.width = window.innerWidth;
      canvas.height = window.innerHeight;
    };
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
        spawnBurst(pool, canvas.width * 0.5, canvas.height * 0.32, 16, Math.PI * 0.55, orch.flowAngle);
      }

      if (active) {
        stepParticles(pool, dt, 2.8, 18);
      }

      ctx.clearRect(0, 0, canvas.width, canvas.height);
      const fill =
        getComputedStyle(document.documentElement).getPropertyValue("--rex-particle-fill").trim() ||
        getComputedStyle(document.documentElement).getPropertyValue("--rex-accent-glow").trim();
      for (const p of pool) {
        if (!p.active) continue;
        const alpha = (p.life / p.maxLife) * 0.75;
        ctx.beginPath();
        ctx.arc(p.x, p.y, 2 + alpha * 2, 0, Math.PI * 2);
        ctx.globalAlpha = alpha;
        ctx.fillStyle = fill;
        ctx.fill();
      }
      ctx.globalAlpha = 1;

      frame = requestAnimationFrame(loop);
    };
    frame = requestAnimationFrame(loop);

    return () => {
      running = false;
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", resize);
    };
  }, [phase, active]);

  return (
    <canvas
      id="particles"
      ref={canvasRef}
      data-testid="particles"
      data-motion-tier={active ? "cinematic" : "idle"}
      aria-hidden
    />
  );
}
