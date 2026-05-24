import * as vscode from "vscode";

export interface RexSettings {
  readonly cliPath: string;
  readonly daemonBinaryPath: string;
  readonly daemonAutoStart: boolean;
  readonly modelId: string;
  readonly rexRoot: string;
}

const SECTION = "rex";

export function readSettings(): RexSettings {
  const config = vscode.workspace.getConfiguration(SECTION);
  return {
    cliPath: (config.get<string>("cliPath") ?? "rex").trim() || "rex",
    daemonBinaryPath:
      (config.get<string>("daemonBinaryPath") ?? "rex").trim() || "rex",
    daemonAutoStart: config.get<boolean>("daemonAutoStart") ?? false,
    modelId: (config.get<string>("modelId") ?? "").trim(),
    rexRoot: (config.get<string>("rexRoot") ?? "").trim(),
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
