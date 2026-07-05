import { useEffect, useRef } from "react";
import {
  createParticlePool,
  spawnBurst,
  stepParticles,
} from "../design-system/physics/particle-euler";
import type { Particle2D } from "../design-system/physics/types";
import { useDecorativeMotionEnabled } from "../design-system/motion/orchestrator";

interface Props {
  active: boolean;
}

const POOL_SIZE = 48;

export function ModalParticleRing({ active }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const poolRef = useRef<Particle2D[]>(createParticlePool(POOL_SIZE));
  const spawnedRef = useRef(false);
  const enabled = useDecorativeMotionEnabled();

  useEffect(() => {
    if (!active) {
      spawnedRef.current = false;
      return;
    }
    if (spawnedRef.current) return;
    spawnedRef.current = true;
    const modal = document.querySelector('[data-testid="modal"]');
    if (!modal) return;
    const rect = modal.getBoundingClientRect();
    spawnBurst(poolRef.current, rect.left + rect.width / 2, rect.top + rect.height / 2, 20, Math.PI * 2, -Math.PI / 2);
  }, [active]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !active || !enabled) return;
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
      stepParticles(poolRef.current, dt, 3.2, 0);
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      const fill =
        getComputedStyle(document.documentElement).getPropertyValue("--rex-particle-fill").trim();
      for (const p of poolRef.current) {
        if (!p.active) continue;
        ctx.beginPath();
        ctx.arc(p.x, p.y, 2, 0, Math.PI * 2);
        ctx.globalAlpha = (p.life / p.maxLife) * 0.8;
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
  }, [active, enabled]);

  if (!active || !enabled) return null;

  return (
    <canvas
      ref={canvasRef}
      className="rex-modal-particles"
      data-testid="modal-particles"
      data-motion-tier="cinematic"
      aria-hidden
    />
  );
}
