import { useEffect, useRef } from "react";
import { useDecorativeMotionEnabled, useMotionOrchestrator } from "../design-system/motion/orchestrator";

interface Props {
  working: boolean;
  anchorId?: string;
}

export function StatusOrbit({ working, anchorId = "status-dot" }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const enabled = useDecorativeMotionEnabled();
  const orchestrator = useMotionOrchestrator();
  const orchestratorRef = useRef(orchestrator);
  orchestratorRef.current = orchestrator;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !working || !enabled) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let frame = 0;
    let running = true;

    const layout = () => {
      const anchor = document.getElementById(anchorId);
      if (!anchor) return false;
      const rect = anchor.getBoundingClientRect();
      const size = 28;
      canvas.width = size;
      canvas.height = size;
      canvas.style.left = `${rect.left + rect.width / 2 - size / 2}px`;
      canvas.style.top = `${rect.top + rect.height / 2 - size / 2}px`;
      return true;
    };

    const loop = () => {
      if (!running || !layout()) {
        frame = requestAnimationFrame(loop);
        return;
      }
      const t = orchestratorRef.current.clock;
      const cx = canvas.width / 2;
      const cy = canvas.height / 2;
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      const fill =
        getComputedStyle(document.documentElement).getPropertyValue("--rex-accent-glow").trim();
      for (let i = 0; i < 5; i += 1) {
        const angle = t * 2.2 + (i * Math.PI * 2) / 5;
        const r = 8 + Math.sin(t * 3 + i) * 2;
        const x = cx + Math.cos(angle) * r;
        const y = cy + Math.sin(angle) * r;
        ctx.beginPath();
        ctx.arc(x, y, 1.5, 0, Math.PI * 2);
        ctx.globalAlpha = 0.45 + 0.35 * Math.sin(t * 4 + i);
        ctx.fillStyle = fill;
        ctx.fill();
      }
      ctx.globalAlpha = 1;
      frame = requestAnimationFrame(loop);
    };
    loop();

    window.addEventListener("resize", layout);
    return () => {
      running = false;
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", layout);
    };
  }, [working, enabled, anchorId]);

  if (!working || !enabled) return null;

  return (
    <canvas
      ref={canvasRef}
      className="rex-status-orbit"
      data-testid="status-orbit"
      data-motion-tier="ambient"
      aria-hidden
    />
  );
}
