export interface SpringConfig {
  mass?: number;
  stiffness?: number;
  damping?: number;
  precision?: number;
}

export interface SpringState {
  position: number;
  velocity: number;
  atRest: boolean;
}

export interface Particle2D {
  x: number;
  y: number;
  vx: number;
  vy: number;
  life: number;
  maxLife: number;
  active: boolean;
}

export interface MotionOrchestratorState {
  phase: string;
  intensity: number;
  flowAngle: number;
  clock: number;
  isTyping: boolean;
  hasError: boolean;
  streamTick: number;
}

export type EffectTrigger =
  | "daemon_connect"
  | "stream_start"
  | "stream_token"
  | "timeline_add"
  | "approval_open"
  | "approval_close"
  | "error"
  | "composer_typing"
  | "session_focus";
