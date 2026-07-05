import { invoke, Channel } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  DaemonLifecycleEvent,
  FetchSessionEventsResult,
  StreamEvent,
  SystemStatus,
  ToolApprovalResult,
} from "./types";
import { useAppStore } from "./store";

export async function ensureDaemon(): Promise<SystemStatus> {
  return invoke<SystemStatus>("ensure_daemon");
}

export async function getSystemStatus(): Promise<SystemStatus> {
  return invoke<SystemStatus>("get_system_status");
}

export async function fetchSessionEvents(
  harnessSessionId: string,
  options: { beforeSequence?: number; afterSequence?: number; limit?: number } = {}
): Promise<FetchSessionEventsResult> {
  return invoke<FetchSessionEventsResult>("fetch_session_events", {
    harnessSessionId,
    beforeSequence: options.beforeSequence ?? 0,
    afterSequence: options.afterSequence ?? 0,
    limit: options.limit ?? 100,
  });
}

export async function respondToToolApproval(
  approvalToken: string,
  approved: boolean,
  toolCallId: string,
  harnessSessionId: string
): Promise<ToolApprovalResult> {
  return invoke<ToolApprovalResult>("respond_to_tool_approval", {
    approvalToken,
    approved,
    toolCallId,
    harnessSessionId,
  });
}

export function subscribeDaemonLifecycle(
  handler: (event: DaemonLifecycleEvent) => void
): Promise<() => void> {
  return listen<DaemonLifecycleEvent>("daemon-lifecycle", (payload) => {
    handler(payload.payload);
  });
}

export async function submitPrompt(prompt: string, mode = "agent"): Promise<void> {
  const store = useAppStore.getState();
  store.resetTurn();
  store.addUserMessage(prompt);
  store.setPhase("generating");

  const channel = new Channel<StreamEvent>();
  channel.onmessage = (event) => {
    const s = useAppStore.getState();
    switch (event.kind) {
      case "chunk":
        s.appendAssistantChunk(event.text);
        break;
      case "phase":
        s.setPhase(event.phase);
        break;
      case "message":
        s.addTimelineTask({ id: `t-${Date.now()}`, label: event.text });
        break;
      case "done":
        s.setPhase("terminal");
        s.setStatusLabel("Ready");
        break;
      case "error":
        s.setError(event.message);
        break;
    }
  };

  await invoke("submit_prompt", { prompt, mode, onEvent: channel });
}
