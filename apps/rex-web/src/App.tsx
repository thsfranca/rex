import { useCallback, useEffect, useMemo, useState } from "react";
import { ShellGrid, Text } from "./design-system";
import { AppHeader } from "./components/AppHeader";
import { ApprovalModal } from "./components/ApprovalModal";
import { AmbientCanvas } from "./components/AmbientCanvas";
import { ParticleField } from "./components/ParticleField";
import {
  ShellEntrance,
  useOrchestratorErrorBinding,
  useOrchestratorPhaseBinding,
  useOrchestratorStreamBinding,
  motionOrchestrator,
} from "./design-system";
import { CommandPalette, ErrorBanner, type CommandAction } from "./components/CommandPalette";
import { Composer } from "./components/Composer";
import { StatusPanel } from "./components/StatusPanel";
import { SessionPicker } from "./components/SessionPicker";
import { Timeline } from "./components/Timeline";
import { Transcript } from "./components/Transcript";
import { StatusOrbit } from "./components/StatusOrbit";
import { UiObservability } from "./components/UiObservability";
import {
  ensureDaemon,
  fetchSessionEvents,
  getLaunchOptions,
  getSystemStatus,
  listClosedSessions,
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
import type { SystemStatus } from "./types";

async function hydrateSession(sessionId: string) {
  const result = await fetchSessionEvents(sessionId);
  const hydrated = sessionEventsToMessages(result.events).map((message, index) => ({
    id: `h-${index}`,
    ...message,
  }));
  const store = useAppStore.getState();
  store.hydrateMessages(hydrated);
  store.setHarnessSessionId(sessionId);
}

export default function App() {
  const [debugEnabled, setDebugEnabled] = useState(false);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [statusPanelOpen, setStatusPanelOpen] = useState(false);
  const [systemStatus, setSystemStatus] = useState<SystemStatus | null>(null);

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
  const composerMode = useAppStore((s) => s.composerMode);

  useOrchestratorPhaseBinding(phase);
  useOrchestratorErrorBinding(error);
  useOrchestratorStreamBinding(messages.length);

  const refreshSessions = useCallback(async () => {
    try {
      const items = await listClosedSessions();
      useAppStore.getState().setSessions(items);
    } catch {
      useAppStore.getState().setSessions([]);
    }
  }, []);

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
    motionOrchestrator.signalDaemonReady();

    let unlistenLifecycle: (() => void) | undefined;
    let unlistenMenu: (() => void) | undefined;

    void (async () => {
      try {
        const status = await ensureDaemon();
        useAppStore.getState().setWorkspaceRoot(status.workspaceRoot || null);
        setSystemStatus(status);
        motionOrchestrator.signalDaemonReady();
        await refreshSessions();
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        useAppStore.getState().setError(message);
        motionOrchestrator.signalDaemonReady();
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
          await refreshSessions();
          store.setSessionPickerOpen(true);
          return;
        }
        if (action === "session_last" && store.harnessSessionId) {
          await hydrateSession(store.harnessSessionId);
        }
      });
    })();

    return () => {
      unlistenLifecycle?.();
      unlistenMenu?.();
    };
  }, [refreshSessions]);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setCommandPaletteOpen((open) => !open);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
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

  const commandActions: CommandAction[] = useMemo(
    () => [
      {
        id: "new-session",
        label: "New session",
        run: () => useAppStore.getState().newSession(),
      },
      {
        id: "continue",
        label: "Continue session",
        run: () => {
          void refreshSessions();
          useAppStore.getState().setSessionPickerOpen(true);
        },
      },
      {
        id: "last-session",
        label: "Open last session",
        run: () => {
          const id = useAppStore.getState().harnessSessionId;
          if (id) void hydrateSession(id);
        },
      },
      {
        id: "status",
        label: "Show system status",
        run: () => {
          void getSystemStatus().then(setSystemStatus);
          setStatusPanelOpen(true);
        },
      },
      {
        id: "toggle-debug",
        label: debugEnabled ? "Hide debug overlay" : "Show debug overlay",
        run: () => setDebugEnabled((enabled) => !enabled),
      },
      {
        id: "reload",
        label: "Reload window",
        shortcut: "View menu",
        run: () => window.location.reload(),
      },
    ],
    [debugEnabled, refreshSessions]
  );

  return (
    <>
      <AmbientCanvas phase={phase} />
      <ParticleField phase={phase} />
      <StatusOrbit working={working} />
      <ShellEntrance>
      <ShellGrid
        header={
          <AppHeader
            workspaceRoot={workspaceRoot}
            mode={composerMode}
            statusLabel={statusLabel}
            working={working}
            hasError={Boolean(error)}
            onOpenCommands={() => setCommandPaletteOpen(true)}
          />
        }
        transcript={<Transcript messages={messages} phase={phase} working={working} />}
        timeline={<Timeline tasks={timeline} phase={phase} />}
        composer={<Composer disabled={working || phase === "tool_approval"} typing={composerBusy} />}
        footer={
          <Text tone="secondary" data-testid="footer">
            {workspaceRoot ? `Ready · ${workspaceRoot}` : "Ready"} · ⌘K commands
          </Text>
        }
      />
      </ShellEntrance>
      {error ? (
        <ErrorBanner message={error} onDismiss={() => useAppStore.getState().setError(null)} />
      ) : null}
      {pendingApproval && phase === "tool_approval" ? (
        <ApprovalModal
          open
          pending={pendingApproval}
          onApprove={() => void handleApproval(true)}
          onDeny={() => void handleApproval(false)}
        />
      ) : null}
      {sessionPickerOpen ? (
        <SessionPicker
          sessions={sessions}
          onSelect={(sessionId) => {
            void hydrateSession(sessionId).then(() => {
              useAppStore.getState().setSessionPickerOpen(false);
            });
          }}
          onClose={() => useAppStore.getState().setSessionPickerOpen(false)}
        />
      ) : null}
      <CommandPalette
        open={commandPaletteOpen}
        actions={commandActions}
        onClose={() => setCommandPaletteOpen(false)}
      />
      {statusPanelOpen ? (
        <StatusPanel status={systemStatus} onClose={() => setStatusPanelOpen(false)} />
      ) : null}
      <UiObservability snapshot={observabilitySnapshot} />
    </>
  );
}
