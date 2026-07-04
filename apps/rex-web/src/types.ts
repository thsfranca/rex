export type TurnPhase = "idle" | "generating" | "tool_running" | "tool_approval" | "terminal";

export type StreamEvent =
  | { kind: "chunk"; text: string }
  | { kind: "phase"; phase: TurnPhase }
  | { kind: "message"; text: string }
  | { kind: "done" }
  | { kind: "error"; code: string; message: string };

export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
}

export interface TimelineTask {
  id: string;
  label: string;
}
