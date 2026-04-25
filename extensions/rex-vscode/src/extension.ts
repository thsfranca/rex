import * as vscode from "vscode";

import { readSettings, onSettingsChanged, type RexSettings } from "./config/settings";
import { snapshotActiveEditor } from "./editor/context";
import { activateCursorAdapter } from "./platform/cursorAdapter";
import { DaemonLifecycle, type DaemonLifecycleState } from "./runtime/daemonLifecycle";
import { ChatPanelProvider, CHAT_VIEW_ID } from "./ui/chatPanel";
import { createStatusBar, type StatusBar } from "./ui/statusBar";

const PROBE_INTERVAL_MS = 10_000;

interface ActivationResources {
  readonly output: vscode.OutputChannel;
  readonly statusBar: StatusBar;
  readonly chatPanel: ChatPanelProvider;
  lifecycle: DaemonLifecycle;
  probeTimer: NodeJS.Timeout | undefined;
  settings: RexSettings;
}

let resources: ActivationResources | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const output = vscode.window.createOutputChannel("REX");
  context.subscriptions.push(output);
  const statusBar = createStatusBar();
  context.subscriptions.push({ dispose: () => statusBar.dispose() });

  const settings = readSettings();
  output.appendLine(`[activate] settings: ${summarizeSettings(settings)}`);

  const chatPanel = new ChatPanelProvider({
    context,
    getCliOptions: () => ({ cliPath: resources?.settings.cliPath ?? settings.cliPath }),
    getDaemonAutoStart: () => resources?.settings.daemonAutoStart ?? settings.daemonAutoStart,
    ensureDaemonReady: (signal) => {
      const r = resources;
      if (r === undefined) {
        return Promise.resolve({
          kind: "unavailable",
          reason: "REX extension is not active",
        } as DaemonLifecycleState);
      }
      return r.lifecycle.ensureRunning(signal);
    },
    getDaemonState: () => lastLifecycleState,
    log: (message) => output.appendLine(message),
  });
  context.subscriptions.push(chatPanel.register());

  let lastLifecycleState: DaemonLifecycleState | undefined;

  const lifecycle = buildLifecycle(settings, statusBar, output, (state) => {
    lastLifecycleState = state;
    chatPanel.broadcastDaemonState(state);
  });
  resources = {
    output,
    statusBar,
    chatPanel,
    lifecycle,
    probeTimer: undefined,
    settings,
  };

  context.subscriptions.push(
    vscode.commands.registerCommand("rex.showStatus", async () => {
      const state = await refreshDaemonConnection(resources);
      output.appendLine(`[command] showStatus -> ${describeState(state)}`);
      if (state?.kind === "ready") {
        void vscode.window.showInformationMessage(
          `REX ready (daemon ${state.status.daemonVersion}, uptime ${state.status.uptimeSeconds}s, model ${state.status.activeModelId || "unknown"}).`,
        );
      } else if (state?.kind === "starting") {
        void vscode.window.showInformationMessage("REX daemon is starting...");
      } else if (state?.kind === "unavailable") {
        void vscode.window.showWarningMessage(`REX daemon unavailable: ${state.reason}`);
      }
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("rex.howToStartDaemon", async () => {
      const pick = await vscode.window.showInformationMessage(
        "Start `rex-daemon` in a terminal (e.g. `cargo run -p rex-daemon`), then rerun `REX: Show Daemon Status`.",
        "Open Output",
      );
      if (pick === "Open Output") {
        output.show(true);
      }
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("rex.openOutput", () => {
      output.show(true);
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("rex.focusChat", async () => {
      await vscode.commands.executeCommand(`${CHAT_VIEW_ID}.focus`);
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("rex.clearChat", () => {
      resources?.chatPanel.clearChat();
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("rex.explainSelection", async () => {
      await prefillFromSelection("Explain the selected code. Focus on control flow and intent.");
    }),
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("rex.fixSelection", async () => {
      await prefillFromSelection("Fix the selected code and explain the root cause briefly.");
    }),
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("rex.refactorSelection", async () => {
      await prefillFromSelection(
        "Refactor the selected code for readability; preserve behavior and mention trade-offs.",
      );
    }),
  );

  context.subscriptions.push(
    onSettingsChanged((updated) => {
      output.appendLine(`[settings] changed -> ${summarizeSettings(updated)}`);
      if (resources === undefined) {
        return;
      }
      const previousLifecycle = resources.lifecycle;
      resources.settings = updated;
      resources.lifecycle = buildLifecycle(updated, statusBar, output, (state) => {
        lastLifecycleState = state;
        chatPanel.broadcastDaemonState(state);
      });
      void previousLifecycle.shutdown().catch(() => undefined);
      if (updated.daemonAutoStart) {
        void resources.lifecycle.ensureRunning();
      } else {
        void resources.lifecycle.probe();
      }
    }),
  );

  context.subscriptions.push({
    dispose: () => {
      if (resources?.probeTimer !== undefined) {
        clearInterval(resources.probeTimer);
      }
    },
  });

  await activateCursorAdapter(context, (message) => output.appendLine(`[cursor] ${message}`));

  if (settings.daemonAutoStart) {
    output.appendLine("[activate] daemonAutoStart=true -> ensuring daemon is running");
    void lifecycle.ensureRunning().then((state) => {
      output.appendLine(`[activate] auto-start result -> ${describeState(state)}`);
    });
  } else {
    void lifecycle.probe();
  }
  resources.probeTimer = setInterval(() => {
    void refreshDaemonConnection(resources);
  }, PROBE_INTERVAL_MS);

  async function prefillFromSelection(template: string): Promise<void> {
    const snapshot = snapshotActiveEditor();
    const prompt =
      snapshot?.selectionText !== undefined
        ? `${template}\n\nSelection:\n\`\`\`${snapshot.languageId}\n${snapshot.selectionText}\n\`\`\``
        : template;
    chatPanel.prefillPrompt({ prompt, context: snapshot });
    await vscode.commands.executeCommand(`${CHAT_VIEW_ID}.focus`);
  }
}

export async function deactivate(): Promise<void> {
  if (resources === undefined) {
    return;
  }
  if (resources.probeTimer !== undefined) {
    clearInterval(resources.probeTimer);
  }
  resources.chatPanel.dispose();
  await resources.lifecycle.shutdown();
  resources = undefined;
}

function buildLifecycle(
  settings: RexSettings,
  statusBar: StatusBar,
  output: vscode.OutputChannel,
  onStateChange: (state: DaemonLifecycleState) => void,
): DaemonLifecycle {
  return new DaemonLifecycle({
    cli: { cliPath: settings.cliPath },
    daemonBinaryPath: settings.daemonBinaryPath,
    onState: (state) => {
      statusBar.update(state);
      output.appendLine(`[lifecycle] ${describeState(state)}`);
      onStateChange(state);
    },
  });
}

function describeState(state: DaemonLifecycleState | undefined): string {
  if (state === undefined) {
    return "unknown";
  }
  switch (state.kind) {
    case "ready":
      return `ready (version=${state.status.daemonVersion}, uptime=${state.status.uptimeSeconds}s)`;
    case "starting":
      return "starting";
    case "unavailable":
      return `unavailable (${state.reason})`;
  }
}

function summarizeSettings(settings: RexSettings): string {
  return `cli=${settings.cliPath} daemon=${settings.daemonBinaryPath} autoStart=${settings.daemonAutoStart}`;
}

function refreshDaemonConnection(
  r: ActivationResources | undefined,
): Promise<DaemonLifecycleState | undefined> {
  if (r === undefined) {
    return Promise.resolve(undefined);
  }
  if (r.settings.daemonAutoStart) {
    return r.lifecycle.ensureRunning();
  }
  return r.lifecycle.probe();
}
