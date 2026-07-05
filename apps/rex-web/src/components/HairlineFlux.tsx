import { useEffect, useRef } from "react";
import { useDecorativeMotionEnabled, useMotionOrchestrator } from "../design-system/motion/orchestrator";
import type { TurnPhase } from "../types";

interface Props {
  phase: TurnPhase;
  testId?: string;
}

function isActivePhase(phase: TurnPhase): boolean {
  return phase === "generating" || phase === "tool_running" || phase === "tool_approval";
}

export function HairlineFlux({ phase, testId = "hairline-flux" }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const orchestrator = useMotionOrchestrator();
  const orchestratorRef = useRef(orchestrator);
  orchestratorRef.current = orchestrator;

  const enabled = useDecorativeMotionEnabled();
  const active = isActivePhase(phase) && enabled;

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
      canvas.height = 3;
    };
    resize();
    window.addEventListener("resize", resize);

    const loop = () => {
      if (!running) return;
      const w = canvas.width;
      const h = canvas.height;
      ctx.clearRect(0, 0, w, h);

      if (active) {
        const t = orchestratorRef.current.clock;
        const sweep = ((t * 120) % (w + 80)) - 40;
        const gradient = ctx.createLinearGradient(sweep - 60, 0, sweep + 60, 0);
        gradient.addColorStop(0, "transparent");
        gradient.addColorStop(0.5, "var(--rex-hairline-flux)");
        gradient.addColorStop(1, "transparent");
        ctx.fillStyle = gradient;
        ctx.fillRect(0, 0, w, h);

        ctx.fillStyle = "var(--rex-hairline-flux)";
        for (let i = 0; i < 6; i += 1) {
          const x = (sweep + i * 14) % w;
          ctx.globalAlpha = 0.3 + 0.7 * Math.sin(t * 4 + i);
          ctx.fillRect(x, 0, 2, h);
        }
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
  }, [active, enabled]);

  return (
    <canvas
      ref={canvasRef}
      className="rex-hairline-flux"
      data-testid={testId}
      data-motion-tier={active ? "ambient" : "idle"}
      aria-hidden
    />
  );
}
