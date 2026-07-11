import type {
  DaemonLifecycleEvent,
  FetchSessionEventsResult,
  SessionSummary,
  StreamEvent,
  SystemStatus,
  ToolApprovalResult,
} from "./types";
import { useAppStore } from "./store";
import { formatStreamEvent } from "./observability";

type RexDesktopApi = {
  host: string;
  shell: string;
  ensureDaemon: () => Promise<SystemStatus>;
  getLaunchOptions: () => Promise<{ debug: boolean }>;
  getSystemStatus: () => Promise<SystemStatus>;
  listClosedSessions: () => Promise<SessionSummary[]>;
  fetchSessionEvents: (
    harnessSessionId: string,
    opts?: { beforeSequence?: number; afterSequence?: number; limit?: number }
  ) => Promise<FetchSessionEventsResult>;
  respondToToolApproval: (
    approvalToken: string,
    approved: boolean,
    toolCallId: string,
    harnessSessionId: string
  ) => Promise<ToolApprovalResult>;
  submitPrompt: (
    prompt: string,
    mode: string,
    onEvent: (event: StreamEvent) => void
  ) => Promise<string>;
  subscribeDaemonLifecycle: (
    handler: (event: DaemonLifecycleEvent) => void
  ) => () => void;
  subscribeMenuAction: (handler: (action: string) => void) => () => void;
};

function desktopApi(): RexDesktopApi {
  const api = (window as Window & { rexDesktop?: RexDesktopApi }).rexDesktop;
  if (!api) {
    throw new Error("rexDesktop bridge unavailable (Electron preload missing)");
  }
  return api;
}

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
  return desktopApi().ensureDaemon();
}

export async function getLaunchOptions(): Promise<{ debug: boolean }> {
  return desktopApi().getLaunchOptions();
}

export async function getSystemStatus(): Promise<SystemStatus> {
  return desktopApi().getSystemStatus();
}

export async function listClosedSessions(): Promise<SessionSummary[]> {
  return desktopApi().listClosedSessions();
}

export async function fetchSessionEvents(
  harnessSessionId: string,
  options: { beforeSequence?: number; afterSequence?: number; limit?: number } = {}
): Promise<FetchSessionEventsResult> {
  return desktopApi().fetchSessionEvents(harnessSessionId, options);
}

export async function respondToToolApproval(
  approvalToken: string,
  approved: boolean,
  toolCallId: string,
  harnessSessionId: string
): Promise<ToolApprovalResult> {
  return desktopApi().respondToToolApproval(
    approvalToken,
    approved,
    toolCallId,
    harnessSessionId
  );
}

export function subscribeDaemonLifecycle(
  handler: (event: DaemonLifecycleEvent) => void
): Promise<() => void> {
  return Promise.resolve(desktopApi().subscribeDaemonLifecycle(handler));
}

export function subscribeMenuAction(
  handler: (action: string) => void
): Promise<() => void> {
  return Promise.resolve(desktopApi().subscribeMenuAction(handler));
}

export async function submitPrompt(prompt: string, mode = "agent"): Promise<void> {
  const store = useAppStore.getState();
  store.resetTurn();
  store.addUserMessage(prompt);
  store.setPhase("generating");

  try {
    const sessionId = await desktopApi().submitPrompt(prompt, mode, (event) => {
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
