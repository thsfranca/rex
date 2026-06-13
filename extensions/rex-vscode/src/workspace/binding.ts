import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";

const PROJECT_CONFIG_REL = path.join(".rex", "config.json");

export type WorkspaceBindingResult =
  | { readonly ok: true; readonly workspaceRoot: string; readonly multiRoot: boolean }
  | { readonly ok: false; readonly reason: string };

export function resolvePrimaryWorkspaceFolder(): string | undefined {
  const folders = vscode.workspace.workspaceFolders;
  if (folders === undefined || folders.length === 0) {
    return undefined;
  }
  return folders[0].uri.fsPath;
}

export function workspaceBindingState(): WorkspaceBindingResult {
  const folders = vscode.workspace.workspaceFolders;
  if (folders === undefined || folders.length === 0) {
    return {
      ok: false,
      reason: "no workspace folder open; open a folder before starting the daemon",
    };
  }
  const root = folders[0].uri.fsPath;
  return { ok: true, workspaceRoot: root, multiRoot: folders.length > 1 };
}

type RexConfigJson = Record<string, unknown>;

function readJsonIfExists(filePath: string): RexConfigJson {
  if (!fs.existsSync(filePath)) {
    return { version: 1 };
  }
  const raw = fs.readFileSync(filePath, "utf8");
  const parsed: unknown = JSON.parse(raw);
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error(`invalid JSON at ${filePath}`);
  }
  return parsed as RexConfigJson;
}

function mergeWorkspaceRoot(config: RexConfigJson, workspaceRoot: string): RexConfigJson {
  const workspace =
    config.workspace !== undefined &&
    typeof config.workspace === "object" &&
    !Array.isArray(config.workspace)
      ? { ...(config.workspace as Record<string, unknown>) }
      : {};
  workspace.root = workspaceRoot;
  return { ...config, version: config.version ?? 1, workspace };
}

function applyProductAgentOverlay(config: RexConfigJson): RexConfigJson {
  const sidecars =
    config.sidecars !== undefined &&
    typeof config.sidecars === "object" &&
    !Array.isArray(config.sidecars)
      ? { ...(config.sidecars as Record<string, unknown>) }
      : {};
  sidecars.active = "agent";
  sidecars.required = sidecars.required ?? true;
  const list = Array.isArray(sidecars.list) ? [...(sidecars.list as unknown[])] : [];
  const hasAgent = list.some(
    (entry) =>
      entry !== null &&
      typeof entry === "object" &&
      (entry as { name?: string }).name === "agent",
  );
  if (!hasAgent) {
    list.push({
      name: "agent",
      binary: "rex-agent",
      enabled: true,
      socket: "/tmp/rex-sidecar.sock",
    });
  }
  sidecars.list = list;
  const agent =
    config.agent !== undefined &&
    typeof config.agent === "object" &&
    !Array.isArray(config.agent)
      ? { ...(config.agent as Record<string, unknown>) }
      : {};
  agent.approvals_enabled = true;
  if (agent.max_tool_steps_ask === undefined) {
    agent.max_tool_steps_ask = 12;
  }

  const search =
    config.search !== undefined &&
    typeof config.search === "object" &&
    !Array.isArray(config.search)
      ? { ...(config.search as Record<string, unknown>) }
      : {};
  if (search.enabled === undefined) {
    search.enabled = true;
    search.provider = search.provider ?? "mock";
  }

  return { ...config, sidecars, agent, search, version: config.version ?? 1 };
}

/** Exported for unit tests. */
export function applyProductAgentOverlayForTest(config: RexConfigJson): RexConfigJson {
  return applyProductAgentOverlay(config);
}

export function ensureProjectRexConfig(workspaceRoot: string): void {
  const rexDir = path.join(workspaceRoot, ".rex");
  const configPath = path.join(workspaceRoot, PROJECT_CONFIG_REL);
  fs.mkdirSync(rexDir, { recursive: true });
  let config = readJsonIfExists(configPath);
  config = mergeWorkspaceRoot(config, workspaceRoot);
  config = applyProductAgentOverlay(config);
  fs.writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`, "utf8");
}
