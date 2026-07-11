import { stopHarnessDesktopApps } from "./desktopProcess.js";
import { stopWebUiServer } from "./devServer.js";
import { pageFill, pageFocus, pagePress, pageWaitForSelector, pageWaitForText, readObservabilitySnapshot, } from "./page.js";
let session = null;
let desktopRepoRoot = null;
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
    // Product shell is Electron (apps/rex-desktop). Playwright-electron session + daemon
    // bridge IPC land in W127/W129. Desktop CI uses compositor proof until then.
    desktopRepoRoot = cfg.repoRoot;
    throw new Error("Desktop ui_open awaits Electron daemon bridge (W127) and Playwright-electron harness (W129). " +
        "Run ./scripts/ci/run_electron_compositor_proof.sh for the compositor gate, or use build-mode harness checks.");
}
export async function closeSession() {
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
