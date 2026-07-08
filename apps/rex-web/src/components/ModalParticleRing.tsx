import { useEffect, useRef } from "react";
import { createParticleRenderer } from "../design-system/canvas/particle-renderer";
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

    const renderer = (() => {
      try {
        return createParticleRenderer(canvas, POOL_SIZE);
      } catch {
        return null;
      }
    })();
    if (!renderer) return;
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
      stepParticles(poolRef.current, dt, 3.2, 0);
      renderer.updatePool(poolRef.current);
      renderer.draw(true);
      frame = requestAnimationFrame(loop);
    };
    frame = requestAnimationFrame(loop);

    return () => {
      running = false;
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", resize);
      renderer.destroy();
    };
  }, [active, enabled]);

  if (!active || !enabled) return null;

  return (
    <canvas
      ref={canvasRef}
      className="rex-modal-particles"
      data-testid="modal-particles"
      data-renderer="regl"
      data-motion-tier="cinematic"
      aria-hidden
    />
  );
}
