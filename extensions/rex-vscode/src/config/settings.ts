import * as vscode from "vscode";

import { resolveRexExecutable } from "./resolveExecutable";

export type ChatLocationSetting = "auto" | "right" | "left" | "editor";

export type EditorChatColumnSetting = "beside" | "active" | "one" | "two" | "three";

export interface RexSettings {
  readonly cliPath: string;
  readonly daemonBinaryPath: string;
  readonly daemonAutoStart: boolean;
  readonly modelId: string;
  readonly rexRoot: string;
  readonly chatLocation: ChatLocationSetting;
  readonly editorChatColumn: EditorChatColumnSetting;
}

const SECTION = "rex";

const CHAT_LOCATIONS = new Set<ChatLocationSetting>(["auto", "right", "left", "editor"]);
const EDITOR_CHAT_COLUMNS = new Set<EditorChatColumnSetting>([
  "beside",
  "active",
  "one",
  "two",
  "three",
]);

function parseChatLocation(value: unknown): ChatLocationSetting {
  return typeof value === "string" && CHAT_LOCATIONS.has(value as ChatLocationSetting)
    ? (value as ChatLocationSetting)
    : "auto";
}

function parseEditorChatColumn(value: unknown): EditorChatColumnSetting {
  return typeof value === "string" && EDITOR_CHAT_COLUMNS.has(value as EditorChatColumnSetting)
    ? (value as EditorChatColumnSetting)
    : "beside";
}

export function readSettings(): RexSettings {
  const config = vscode.workspace.getConfiguration(SECTION);
  const cliPathSetting = (config.get<string>("cliPath") ?? "rex").trim() || "rex";
  const daemonBinaryPathSetting =
    (config.get<string>("daemonBinaryPath") ?? "rex").trim() || "rex";
  return {
    cliPath: resolveRexExecutable(cliPathSetting),
    daemonBinaryPath: resolveRexExecutable(daemonBinaryPathSetting),
    daemonAutoStart: config.get<boolean>("daemonAutoStart") ?? true,
    modelId: (config.get<string>("modelId") ?? "").trim(),
    rexRoot: (config.get<string>("rexRoot") ?? "").trim(),
    chatLocation: parseChatLocation(config.get("chatLocation")),
    editorChatColumn: parseEditorChatColumn(config.get("editorChatColumn")),
  };
}

export function onSettingsChanged(
  listener: (settings: RexSettings, event?: vscode.ConfigurationChangeEvent) => void,
): vscode.Disposable {
  return vscode.workspace.onDidChangeConfiguration((event) => {
    if (event.affectsConfiguration(SECTION)) {
      listener(readSettings(), event);
    }
  });
}
