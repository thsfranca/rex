import fs from "node:fs";
import path from "node:path";
import toml from "toml";

export interface HarnessConfig {
  baseUrl: string;
  viewport: { width: number; height: number };
  baselineDir: string;
  staticRoot: string;
}

const DEFAULTS: HarnessConfig = {
  baseUrl: "file:///fixtures/ui_probe/static/index.html",
  viewport: { width: 1200, height: 800 },
  baselineDir: ".rex-ui-harness/baselines",
  staticRoot: "fixtures/ui_probe/static",
};

export function loadConfig(repoRoot: string): HarnessConfig {
  const configPath = path.join(repoRoot, "rex-ui-harness.toml");
  if (!fs.existsSync(configPath)) {
    return resolvePaths(repoRoot, DEFAULTS);
  }
  const raw = toml.parse(fs.readFileSync(configPath, "utf8")) as Record<string, unknown>;
  const viewport = (raw.viewport as { width?: number; height?: number }) ?? {};
  return resolvePaths(repoRoot, {
    baseUrl: (raw.base_url as string) ?? DEFAULTS.baseUrl,
    viewport: {
      width: viewport.width ?? DEFAULTS.viewport.width,
      height: viewport.height ?? DEFAULTS.viewport.height,
    },
    baselineDir: (raw.baseline_dir as string) ?? DEFAULTS.baselineDir,
    staticRoot: (raw.static_root as string) ?? DEFAULTS.staticRoot,
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
    baseUrl,
    baselineDir: path.join(repoRoot, cfg.baselineDir),
    staticRoot: staticAbs,
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
