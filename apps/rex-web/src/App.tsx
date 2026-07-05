import { useEffect } from "react";
import { Composer } from "./components/Composer";
import { Timeline } from "./components/Timeline";
import { Transcript } from "./components/Transcript";
import { ensureDaemon, subscribeDaemonLifecycle } from "./ipc";
import { useAppStore } from "./store";

export default function App() {
  const phase = useAppStore((s) => s.phase);
  const statusLabel = useAppStore((s) => s.statusLabel);
  const workspaceRoot = useAppStore((s) => s.workspaceRoot);
  const messages = useAppStore((s) => s.messages);
  const timeline = useAppStore((s) => s.timeline);
  const error = useAppStore((s) => s.error);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void (async () => {
      try {
        const status = await ensureDaemon();
        useAppStore.getState().setWorkspaceRoot(status.workspaceRoot || null);
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        useAppStore.getState().setError(message);
      }

      unlisten = await subscribeDaemonLifecycle((event) => {
        const store = useAppStore.getState();
        switch (event.kind) {
          case "ready":
            store.setWorkspaceRoot(event.workspaceRoot || null);
            if (store.phase === "idle" || store.phase === "terminal") {
              store.setStatusLabel("Ready");
            }
            store.setError(null);
            break;
          case "unavailable":
            store.setError(event.message);
            break;
        }
      });
    })();

    return () => {
      unlisten?.();
    };
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
        {error ? error : workspaceRoot ? `Ready · ${workspaceRoot}` : "Ready"} · ⌘K ?
      </footer>
    </div>
  );
}
