import { useEffect, useMemo, useState } from "react";
import { ApprovalModal } from "./components/ApprovalModal";
import { AmbientCanvas } from "./components/AmbientCanvas";
import { Composer } from "./components/Composer";
import { MotionStatusDot } from "./components/Motion";
import { SessionPicker } from "./components/SessionPicker";
import { Timeline } from "./components/Timeline";
import { Transcript } from "./components/Transcript";
import { UiObservability } from "./components/UiObservability";
import {
  ensureDaemon,
  fetchSessionEvents,
  getLaunchOptions,
  respondToToolApproval,
  sessionEventsToMessages,
  subscribeDaemonLifecycle,
  subscribeMenuAction,
} from "./ipc";
import {
  buildObservabilitySnapshot,
  publishObservabilitySnapshot,
} from "./observability";
import { useAppStore } from "./store";

export default function App() {
  const [debugEnabled, setDebugEnabled] = useState(false);
  const phase = useAppStore((s) => s.phase);
  const statusLabel = useAppStore((s) => s.statusLabel);
  const workspaceRoot = useAppStore((s) => s.workspaceRoot);
  const messages = useAppStore((s) => s.messages);
  const timeline = useAppStore((s) => s.timeline);
  const error = useAppStore((s) => s.error);
  const pendingApproval = useAppStore((s) => s.pendingApproval);
  const sessionPickerOpen = useAppStore((s) => s.sessionPickerOpen);
  const sessions = useAppStore((s) => s.sessions);
  const harnessSessionId = useAppStore((s) => s.harnessSessionId);
  const streamEvents = useAppStore((s) => s.streamEvents);
  const lastSubmitError = useAppStore((s) => s.lastSubmitError);
  const composerBusy = useAppStore((s) => s.composerBusy);

  const observabilitySnapshot = useMemo(
    () =>
      buildObservabilitySnapshot({
        enabled: debugEnabled,
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
      debugEnabled,
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
    void getLaunchOptions()
      .then((options) => setDebugEnabled(options.debug))
      .catch(() => setDebugEnabled(false));
  }, []);

  useEffect(() => {
    publishObservabilitySnapshot(observabilitySnapshot);
  }, [observabilitySnapshot]);

  useEffect(() => {
    let unlistenLifecycle: (() => void) | undefined;
    let unlistenMenu: (() => void) | undefined;

    void (async () => {
      try {
        const status = await ensureDaemon();
        useAppStore.getState().setWorkspaceRoot(status.workspaceRoot || null);
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        useAppStore.getState().setError(message);
      }

      unlistenLifecycle = await subscribeDaemonLifecycle((event) => {
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

      unlistenMenu = await subscribeMenuAction(async (action) => {
        const store = useAppStore.getState();
        if (action === "session_new") {
          store.newSession();
          return;
        }
        if (action === "session_continue") {
          store.setSessionPickerOpen(true);
          return;
        }
        if (action === "session_last" && store.harnessSessionId) {
          const result = await fetchSessionEvents(store.harnessSessionId);
          const hydrated = sessionEventsToMessages(result.events).map((message, index) => ({
            id: `h-${index}`,
            ...message,
          }));
          store.hydrateMessages(hydrated);
        }
      });
    })();

    return () => {
      unlistenLifecycle?.();
      unlistenMenu?.();
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
      <AmbientCanvas phase={phase} />
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
      {sessionPickerOpen ? (
        <SessionPicker
          sessions={sessions}
          onSelect={(sessionId) => {
            void fetchSessionEvents(sessionId).then((result) => {
              const hydrated = sessionEventsToMessages(result.events).map((message, index) => ({
                id: `h-${index}`,
                ...message,
              }));
              useAppStore.getState().hydrateMessages(hydrated);
              useAppStore.getState().setHarnessSessionId(sessionId);
              useAppStore.getState().setSessionPickerOpen(false);
            });
          }}
          onClose={() => useAppStore.getState().setSessionPickerOpen(false)}
        />
      ) : null}
      <UiObservability snapshot={observabilitySnapshot} />
    </>
  );
}
