import { create } from "zustand";
import type { ChatMessage, PendingApproval, SessionSummary, TimelineTask, TurnPhase } from "./types";

interface AppState {
  phase: TurnPhase;
  statusLabel: string;
  workspaceRoot: string | null;
  harnessSessionId: string | null;
  messages: ChatMessage[];
  timeline: TimelineTask[];
  draft: string;
  error: string | null;
  pendingApproval: PendingApproval | null;
  sessionPickerOpen: boolean;
  sessions: SessionSummary[];
  streamEvents: string[];
  lastSubmitError: string | null;
  composerBusy: boolean;
  setDraft: (draft: string) => void;
  setPhase: (phase: TurnPhase) => void;
  setStatusLabel: (label: string) => void;
  setWorkspaceRoot: (root: string | null) => void;
  setHarnessSessionId: (id: string | null) => void;
  addUserMessage: (content: string) => void;
  appendAssistantChunk: (text: string) => void;
  setError: (message: string | null) => void;
  addTimelineTask: (task: TimelineTask) => void;
  setPendingApproval: (approval: PendingApproval | null) => void;
  mergeApprovalToken: (token: string) => void;
  setSessionPickerOpen: (open: boolean) => void;
  setSessions: (sessions: SessionSummary[]) => void;
  hydrateMessages: (messages: ChatMessage[]) => void;
  recordStreamEvent: (label: string) => void;
  recordSubmitError: (message: string | null) => void;
  setComposerBusy: (busy: boolean) => void;
  resetTurn: () => void;
  newSession: () => void;
}

let messageCounter = 0;

export const useAppStore = create<AppState>((set, get) => ({
  phase: "idle",
  statusLabel: "Ready",
  workspaceRoot: null,
  harnessSessionId: null,
  messages: [],
  timeline: [],
  draft: "",
  error: null,
  pendingApproval: null,
  sessionPickerOpen: false,
  sessions: [],
  streamEvents: [],
  lastSubmitError: null,
  composerBusy: false,
  setDraft: (draft) => set({ draft }),
  setPhase: (phase) => {
    const statusLabel =
      phase === "generating" || phase === "tool_running"
        ? "Working"
        : phase === "tool_approval"
          ? "Approval required"
          : phase === "terminal"
            ? "Ready"
            : get().statusLabel;
    set({ phase, statusLabel });
  },
  setStatusLabel: (statusLabel) => set({ statusLabel }),
  setWorkspaceRoot: (workspaceRoot) => set({ workspaceRoot }),
  setHarnessSessionId: (harnessSessionId) => set({ harnessSessionId }),
  addUserMessage: (content) => {
    messageCounter += 1;
    set((s) => ({
      messages: [
        ...s.messages,
        { id: `u-${messageCounter}`, role: "user", content },
      ],
    }));
  },
  appendAssistantChunk: (text) => {
    set((s) => {
      const last = s.messages[s.messages.length - 1];
      if (last?.role === "assistant") {
        const updated = [...s.messages];
        updated[updated.length - 1] = {
          ...last,
          content: last.content + text,
        };
        return { messages: updated };
      }
      messageCounter += 1;
      return {
        messages: [
          ...s.messages,
          { id: `a-${messageCounter}`, role: "assistant", content: text },
        ],
      };
    });
  },
  setError: (message) =>
    set({
      error: message,
      statusLabel: message ? "Error" : "Ready",
      phase: message ? "terminal" : "idle",
    }),
  addTimelineTask: (task) =>
    set((s) => ({
      timeline: [...s.timeline.filter((t) => t.id !== task.id), task],
    })),
  setPendingApproval: (pendingApproval) => set({ pendingApproval }),
  mergeApprovalToken: (approvalToken) =>
    set((s) => {
      if (!s.pendingApproval) {
        return {
          pendingApproval: {
            toolCallId: "",
            toolName: "",
            detail: "",
            approvalToken,
          },
        };
      }
      return {
        pendingApproval: { ...s.pendingApproval, approvalToken },
      };
    }),
  setSessionPickerOpen: (sessionPickerOpen) => set({ sessionPickerOpen }),
  setSessions: (sessions) => set({ sessions }),
  hydrateMessages: (messages) => set({ messages }),
  recordStreamEvent: (label) =>
    set((s) => ({
      streamEvents: [...s.streamEvents.slice(-31), label],
    })),
  recordSubmitError: (lastSubmitError) => set({ lastSubmitError }),
  setComposerBusy: (composerBusy) => set({ composerBusy }),
  resetTurn: () =>
    set({ phase: "idle", statusLabel: "Ready", error: null, lastSubmitError: null }),
  newSession: () =>
    set({
      messages: [],
      timeline: [],
      pendingApproval: null,
      phase: "idle",
      statusLabel: "Ready",
      error: null,
      harnessSessionId: null,
      streamEvents: [],
      lastSubmitError: null,
      composerBusy: false,
    }),
}));
