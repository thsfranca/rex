import * as path from "node:path";

import * as vscode from "vscode";

import { detectCapabilities, type EditorCapabilities } from "./capabilities";

export interface CursorActivationResult {
  readonly capabilities: EditorCapabilities;
  readonly registeredPluginPath?: string;
}

/**
 * Register Cursor-only surfaces when running in Cursor. A no-op in plain VS Code.
 *
 * Today this only registers the bundled `cursor-plugins/` directory so Cursor
 * can load any plugin manifests shipped with the extension. MCP registration
 * is deferred behind a feature flag until REX exposes an MCP endpoint.
 */
export async function activateCursorAdapter(
  context: Pick<vscode.ExtensionContext, "extensionPath">,
  log: (message: string) => void = () => undefined,
): Promise<CursorActivationResult> {
  const capabilities = detectCapabilities();
  if (!capabilities.hasCursor) {
    return { capabilities };
  }
  if (!capabilities.hasCursorPlugins) {
    return { capabilities };
  }
  const pluginsPath = path.join(context.extensionPath, "cursor-plugins");
  try {
    await (vscode as unknown as {
      cursor: { plugins: { registerPath(p: string): Promise<void> | void } };
    }).cursor.plugins.registerPath(pluginsPath);
    log(`cursor plugins path registered: ${pluginsPath}`);
    return { capabilities, registeredPluginPath: pluginsPath };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    log(`cursor plugins registration failed: ${message}`);
    return { capabilities };
  }
}
