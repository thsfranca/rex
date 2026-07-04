import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import toml from "toml";

export type HarnessMode = "desktop" | "static";

export interface HarnessConfig {
  mode: HarnessMode;
  repoRoot: string;
  baseUrl: string;
  viewport: { width: number; height: number };
  baselineDir: string;
  staticRoot: string;
  rexRoot: string;
  workspaceDir: string;
  desktopSocket: string;
  desktopStartTimeoutSecs: number;
}

const DEFAULT_MODE: HarnessMode = os.platform() === "darwin" ? "desktop" : "static";

const DEFAULTS: HarnessConfig = {
  mode: DEFAULT_MODE,
  repoRoot: "",
  baseUrl: "file:///fixtures/ui_probe/static/index.html",
  viewport: { width: 1200, height: 800 },
  baselineDir: ".rex-ui-harness/baselines",
  staticRoot: "fixtures/ui_probe/static",
  rexRoot: "fixtures/ui_probe/rex_root",
  workspaceDir: "fixtures/ui_probe/workspace",
  desktopSocket: "/tmp/rex-playwright.sock",
  desktopStartTimeoutSecs: 180,
};

export function loadConfig(repoRoot: string): HarnessConfig {
  const configPath = path.join(repoRoot, "rex-ui-harness.toml");
  if (!fs.existsSync(configPath)) {
    return resolvePaths(repoRoot, DEFAULTS);
  }
  const raw = toml.parse(fs.readFileSync(configPath, "utf8")) as Record<string, unknown>;
  const viewport = (raw.viewport as { width?: number; height?: number }) ?? {};
  const launch = (raw.launch as { mode?: string }) ?? {};
  const desktop = (raw.desktop as Record<string, unknown>) ?? {};
  const modeRaw = (launch.mode as string | undefined) ?? DEFAULT_MODE;
  const mode: HarnessMode = modeRaw === "static" ? "static" : "desktop";
  return resolvePaths(repoRoot, {
    mode,
    repoRoot: "",
    baseUrl: (raw.base_url as string) ?? DEFAULTS.baseUrl,
    viewport: {
      width: viewport.width ?? DEFAULTS.viewport.width,
      height: viewport.height ?? DEFAULTS.viewport.height,
    },
    baselineDir: (raw.baseline_dir as string) ?? DEFAULTS.baselineDir,
    staticRoot: (raw.static_root as string) ?? DEFAULTS.staticRoot,
    rexRoot: (desktop.rex_root as string) ?? DEFAULTS.rexRoot,
    workspaceDir: (desktop.workspace_dir as string) ?? DEFAULTS.workspaceDir,
    desktopSocket: (desktop.socket as string) ?? DEFAULTS.desktopSocket,
    desktopStartTimeoutSecs:
      Number(desktop.start_timeout_secs ?? DEFAULTS.desktopStartTimeoutSecs),
  });
}

function resolvePaths(repoRoot: string, cfg: HarnessConfig): HarnessConfig {
  const staticAbs = path.join(repoRoot, cfg.staticRoot);
  const indexFile = path.join(staticAbs, "index.html");
  const baseUrl =
    cfg.baseUrl.startsWith("file://") && !cfg.baseUrl.includes("://fixtures")
      ? cfg.baseUrl
      : `file://${indexFile}`;
  return {
    ...cfg,
    repoRoot,
    baseUrl,
    baselineDir: path.join(repoRoot, cfg.baselineDir),
    staticRoot: staticAbs,
    rexRoot: path.join(repoRoot, cfg.rexRoot),
    workspaceDir: path.join(repoRoot, cfg.workspaceDir),
  };
}

export function findRepoRoot(start: string): string {
  let dir = path.resolve(start);
  while (true) {
    if (fs.existsSync(path.join(dir, "Cargo.toml")) && fs.existsSync(path.join(dir, "crates"))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) return start;
    dir = parent;
  }
}
