import { useEffect, useRef, useSyncExternalStore, type RefObject } from "react";
import type { MotionOrchestratorState } from "../physics/types";
import type { TurnPhase } from "../../types";

const IDLE: MotionOrchestratorState = {
  phase: "idle",
  intensity: 0,
  flowAngle: 0,
  clock: 0,
  isTyping: false,
  hasError: false,
  streamTick: 0,
  connectFade: 0,
  decorativePaused: false,
  approvalOpen: false,
};

type Listener = () => void;

class MotionOrchestrator {
  private state: MotionOrchestratorState = { ...IDLE };
  private listeners = new Set<Listener>();
  private raf = 0;
  private last = 0;
  private running = false;

  subscribe = (listener: Listener): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  getSnapshot = (): MotionOrchestratorState => this.state;

  private emit(): void {
    for (const listener of this.listeners) listener();
  }

  private intensityForPhase(phase: TurnPhase): number {
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

  private flowAngleForPhase(phase: TurnPhase, clock: number): number {
    if (phase === "idle") return 0;
    return Math.sin(clock * 0.8) * 0.4 + (phase === "generating" ? -Math.PI / 2 : -Math.PI / 4);
  }

  setPhase(phase: TurnPhase): void {
    const prev = this.state.phase;
    this.state = {
      ...this.state,
      phase,
      intensity: this.intensityForPhase(phase),
      flowAngle: this.flowAngleForPhase(phase, this.state.clock),
      approvalOpen: phase === "tool_approval",
    };
    if (phase !== "idle" && prev === "idle") {
      this.state.streamTick += 1;
    }
    this.emit();
    this.ensureLoop();
  }

  setTyping(isTyping: boolean): void {
    if (this.state.isTyping === isTyping) return;
    this.state = { ...this.state, isTyping };
    this.emit();
    this.ensureLoop();
  }

  pulseStream(): void {
    this.state = { ...this.state, streamTick: this.state.streamTick + 1 };
    this.emit();
  }

  setError(hasError: boolean): void {
    this.state = { ...this.state, hasError };
    this.emit();
    this.ensureLoop();
  }

  signalDaemonReady(): void {
    this.state = { ...this.state, connectFade: 1 };
    this.emit();
    this.ensureLoop();
  }

  setDecorativePaused(paused: boolean): void {
    if (this.state.decorativePaused === paused) return;
    this.state = { ...this.state, decorativePaused: paused };
    this.emit();
  }

  private tick = (now: number): void => {
    if (!this.running) return;
    const dt = this.last ? (now - this.last) / 1000 : 0;
    this.last = now;
    const active =
      this.state.phase !== "idle" ||
      this.state.isTyping ||
      this.state.hasError ||
      this.state.connectFade > 0;
    if (active) {
      const connectFade =
        this.state.connectFade > 0 ? Math.max(0, this.state.connectFade - dt * 2.5) : 0;
      this.state = {
        ...this.state,
        clock: this.state.clock + dt,
        connectFade,
        flowAngle: this.flowAngleForPhase(
          this.state.phase as TurnPhase,
          this.state.clock + dt,
        ),
      };
      this.emit();
    }
    this.raf = requestAnimationFrame(this.tick);
  };

  private ensureLoop(): void {
    if (this.running) return;
    if (typeof window === "undefined") return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    this.running = true;
    this.last = 0;
    this.raf = requestAnimationFrame(this.tick);
  }

  stop(): void {
    this.running = false;
    cancelAnimationFrame(this.raf);
  }
}

export const motionOrchestrator = new MotionOrchestrator();

export function useMotionOrchestrator(): MotionOrchestratorState {
  return useSyncExternalStore(
    motionOrchestrator.subscribe,
    motionOrchestrator.getSnapshot,
    () => IDLE,
  );
}

export function useDecorativeMotionEnabled(): boolean {
  const { decorativePaused } = useMotionOrchestrator();
  return !decorativePaused;
}

export function useOrchestratorPhaseBinding(phase: TurnPhase): void {
  useEffect(() => {
    motionOrchestrator.setPhase(phase);
  }, [phase]);
}

export function useOrchestratorErrorBinding(error: string | null): void {
  useEffect(() => {
    motionOrchestrator.setError(Boolean(error));
  }, [error]);
}

export function useOrchestratorStreamBinding(messageCount: number): void {
  const prev = useRef(messageCount);
  useEffect(() => {
    if (messageCount > prev.current) {
      motionOrchestrator.pulseStream();
    }
    prev.current = messageCount;
  }, [messageCount]);
}

export function useReflowGuard(ref: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    const node = ref.current;
    if (!node) return;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const observer = new ResizeObserver(() => {
      motionOrchestrator.setDecorativePaused(true);
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => motionOrchestrator.setDecorativePaused(false), 120);
    });
    observer.observe(node);
    return () => {
      observer.disconnect();
      if (timer) clearTimeout(timer);
      motionOrchestrator.setDecorativePaused(false);
    };
  }, [ref]);
}
