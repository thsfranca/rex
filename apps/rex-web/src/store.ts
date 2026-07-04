import { create } from "zustand";
import type { ChatMessage, TimelineTask, TurnPhase } from "./types";

interface AppState {
  phase: TurnPhase;
  statusLabel: string;
  messages: ChatMessage[];
  timeline: TimelineTask[];
  draft: string;
  error: string | null;
  setDraft: (draft: string) => void;
  setPhase: (phase: TurnPhase) => void;
  setStatusLabel: (label: string) => void;
  addUserMessage: (content: string) => void;
  appendAssistantChunk: (text: string) => void;
  setError: (message: string | null) => void;
  addTimelineTask: (task: TimelineTask) => void;
  resetTurn: () => void;
}

let messageCounter = 0;

export const useAppStore = create<AppState>((set, get) => ({
  phase: "idle",
  statusLabel: "Ready",
  messages: [],
  timeline: [],
  draft: "",
  error: null,
  setDraft: (draft) => set({ draft }),
  setPhase: (phase) => {
    const statusLabel =
      phase === "generating" || phase === "tool_running"
        ? "Working"
        : phase === "terminal"
          ? "Ready"
          : get().statusLabel;
    set({ phase, statusLabel });
  },
  setStatusLabel: (statusLabel) => set({ statusLabel }),
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
  resetTurn: () => set({ phase: "idle", statusLabel: "Ready", error: null }),
}));
