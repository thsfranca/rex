import * as vscode from "vscode";

import { resolveRexExecutable } from "./resolveExecutable";

export interface RexSettings {
  readonly cliPath: string;
  readonly daemonBinaryPath: string;
  readonly daemonAutoStart: boolean;
  readonly modelId: string;
  readonly rexRoot: string;
  readonly productAgentConfig: boolean;
}

const SECTION = "rex";

export function readSettings(): RexSettings {
  const config = vscode.workspace.getConfiguration(SECTION);
  const cliPathSetting = (config.get<string>("cliPath") ?? "rex").trim() || "rex";
  const daemonBinaryPathSetting =
    (config.get<string>("daemonBinaryPath") ?? "rex").trim() || "rex";
  return {
    cliPath: resolveRexExecutable(cliPathSetting),
    daemonBinaryPath: resolveRexExecutable(daemonBinaryPathSetting),
    daemonAutoStart: config.get<boolean>("daemonAutoStart") ?? false,
    modelId: (config.get<string>("modelId") ?? "").trim(),
    rexRoot: (config.get<string>("rexRoot") ?? "").trim(),
    productAgentConfig: config.get<boolean>("productAgentConfig") ?? true,
  };
}

export function onSettingsChanged(
  listener: (settings: RexSettings) => void,
): vscode.Disposable {
  return vscode.workspace.onDidChangeConfiguration((event) => {
    if (event.affectsConfiguration(SECTION)) {
      listener(readSettings());
    }
  });
}
