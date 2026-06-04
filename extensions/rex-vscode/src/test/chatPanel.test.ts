import * as path from "node:path";
import { describe, expect, it, vi } from "vitest";
import { EventEmitter, Uri } from "vscode";

import type { ExtensionToWebview, WebviewToExtension } from "../shared/messages";
import type { StatusSnapshot } from "../runtime/cliBridge";
import { ChatPanelProvider } from "../ui/chatPanel";

function fixtureBinary(name: string): string {
  return path.resolve(__dirname, "fixtures", name);
}

const TEST_STATUS: StatusSnapshot = {
  daemonVersion: "test",
  activeModelId: "test",
  uptimeSeconds: 1,
  capturedAt: 1,
};

function terminalMessages(
  calls: ReadonlyArray<readonly [ExtensionToWebview]>,
): ExtensionToWebview[] {
  return calls
    .map(([message]) => message)
    .filter((message) => message.type === "streamDone" || message.type === "streamError");
}

async function waitFor(
  predicate: () => boolean,
  timeoutMs = 3_000,
  intervalMs = 25,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (predicate()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }
  throw new Error("waitFor timed out");
}

function makeHarness(cliPath: string) {
  const postMessage = vi.fn<(message: ExtensionToWebview) => void>();
  const messageEmitter = new EventEmitter<unknown>();

  const webview = {
    options: {} as Record<string, unknown>,
    html: "",
    postMessage,
    onDidReceiveMessage: (listener: (raw: unknown) => void) => messageEmitter.event(listener),
    asWebviewUri: (uri: Uri) => uri,
    cspSource: "https://example.test",
  };

  const view = {
    webview,
    onDidDispose: () => ({ dispose: () => undefined }),
    show: vi.fn(),
  };

  const context = {
    extensionUri: Uri.from({ scheme: "file", path: "/tmp/rex-extension" }),
    subscriptions: [],
  };

  const provider = new ChatPanelProvider({
    context: context as never,
    getCliOptions: () => ({ cliPath, timeoutMs: 5_000 }),
    getModelId: () => "",
    getDaemonAutoStart: () => false,
    ensureDaemonReady: async () => ({
      kind: "ready",
      status: TEST_STATUS,
    }),
    getDaemonState: () => ({
      kind: "ready",
      status: TEST_STATUS,
    }),
    log: () => undefined,
  });

  provider.resolveWebviewView(view as never);

  async function receive(message: WebviewToExtension): Promise<void> {
    messageEmitter.fire(message);
    await new Promise((resolve) => setImmediate(resolve));
  }

  return { postMessage, receive };
}

describe("ChatPanelProvider — cancel to idle", () => {
  it("posts exactly one cancelled streamError when cancel interrupts a slow stream", async () => {
    const { postMessage, receive } = makeHarness(fixtureBinary("cli_slow.sh"));
    const streamId = "stream-cancel-mid-flight";

    void receive({
      type: "submitPrompt",
      id: streamId,
      prompt: "hello",
      attachContext: false,
    });

    await waitFor(() =>
      postMessage.mock.calls.some(([message]) => message.type === "streamChunk" && message.id === streamId),
    );

    await receive({ type: "cancelStream", id: streamId });

    await waitFor(() =>
      postMessage.mock.calls.some(
        ([message]) =>
          message.type === "streamError" && message.id === streamId && message.code === "cancelled",
      ),
    );

    const terminals = terminalMessages(postMessage.mock.calls).filter(
      (message) => "id" in message && message.id === streamId,
    );
    expect(terminals).toHaveLength(1);
    expect(terminals[0]).toMatchObject({
      type: "streamError",
      code: "cancelled",
    });

    const terminalIndex = postMessage.mock.calls.findIndex(
      ([message]) =>
        message.type === "streamError" && message.id === streamId && message.code === "cancelled",
    );
    const afterTerminal = postMessage.mock.calls.slice(terminalIndex + 1);
    expect(
      afterTerminal.some(
        ([message]) =>
          (message.type === "streamDone" || message.type === "streamError") &&
          "id" in message &&
          message.id === streamId,
      ),
    ).toBe(false);
  });

  it("returns cancelled streamError when cancel arrives during agent execution approval", async () => {
    const { postMessage, receive } = makeHarness(fixtureBinary("cli_success.sh"));
    const streamId = "stream-cancel-approval";

    await receive({ type: "setMode", mode: "agent" });
    postMessage.mockClear();

    void receive({
      type: "submitPrompt",
      id: streamId,
      prompt: "run agent",
      attachContext: false,
    });

    await waitFor(() =>
      postMessage.mock.calls.some(([message]) => message.type === "approvalRequested"),
    );

    await receive({ type: "cancelStream", id: streamId });

    await waitFor(() =>
      postMessage.mock.calls.some(
        ([message]) =>
          message.type === "streamError" && message.id === streamId && message.code === "cancelled",
      ),
    );

    const terminals = terminalMessages(postMessage.mock.calls).filter(
      (message) => "id" in message && message.id === streamId,
    );
    expect(terminals).toHaveLength(1);
    expect(terminals[0]).toMatchObject({
      type: "streamError",
      code: "cancelled",
    });
  });
});
