import * as vscode from "vscode";

import { readSettings, onSettingsChanged, type RexSettings } from "./config/settings";
import { snapshotActiveEditor } from "./editor/context";
import { activateCursorAdapter } from "./platform/cursorAdapter";
import { applyChatLayoutContext, resolveEditorViewColumn } from "./platform/editorLayout";
import { DaemonLifecycle, type DaemonLifecycleState } from "./runtime/daemonLifecycle";
import { streamFailureWantsSetupHint } from "./runtime/userActionableFailure";
import { ChatPanelProvider } from "./ui/chatPanel";
import { focusRexChat } from "./ui/focusChat";
import { openEditorChatPanel } from "./ui/editorChatPanel";
import { runInlineEditOnSelection } from "./ui/inlineEdit";
import { createStatusBar, type StatusBar } from "./ui/statusBar";
import {
  ensureProjectRexConfig,
  workspaceBindingState,
} from "./workspace/binding";

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
  const settings = readSettings();
  await applyChatLayoutContext(settings.chatLocation);
  const output = vscode.window.createOutputChannel("REX");
  context.subscriptions.push(output);
  const statusBar = createStatusBar();
  context.subscriptions.push({ dispose: () => statusBar.dispose() });
  output.appendLine(`[activate] settings: ${summarizeSettings(settings)}`);

  const chatPanel = new ChatPanelProvider({
    context,
    getCliOptions: () => {
      const cliPath = resources?.settings.cliPath ?? settings.cliPath;
      const binding = workspaceBindingState();
      return binding.ok
        ? { cliPath, cwd: binding.workspaceRoot }
        : { cliPath };
    },
    getModelId: () => resources?.settings.modelId ?? settings.modelId,
    getDaemonAutoStart: () => resources?.settings.daemonAutoStart ?? settings.daemonAutoStart,
    ensureDaemonReady: (signal) => {
      const r = resources;
      if (r === undefined) {
        return Promise.resolve({
          kind: "unavailable",
          reason: "REX extension is not active",
        } as DaemonLifecycleState);
      }
      return ensureDaemonWithWorkspaceBinding(r.lifecycle, r.output, signal);
    },
    getDaemonState: () => lastLifecycleState,
    log: (message) => output.appendLine(message),
    onStreamActivity: (hint) => statusBar.setStreamingActivity(hint),
    notifyStreamFailure: ({ code, message }) => {
      const firstLine = message.split("\n")[0].trim();
      output.appendLine(`[chat] terminal_error code=${code} detail=${firstLine}`);
      if (!streamFailureWantsSetupHint(code, message)) {
        return;
      }
      void vscode.window
        .showWarningMessage(
          `REX: ${firstLine}`,
          "How to start daemon",
          "Open REX output",
        )
        .then((choice) => {
          if (choice === "How to start daemon") {
            void vscode.commands.executeCommand("rex.howToStartDaemon");
          } else if (choice === "Open REX output") {
            output.show();
            void vscode.commands.executeCommand("rex.openOutput");
          }
        });
    },
  });
  context.subscriptions.push(chatPanel.register());

  let lastLifecycleState: DaemonLifecycleState | undefined;

  const initialBinding = workspaceBindingState();
  let initialSpawnCwd: string | undefined;
  if (initialBinding.ok) {
    try {
      ensureProjectRexConfig(initialBinding.workspaceRoot);
      initialSpawnCwd = initialBinding.workspaceRoot;
      if (initialBinding.multiRoot) {
        output.appendLine("[activate] workspace.warning=multi_root (binding primary folder only)");
      }
    } catch (err) {
      output.appendLine(`[activate] workspace binding failed: ${String(err)}`);
    }
  }

  const lifecycle = buildLifecycle(settings, statusBar, output, initialSpawnCwd, (state) => {
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
        "Product path: run rex config init, set inference.openai_compat and sidecars.active=agent (binary rex-agent) in JSON, then rex daemon. See docs/EXTENSION_LOCAL_E2E.md §3 and docs/CONFIGURATION.md in the REX repository.",
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
      const current = resources?.settings ?? readSettings();
      await focusRexChat(context, current, (raw) => chatPanel.handleExternalMessage(raw));
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("rex.openChatInEditor", () => {
      const current = resources?.settings ?? readSettings();
      openEditorChatPanel(
        context,
        (raw) => chatPanel.handleExternalMessage(raw),
        resolveEditorViewColumn(current.editorChatColumn),
      );
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("rex.cancelStream", () => {
      resources?.chatPanel.cancelActiveStream();
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("rex.inlineEditSelection", async () => {
      const panel = resources?.chatPanel;
      if (panel === undefined) {
        return;
      }
      await runInlineEditOnSelection({
        getCliOptions: () => {
          const cliPath = resources?.settings.cliPath ?? settings.cliPath;
          const binding = workspaceBindingState();
          return binding.ok ? { cliPath, cwd: binding.workspaceRoot } : { cliPath };
        },
        getModelId: () => resources?.settings.modelId ?? settings.modelId,
        getProposalProvider: () => panel.getProposalProvider(),
        log: (message) => output.appendLine(message),
        chatPanel: panel,
      });
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("rex.sendTerminalSelectionToRex", async () => {
      const selection = await getTerminalSelection();
      if (selection === undefined || selection.trim().length === 0) {
        void vscode.window.showWarningMessage("Select text in the terminal first.");
        return;
      }
      resources?.chatPanel.attachTerminalContext(selection);
      await vscode.commands.executeCommand("rex.focusChat");
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
    onSettingsChanged(async (updated, event) => {
      output.appendLine(`[settings] changed -> ${summarizeSettings(updated)}`);
      if (resources === undefined) {
        return;
      }
      const chatLocationChanged =
        event === undefined || event.affectsConfiguration("rex.chatLocation");
      if (chatLocationChanged) {
        await applyChatLayoutContext(updated.chatLocation);
        void vscode.window
          .showInformationMessage(
            "REX chat location changed. Reload the window for the sidebar container to move.",
            "Reload Window",
          )
          .then((choice) => {
            if (choice === "Reload Window") {
              void vscode.commands.executeCommand("workbench.action.reloadWindow");
            }
          });
      }
      const previousLifecycle = resources.lifecycle;
      resources.settings = updated;
      resources.lifecycle = buildLifecycle(updated, statusBar, output, undefined, (state) => {
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
    void ensureDaemonWithWorkspaceBinding(lifecycle, output).then((state) => {
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
    await focusRexChat(context, resources?.settings ?? readSettings(), (raw) =>
      chatPanel.handleExternalMessage(raw),
    );
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
  spawnCwd: string | undefined,
  onStateChange: (state: DaemonLifecycleState) => void,
): DaemonLifecycle {
  const daemonEnv =
    settings.rexRoot.length > 0 ? { REX_ROOT: settings.rexRoot } : undefined;
  return new DaemonLifecycle({
    cli: { cliPath: settings.cliPath, cwd: spawnCwd },
    daemonBinaryPath: settings.daemonBinaryPath,
    daemonEnv,
    spawnCwd,
    onState: (state) => {
      statusBar.update(state);
      output.appendLine(`[lifecycle] ${describeState(state)}`);
      onStateChange(state);
    },
  });
}

async function ensureDaemonWithWorkspaceBinding(
  lifecycle: DaemonLifecycle,
  output: vscode.OutputChannel,
  signal?: AbortSignal,
): Promise<DaemonLifecycleState> {
  const binding = workspaceBindingState();
  if (!binding.ok) {
    output.appendLine(`[lifecycle] workspace binding skipped: ${binding.reason}`);
    return { kind: "unavailable", reason: binding.reason };
  }
  try {
    ensureProjectRexConfig(binding.workspaceRoot);
    if (binding.multiRoot) {
      output.appendLine("[lifecycle] workspace.warning=multi_root (binding primary folder only)");
    }
  } catch (err) {
    const reason = `failed to write project .rex/config.json: ${String(err)}`;
    output.appendLine(`[lifecycle] ${reason}`);
    return { kind: "unavailable", reason };
  }
  return lifecycle.ensureRunning(signal);
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
    return ensureDaemonWithWorkspaceBinding(r.lifecycle, r.output);
  }
  return r.lifecycle.probe();
}

async function getTerminalSelection(): Promise<string | undefined> {
  const previousClipboard = await vscode.env.clipboard.readText();
  await vscode.commands.executeCommand("workbench.action.terminal.copySelection");
  const selection = await vscode.env.clipboard.readText();
  if (selection.length === 0 || selection === previousClipboard) {
    return undefined;
  }
  return selection;
}
