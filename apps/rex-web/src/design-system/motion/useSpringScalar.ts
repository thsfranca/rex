import { useEffect, useRef, useState } from "react";
import { createSpringState, stepSpring } from "../physics/rk4-spring";
import type { SpringConfig } from "../physics/types";

export function useSpringScalar(
  target: number,
  config: SpringConfig = {},
  enabled = true,
): number {
  const stateRef = useRef(createSpringState(target));
  const [value, setValue] = useState(target);

  useEffect(() => {
    if (!enabled) {
      stateRef.current = createSpringState(target);
      setValue(target);
      return;
    }

    let frame = 0;
    let last = performance.now();
    let running = true;

    const tick = (now: number) => {
      if (!running) return;
      const dt = Math.min(0.032, (now - last) / 1000);
      last = now;
      const next = stepSpring(stateRef.current, target, dt, config);
      stateRef.current = next;
      setValue(next.position);
      if (!next.atRest) {
        frame = requestAnimationFrame(tick);
      }
    };

    frame = requestAnimationFrame(tick);
    return () => {
      running = false;
      cancelAnimationFrame(frame);
    };
  }, [target, enabled, config.damping, config.stiffness, config.mass, config.precision]);

  return enabled ? value : target;
}
