import { createRequire } from "node:module";
import path from "node:path";
import { harnessDesktopCwd, resetProbeDaemon, resolveDesktopAppDir, resolveElectronExecutable, resolveRexBinary, stopHarnessDesktopApps, } from "./desktopProcess.js";
import { stopWebUiServer } from "./devServer.js";
import { pageFill, pageFocus, pagePress, pageWaitForSelector, pageWaitForText, readObservabilitySnapshot, } from "./page.js";
const require = createRequire(import.meta.url);
let session = null;
let desktopRepoRoot = null;
let electronApp = null;
export function getSession() {
    if (!session)
        throw new Error("No active session — call ui_open first");
    return session;
}
export async function openSession(cfg, launch = {}) {
    if (session)
        await closeSession();
    const mode = launch.mode ?? cfg.mode;
    if (mode === "build") {
        throw new Error("Build mode does not open a browser session");
    }
    return openDesktopSession(cfg);
}
async function openDesktopSession(cfg) {
    if (process.platform !== "darwin") {
        throw new Error("Desktop mode requires macOS.");
    }
    desktopRepoRoot = cfg.repoRoot;
    await resetProbeDaemon(cfg.rexRoot);
    await stopHarnessDesktopApps(cfg.repoRoot);
    const appDir = resolveDesktopAppDir(cfg.repoRoot);
    const electronPath = resolveElectronExecutable(cfg.repoRoot);
    const rexBin = resolveRexBinary(cfg.repoRoot);
    const { _electron: electron } = require("playwright");
    const timeoutMs = Math.max(30_000, cfg.desktopStartTimeoutSecs * 1000);
    electronApp = await electron.launch({
        executablePath: electronPath,
        args: [appDir],
        cwd: harnessDesktopCwd(),
        timeout: timeoutMs,
        env: {
            ...process.env,
            REX_ROOT: cfg.rexRoot,
            REX_BIN: rexBin,
            REX_SIDECAR_HARNESS: process.env.REX_SIDECAR_HARNESS ?? "direct",
            REX_DESKTOP_HOST: "electron",
            PATH: `${path.dirname(rexBin)}${path.delimiter}${process.env.PATH ?? ""}`,
        },
    });
    const page = await electronApp.firstWindow();
    await pageWaitForSelector({ mode: "desktop", page, motionFrames: [], recording: false }, '[data-testid="shell"]', timeoutMs);
    await pageWaitForText({ mode: "desktop", page, motionFrames: [], recording: false }, "Ready", timeoutMs);
    session = { mode: "desktop", page, motionFrames: [], recording: false };
    return session;
}
export async function closeSession() {
    if (electronApp) {
        try {
            await electronApp.close();
        }
        catch {
            // App may already be closed.
        }
        electronApp = null;
    }
    await stopWebUiServer();
    if (desktopRepoRoot) {
        await stopHarnessDesktopApps(desktopRepoRoot);
        desktopRepoRoot = null;
    }
    session = null;
}
export async function dumpObservability(label) {
    const s = getSession();
    const snapshot = await readObservabilitySnapshot(s);
    return `${label}: ${JSON.stringify(snapshot)}`;
}
export async function gotoScenario(scenario) {
    const s = getSession();
    switch (scenario) {
        case "idle":
            await pageWaitForText(s, "Ready", 30_000);
            break;
        case "streaming": {
            await pageFill(s, '[data-testid="composer-input"]', "hello from harness");
            await pagePress(s, "Enter");
            await pageWaitForSelector(s, "#status-dot.working", 30_000);
            break;
        }
        case "approval_required": {
            await pageFocus(s, '[data-testid="composer-input"]');
            await pageFill(s, '[data-testid="composer-input"]', "__approval_probe__");
            await pagePress(s, "Enter");
            try {
                await pageWaitForSelector(s, '[data-testid="modal-backdrop"]', 30_000);
            }
            catch (err) {
                const obs = await readObservabilitySnapshot(s);
                throw new Error(`${err instanceof Error ? err.message : String(err)}\nUI observability: ${JSON.stringify(obs)}`);
            }
            break;
        }
        case "error": {
            await pageFocus(s, '[data-testid="composer-input"]');
            await pageFill(s, '[data-testid="composer-input"]', "__error_probe__");
            await pagePress(s, "Enter");
            await pageWaitForSelector(s, '[data-testid="error-banner"]', 30_000);
            break;
        }
        default:
            throw new Error(`Unknown scenario: ${scenario}`);
    }
}
