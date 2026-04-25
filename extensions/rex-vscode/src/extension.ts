import * as vscode from "vscode";

import { readSettings, onSettingsChanged, type RexSettings } from "./config/settings";
import { activateCursorAdapter } from "./platform/cursorAdapter";
import { DaemonLifecycle, type DaemonLifecycleState } from "./runtime/daemonLifecycle";
import { createStatusBar, type StatusBar } from "./ui/statusBar";

const PROBE_INTERVAL_MS = 10_000;

interface ActivationResources {
  readonly output: vscode.OutputChannel;
  readonly statusBar: StatusBar;
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

  const lifecycle = buildLifecycle(settings, statusBar, output);
  resources = { output, statusBar, lifecycle, probeTimer: undefined, settings };

  context.subscriptions.push(
    vscode.commands.registerCommand("rex.showStatus", async () => {
      const state = await resources?.lifecycle.probe();
      output.appendLine(`[command] showStatus -> ${describeState(state)}`);
      if (state?.kind === "ready") {
        void vscode.window.showInformationMessage(
          `REX ready (daemon ${state.status.daemonVersion}, uptime ${state.status.uptimeSeconds}s, model ${state.status.activeModelId || "unknown"}).`,
        );
      } else if (state?.kind === "starting") {
        void vscode.window.showInformationMessage("REX daemon is starting...");
      } else if (state?.kind === "unavailable") {
        void vscode.window.showWarningMessage(
          `REX daemon unavailable: ${state.reason}`,
        );
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
    onSettingsChanged((updated) => {
      output.appendLine(`[settings] changed -> ${summarizeSettings(updated)}`);
      if (resources === undefined) {
        return;
      }
      resources.settings = updated;
      resources.lifecycle = buildLifecycle(updated, statusBar, output);
      void resources.lifecycle.probe();
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

  void resources.lifecycle.probe();
  resources.probeTimer = setInterval(() => {
    void resources?.lifecycle.probe();
  }, PROBE_INTERVAL_MS);
}

export async function deactivate(): Promise<void> {
  if (resources === undefined) {
    return;
  }
  if (resources.probeTimer !== undefined) {
    clearInterval(resources.probeTimer);
  }
  await resources.lifecycle.shutdown();
  resources = undefined;
}

function buildLifecycle(
  settings: RexSettings,
  statusBar: StatusBar,
  output: vscode.OutputChannel,
): DaemonLifecycle {
  return new DaemonLifecycle({
    cli: { cliPath: settings.cliPath },
    daemonBinaryPath: settings.daemonBinaryPath,
    onState: (state) => {
      statusBar.update(state);
      output.appendLine(`[lifecycle] ${describeState(state)}`);
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
