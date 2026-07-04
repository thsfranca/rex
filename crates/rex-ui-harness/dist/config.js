import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import toml from "toml";
const DEFAULT_MODE = os.platform() === "darwin" ? "desktop" : "static";
const DEFAULTS = {
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
export function loadConfig(repoRoot) {
    const configPath = path.join(repoRoot, "rex-ui-harness.toml");
    if (!fs.existsSync(configPath)) {
        return resolvePaths(repoRoot, DEFAULTS);
    }
    const raw = toml.parse(fs.readFileSync(configPath, "utf8"));
    const viewport = raw.viewport ?? {};
    const launch = raw.launch ?? {};
    const desktop = raw.desktop ?? {};
    const modeRaw = launch.mode ?? DEFAULT_MODE;
    const mode = modeRaw === "static" ? "static" : "desktop";
    return resolvePaths(repoRoot, {
        mode,
        repoRoot: "",
        baseUrl: raw.base_url ?? DEFAULTS.baseUrl,
        viewport: {
            width: viewport.width ?? DEFAULTS.viewport.width,
            height: viewport.height ?? DEFAULTS.viewport.height,
        },
        baselineDir: raw.baseline_dir ?? DEFAULTS.baselineDir,
        staticRoot: raw.static_root ?? DEFAULTS.staticRoot,
        rexRoot: desktop.rex_root ?? DEFAULTS.rexRoot,
        workspaceDir: desktop.workspace_dir ?? DEFAULTS.workspaceDir,
        desktopSocket: desktop.socket ?? DEFAULTS.desktopSocket,
        desktopStartTimeoutSecs: Number(desktop.start_timeout_secs ?? DEFAULTS.desktopStartTimeoutSecs),
    });
}
function resolvePaths(repoRoot, cfg) {
    const staticAbs = path.join(repoRoot, cfg.staticRoot);
    const indexFile = path.join(staticAbs, "index.html");
    const baseUrl = cfg.baseUrl.startsWith("file://") && !cfg.baseUrl.includes("://fixtures")
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
export function findRepoRoot(start) {
    let dir = path.resolve(start);
    while (true) {
        if (fs.existsSync(path.join(dir, "Cargo.toml")) && fs.existsSync(path.join(dir, "crates"))) {
            return dir;
        }
        const parent = path.dirname(dir);
        if (parent === dir)
            return start;
        dir = parent;
    }
}
