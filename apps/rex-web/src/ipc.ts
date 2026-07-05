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
import { formatStreamEvent } from "./observability";

function readApprovalFields(event: StreamEvent & { kind: "approvalRequired" }) {
  const raw = event as StreamEvent & {
    kind: "approvalRequired";
    tool_call_id?: string;
    tool_name?: string;
    approval_token?: string;
  };
  return {
    toolCallId: event.toolCallId || raw.tool_call_id || "",
    toolName: event.toolName || raw.tool_name || "",
    detail: event.detail || "",
    approvalToken: event.approvalToken || raw.approval_token || "",
  };
}

export async function ensureDaemon(): Promise<SystemStatus> {
  return invoke<SystemStatus>("ensure_daemon");
}

export async function getLaunchOptions(): Promise<{ debug: boolean }> {
  return invoke<{ debug: boolean }>("launch_options");
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

export function subscribeMenuAction(
  handler: (action: string) => void
): Promise<() => void> {
  return listen<string>("menu-action", (payload) => {
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
    s.recordStreamEvent(formatStreamEvent(event));
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
      case "approvalRequired":
        s.setPendingApproval(readApprovalFields(event));
        s.setPhase("tool_approval");
        break;
      case "done":
        s.setPhase("terminal");
        s.setStatusLabel("Ready");
        break;
      case "error":
        if (event.message.startsWith("approval_required:")) {
          s.mergeApprovalToken(event.message.replace("approval_required:", ""));
          s.setPhase("tool_approval");
        } else {
          s.setError(event.message);
        }
        break;
    }
  };

  try {
    const sessionId = await invoke<string>("submit_prompt", {
      prompt,
      mode,
      onEvent: channel,
    });
    useAppStore.getState().setHarnessSessionId(sessionId);
    useAppStore.getState().recordSubmitError(null);
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    useAppStore.getState().recordSubmitError(message);
    throw err;
  }
}

export function sessionEventsToMessages(
  events: FetchSessionEventsResult["events"]
): { role: "user" | "assistant"; content: string }[] {
  const messages: { role: "user" | "assistant"; content: string }[] = [];
  for (const event of events) {
    if (event.event === "chunk" && event.text) {
      const last = messages[messages.length - 1];
      if (last?.role === "assistant") {
        last.content += event.text;
      } else {
        messages.push({ role: "assistant", content: event.text });
      }
    }
    if (event.event === "user" && event.text) {
      messages.push({ role: "user", content: event.text });
    }
  }
  return messages;
}
