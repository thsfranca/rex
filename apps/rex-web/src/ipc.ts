import { invoke, Channel } from "@tauri-apps/api/core";
import type { StreamEvent } from "./types";
import { useAppStore } from "./store";

export async function ensureDaemon(): Promise<void> {
  await invoke("ensure_daemon");
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
