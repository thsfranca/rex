import * as vscode from "vscode";

export interface EditorCapabilities {
  readonly hasCursor: boolean;
  readonly hasCursorPlugins: boolean;
  readonly hasCursorMcp: boolean;
}

export interface CapabilitySource {
  readonly cursor?: unknown;
}

/**
 * Probe `vscode.cursor` at runtime without throwing if the namespace or any
 * sub-namespace is absent (plain VS Code). Returns a structured set of flags
 * so callers can enable Cursor-only features granularly.
 *
 * The `source` parameter exists only for testing; production code relies on
 * the default which reads the live `vscode` module.
 */
export function detectCapabilities(
  source: CapabilitySource = vscode as unknown as CapabilitySource,
): EditorCapabilities {
  const maybeCursor = source.cursor;
  if (!isRecord(maybeCursor)) {
    return { hasCursor: false, hasCursorPlugins: false, hasCursorMcp: false };
  }
  const plugins = maybeCursor["plugins"];
  const mcp = maybeCursor["mcp"];
  return {
    hasCursor: true,
    hasCursorPlugins:
      isRecord(plugins) && typeof plugins["registerPath"] === "function",
    hasCursorMcp: isRecord(mcp) && typeof mcp["registerServer"] === "function",
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
