import { useEffect } from "react";
import { Composer } from "./components/Composer";
import { Timeline } from "./components/Timeline";
import { Transcript } from "./components/Transcript";
import { ensureDaemon } from "./ipc";
import { useAppStore } from "./store";

export default function App() {
  const phase = useAppStore((s) => s.phase);
  const statusLabel = useAppStore((s) => s.statusLabel);
  const messages = useAppStore((s) => s.messages);
  const timeline = useAppStore((s) => s.timeline);
  const error = useAppStore((s) => s.error);

  useEffect(() => {
    void ensureDaemon().catch((err: unknown) => {
      const message = err instanceof Error ? err.message : String(err);
      useAppStore.getState().setError(message);
    });
  }, []);

  const working = phase === "generating" || phase === "tool_running";

  return (
    <div className="shell" data-testid="shell">
      <header className="header" data-testid="header">
        <span
          className={`status-dot${working ? " working" : ""}`}
          id="status-dot"
          data-testid="status-dot"
        />
        <span id="status-label" style={{ marginLeft: 8 }}>
          {statusLabel}
        </span>
      </header>
      <Transcript messages={messages} />
      <Timeline tasks={timeline} phase={phase} />
      <Composer disabled={working} />
      <footer className="footer" data-testid="footer">
        {error ? error : "Ready"} · ⌘K ?
      </footer>
    </div>
  );
}
