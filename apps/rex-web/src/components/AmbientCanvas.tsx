import createREGL from "regl";
import { useEffect, useRef } from "react";
import type { TurnPhase } from "../types";

interface Props {
  phase: TurnPhase;
}

export function AmbientCanvas({ phase }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      return;
    }

    const regl = createREGL({ canvas });
    let t = 0;
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

    const draw = regl({
      frag: `
        precision mediump float;
        uniform float uTime;
        uniform vec2 uResolution;
        void main() {
          vec2 uv = gl_FragCoord.xy / uResolution;
          float pulse = 0.08 + 0.04 * sin(uTime * 1.4 + uv.x * 6.0);
          vec3 color = vec3(0.35, 0.45, 0.85) * pulse;
          gl_FragColor = vec4(color, pulse);
        }
      `,
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
        uTime: () => t,
        uResolution: ({ viewportWidth, viewportHeight }) => [viewportWidth, viewportHeight],
      },
    });

    let frame = 0;
    let running = true;
    const active =
      phase === "generating" || phase === "tool_running" || phase === "tool_approval";

    const loop = () => {
      if (!running) return;
      if (active) {
        t += 0.016 * batterySlowdown;
        draw();
      } else {
        regl.clear({ color: [0, 0, 0, 0] });
      }
      frame = requestAnimationFrame(loop);
    };
    loop();

    const resize = () => regl.poll();
    window.addEventListener("resize", resize);

    return () => {
      running = false;
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", resize);
      regl.destroy();
    };
  }, [phase]);

  return <canvas id="ambient" ref={canvasRef} data-testid="ambient" aria-hidden />;
}
