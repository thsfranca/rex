import type { ExtensionToWebview, WebviewToExtension } from "../src/shared/messages";

declare global {
  interface Window {
    __REX_DEV__?: {
      toggleTheme: () => void;
      reseed: () => void;
    };
  }
}

const SAMPLE_ATTACHMENT = `import * as vscode from "vscode";
import type { ExtensionToWebview } from "../shared/messages";

export class ChatPanel {
  private postMessage(message: ExtensionToWebview): void {
    void this.webview.postMessage(message);
  }
}`;

const ASSISTANT_MARKDOWN = `Here is a minimal export you can adapt for the chat panel:

\`\`\`typescript
export function createChatPanel(context: vscode.ExtensionContext): void {
  const panel = vscode.window.createWebviewPanel(
    "rex.chat",
    "REX",
    vscode.ViewColumn.Beside,
    { enableScripts: true, retainContextWhenHidden: true },
  );
  panel.webview.html = buildHtml(panel.webview);
}

export function registerLongRunningToolHandlers(
  subscriptions: vscode.Disposable[],
): void {
  subscriptions.push(
    vscode.commands.registerCommand("rex.example.withVeryLongSignatureName", async () => {
      await vscode.window.showInformationMessage("Tool completed successfully.");
    }),
  );
}
\`\`\`

Use **Design Mode** to tweak spacing in \`themeVars.css\`. Long lines should scroll horizontally; blocks over 14 lines offer expand.`;

let themeKind: "dark" | "light" = "dark";

function postToWebview(message: ExtensionToWebview): void {
  window.postMessage(message, "*");
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function applyThemeDocument(kind: "dark" | "light"): void {
  themeKind = kind;
  document.documentElement.setAttribute("data-rex-theme", kind);
}

const STREAM_ID = "assistant-seed-stream";

async function seedUiState(): Promise<void> {
  const bootstrap: ExtensionToWebview[] = [
    { type: "daemonState", payload: { state: "ready", detail: "dev-harness" } },
    { type: "theme", payload: { kind: themeKind } },
    {
      type: "modeState",
      payload: {
        mode: "agent",
        canMutateFiles: true,
        requiresExecutionApproval: true,
        requiresMutationApproval: true,
        summary: "Agent mode: tools may run with approval.",
      },
    },
    {
      type: "contextSnapshot",
      context: {
        filePath: "src/example.ts",
        languageId: "typescript",
        selectionText: "function hello() { return 42; }",
      },
    },
    {
      type: "contextAttachments",
      attachments: [
        {
          id: "1",
          kind: "file",
          label: "chatPanel.ts",
          text: SAMPLE_ATTACHMENT,
        },
      ],
    },
    {
      type: "sessionList",
      sessions: [
        { id: "s1", title: "Session 1", isActive: true },
        { id: "s2", title: "Refactor plan", isActive: false },
      ],
    },
    {
      type: "sessionMessages",
      payload: {
        sessionId: "s1",
        messages: [
          {
            id: "user-seed-1",
            role: "user",
            buffer: "Show me how the chat panel is wired to the webview.",
          },
          {
            id: STREAM_ID,
            role: "assistant",
            buffer: "",
          },
        ],
      },
    },
    {
      type: "executionStep",
      payload: {
        id: "e1",
        phase: "running",
        summary: "read_file",
        kind: "tool",
        detail: "chatPanel.ts",
      },
    },
    {
      type: "approvalRequested",
      payload: {
        id: "a1",
        scope: "mutation",
        title: "Apply workspace edit",
        detail: "Changes to webview/components/Chat.tsx",
        edits: [
          {
            filePath: "webview/components/Chat.tsx",
            languageId: "typescript",
            before: `  const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>): void => {
    if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
      event.preventDefault();
      if (canSend) {
        props.onSubmit();
      }
    }`,
            after: `  const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>): void => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      if (canSend) {
        props.onSubmit();
      }
      return;
    }`,
          },
          {
            filePath: "webview/theme/themeVars.css",
            languageId: "css",
            before: "  padding: 8px 12px 12px;",
            after: "  padding: var(--rex-space-sm) var(--rex-space-lg) var(--rex-space-lg);",
          },
        ],
      },
    },
    {
      type: "approvalRequested",
      payload: {
        id: "a2",
        scope: "execution",
        title: "Run terminal command",
        detail: "npm test -- --run extensions/rex-vscode",
      },
    },
  ];

  for (const message of bootstrap) {
    postToWebview(message);
    await delay(16);
  }

  await streamAssistantReply();

  postToWebview({
    type: "planArtifact",
    payload: {
      streamId: "p1",
      phase: "ready",
      title: "UI polish plan",
      detail: "Tighten composer spacing and unify card borders.",
      content: "## Steps\n1. Tighten composer spacing\n2. Unify card borders",
      savePath: "TEMP_ui.md",
    },
  });
}

async function streamAssistantReply(): Promise<void> {
  postToWebview({ type: "streamStarted", id: STREAM_ID });

  const chunks = chunkMarkdown(ASSISTANT_MARKDOWN, 48);
  for (const chunk of chunks) {
    postToWebview({ type: "streamChunk", id: STREAM_ID, text: chunk });
    await delay(35);
  }

  postToWebview({ type: "streamDone", id: STREAM_ID });
}

function chunkMarkdown(text: string, size: number): string[] {
  const chunks: string[] = [];
  for (let index = 0; index < text.length; index += size) {
    chunks.push(text.slice(index, index + size));
  }
  return chunks;
}

function handleOutbound(message: WebviewToExtension): void {
  console.info("[rex dev] webview → host", message);

  if (message.type === "ready") {
    scheduleSeed();
  }
}

let seedTimer: number | undefined;

function scheduleSeed(): void {
  if (seedTimer !== undefined) {
    window.clearTimeout(seedTimer);
  }
  seedTimer = window.setTimeout(() => {
    seedTimer = undefined;
    void seedUiState();
  }, 50);
}

function installMockVsCodeApi(): void {
  window.acquireVsCodeApi = () => ({
    postMessage(message: WebviewToExtension): void {
      handleOutbound(message);
    },
    getState<T>(): T | undefined {
      return undefined;
    },
    setState<T>(_state: T): void {
      // no-op for harness
    },
  });
}

function toggleTheme(): void {
  const next = themeKind === "dark" ? "light" : "dark";
  applyThemeDocument(next);
  postToWebview({ type: "theme", payload: { kind: next } });
}

installMockVsCodeApi();
applyThemeDocument(themeKind);

window.__REX_DEV__ = {
  toggleTheme,
  reseed: () => {
    if (seedTimer !== undefined) {
      window.clearTimeout(seedTimer);
      seedTimer = undefined;
    }
    void seedUiState();
  },
};
