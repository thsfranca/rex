import createREGL from "regl";
import { useEffect, useRef } from "react";
import ambientFrag from "../design-system/canvas/shaders/ambient.frag?raw";
import { useDecorativeMotionEnabled, useMotionOrchestrator } from "../design-system/motion/orchestrator";
import type { TurnPhase } from "../types";

interface Props {
  phase: TurnPhase;
}

function phaseIntensity(phase: TurnPhase): number {
  switch (phase) {
    case "generating":
      return 1;
    case "tool_running":
      return 0.85;
    case "tool_approval":
      return 0.7;
    case "terminal":
      return 0.35;
    default:
      return 0;
  }
}

export function AmbientCanvas({ phase }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const orchestrator = useMotionOrchestrator();
  const orchestratorRef = useRef(orchestrator);
  orchestratorRef.current = orchestrator;

  const enabled = useDecorativeMotionEnabled();
  const active =
    enabled &&
    (phase === "generating" ||
      phase === "tool_running" ||
      phase === "tool_approval" ||
      phase === "terminal");

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      return;
    }

    let regl: ReturnType<typeof createREGL> | null = null;
    try {
      regl = createREGL({
        canvas,
        attributes: { preserveDrawingBuffer: true },
      });
    } catch {
      return;
    }
    if (!regl) return;
    const gl = regl;
    let batterySlowdown = 1;

    const readBattery = async () => {
      try {
        const nav = navigator as Navigator & {
          getBattery?: () => Promise<{ charging: boolean; level: number }>;
        };
        if (!nav.getBattery) return;
        const battery = await nav.getBattery();
        batterySlowdown = battery.charging || battery.level > 0.2 ? 1 : 0.35;
      } catch {
        // Battery API unavailable.
      }
    };
    void readBattery();

    const draw = gl({
      frag: ambientFrag,
      vert: `
        precision mediump float;
        attribute vec2 position;
        void main() {
          gl_Position = vec4(position, 0.0, 1.0);
        }
      `,
      attributes: {
        position: [-1, -1, 1, -1, -1, 1, 1, 1],
      },
      count: 4,
      primitive: "triangle strip",
      uniforms: {
        uTime: () => orchestratorRef.current.clock * batterySlowdown,
        uIntensity: () =>
          Math.max(phaseIntensity(phase), orchestratorRef.current.intensity * 0.5),
        uFlowAngle: () => orchestratorRef.current.flowAngle,
        uResolution: ({ viewportWidth, viewportHeight }) => [viewportWidth, viewportHeight],
      },
    });

    let frame = 0;
    let running = true;

    const loop = () => {
      if (!running) return;
      if (active || orchestratorRef.current.isTyping) {
        draw();
      } else {
        gl.clear({ color: [0, 0, 0, 0] });
      }
      frame = requestAnimationFrame(loop);
    };
    loop();

    const resize = () => gl.poll();
    window.addEventListener("resize", resize);

    return () => {
      running = false;
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", resize);
      gl.destroy();
    };
  }, [phase, active]);

  return (
    <canvas
      id="ambient"
      ref={canvasRef}
      data-testid="ambient"
      data-motion-tier={active ? "cinematic" : "idle"}
      aria-hidden
    />
  );
}
