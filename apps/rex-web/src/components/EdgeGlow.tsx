import { useEffect, useRef } from "react";
import { motionOrchestrator, useDecorativeMotionEnabled } from "../design-system/motion/orchestrator";

interface Props {
  active: boolean;
}

function readGlowColor(): string {
  if (typeof document === "undefined") return "";
  return getComputedStyle(document.documentElement).getPropertyValue("--rex-glow-working").trim();
}

export function EdgeGlow({ active }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const enabled = useDecorativeMotionEnabled();
  const show = active && enabled;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let frame = 0;
    let running = true;

    const resize = () => {
      const parent = canvas.parentElement;
      if (!parent) return;
      canvas.width = parent.clientWidth;
      canvas.height = 4;
    };
    resize();
    window.addEventListener("resize", resize);

    const loop = () => {
      if (!running) return;
      const w = canvas.width;
      const h = canvas.height;
      ctx.clearRect(0, 0, w, h);

      const orch = motionOrchestrator.getSnapshot();
      const glow = readGlowColor();
      if ((show || orch.isTyping) && glow) {
        const t = orch.clock;
        const pulse = 0.35 + 0.65 * (Math.sin(t * 3) * 0.5 + 0.5);
        const gradient = ctx.createLinearGradient(0, 0, w, 0);
        gradient.addColorStop(0, "transparent");
        gradient.addColorStop(0.4, glow);
        gradient.addColorStop(0.6, glow);
        gradient.addColorStop(1, "transparent");
        ctx.globalAlpha = pulse * 0.55;
        ctx.fillStyle = gradient;
        ctx.fillRect(0, 0, w, h);
        ctx.globalAlpha = 1;
      }

      frame = requestAnimationFrame(loop);
    };
    loop();

    return () => {
      running = false;
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", resize);
    };
  }, [show]);

  return (
    <canvas
      ref={canvasRef}
      className="rex-edge-glow"
      data-testid="edge-glow"
      data-motion-tier={show ? "active" : "idle"}
      aria-hidden
    />
  );
}
