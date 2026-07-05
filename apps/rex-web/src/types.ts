export type TurnPhase = "idle" | "generating" | "tool_running" | "tool_approval" | "terminal";

export type StreamEvent =
  | { kind: "chunk"; text: string }
  | { kind: "phase"; phase: TurnPhase }
  | { kind: "message"; text: string }
  | { kind: "done" }
  | { kind: "error"; code: string; message: string };

export type DaemonLifecycleEvent =
  | { kind: "ready"; workspaceRoot: string }
  | { kind: "unavailable"; message: string };

export interface SystemStatus {
  daemonVersion: string;
  uptimeSeconds: number;
  activeModelId: string;
  workspaceRoot: string;
  lifecycleState: string;
  idleSeconds: number;
}

export interface SessionEventRecord {
  sequence: number;
  event: string;
  text: string;
  turnId: string;
  toolName: string;
  phase: string;
}

export interface FetchSessionEventsResult {
  events: SessionEventRecord[];
  hasMoreBefore: boolean;
  hasMoreAfter: boolean;
  headSequence: number;
}

export interface ToolApprovalResult {
  ok: boolean;
  error: string;
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
}

export interface TimelineTask {
  id: string;
  label: string;
}
