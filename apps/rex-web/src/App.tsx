import { useEffect, useMemo } from "react";
import { ApprovalModal } from "./components/ApprovalModal";
import { Composer } from "./components/Composer";
import { MotionStatusDot } from "./components/Motion";
import { Timeline } from "./components/Timeline";
import { Transcript } from "./components/Transcript";
import { UiObservability } from "./components/UiObservability";
import { ensureDaemon, respondToToolApproval, subscribeDaemonLifecycle } from "./ipc";
import {
  buildObservabilitySnapshot,
  publishObservabilitySnapshot,
} from "./observability";
import { useAppStore } from "./store";

export default function App() {
  const phase = useAppStore((s) => s.phase);
  const statusLabel = useAppStore((s) => s.statusLabel);
  const workspaceRoot = useAppStore((s) => s.workspaceRoot);
  const messages = useAppStore((s) => s.messages);
  const timeline = useAppStore((s) => s.timeline);
  const error = useAppStore((s) => s.error);
  const pendingApproval = useAppStore((s) => s.pendingApproval);
  const harnessSessionId = useAppStore((s) => s.harnessSessionId);
  const streamEvents = useAppStore((s) => s.streamEvents);
  const lastSubmitError = useAppStore((s) => s.lastSubmitError);
  const composerBusy = useAppStore((s) => s.composerBusy);

  const observabilitySnapshot = useMemo(
    () =>
      buildObservabilitySnapshot({
        phase,
        statusLabel,
        pendingApproval,
        error,
        harnessSessionId,
        lastSubmitError,
        composerBusy,
        streamEvents,
      }),
    [
      phase,
      statusLabel,
      pendingApproval,
      error,
      harnessSessionId,
      lastSubmitError,
      composerBusy,
      streamEvents,
    ]
  );

  useEffect(() => {
    publishObservabilitySnapshot(observabilitySnapshot);
  }, [observabilitySnapshot]);

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

  const handleApproval = async (approved: boolean) => {
    if (!pendingApproval?.approvalToken || !harnessSessionId) return;
    await respondToToolApproval(
      pendingApproval.approvalToken,
      approved,
      pendingApproval.toolCallId,
      harnessSessionId
    );
    useAppStore.getState().setPendingApproval(null);
    useAppStore.getState().setPhase("generating");
  };

  return (
    <>
      <div className="shell" data-testid="shell">
        <header className="header" data-testid="header">
          <MotionStatusDot working={working} id="status-dot" testId="status-dot" />
          <span id="status-label" style={{ marginLeft: 8 }}>
            {statusLabel}
          </span>
        </header>
        <Transcript messages={messages} />
        <Timeline tasks={timeline} phase={phase} />
        <Composer disabled={working || phase === "tool_approval"} />
        <footer className="footer" data-testid="footer">
          {error ? error : workspaceRoot ? `Ready · ${workspaceRoot}` : "Ready"} · ⌘K ?
        </footer>
      </div>
      {pendingApproval && phase === "tool_approval" ? (
        <ApprovalModal
          pending={pendingApproval}
          onApprove={() => void handleApproval(true)}
          onDeny={() => void handleApproval(false)}
        />
      ) : null}
      <UiObservability snapshot={observabilitySnapshot} />
    </>
  );
}
