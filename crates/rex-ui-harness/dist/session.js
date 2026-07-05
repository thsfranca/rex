import { PluginClient, TauriPage, TauriProcessManager, } from "@srsholmes/tauri-playwright";
import { harnessDesktopCwd, resolveDesktopBinary, resolveRexBinary, resetProbeDaemon, stopHarnessDesktopApps, } from "./desktopProcess.js";
import { ensureWebUiServer, stopWebUiServer } from "./devServer.js";
import { pageFill, pageFocus, pagePress, pageWaitForSelector, pageWaitForText, readObservabilitySnapshot, } from "./page.js";
let session = null;
let staticBrowser = null;
let staticContext = null;
let processManager = null;
let pluginClient = null;
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
    if (mode === "desktop") {
        return openDesktopSession(cfg);
    }
    return openStaticSession(cfg, launch.headless !== false);
}
async function openStaticSession(cfg, headless) {
    const { chromium } = await import("playwright");
    staticBrowser = await chromium.launch({ headless });
    staticContext = await staticBrowser.newContext({ viewport: cfg.viewport });
    const page = await staticContext.newPage();
    await page.clock.install({ time: new Date("2026-01-01T00:00:00Z") });
    await page.goto(cfg.baseUrl, { waitUntil: "domcontentloaded" });
    session = { mode: "static", page, motionFrames: [], recording: false };
    return session;
}
async function openDesktopSession(cfg) {
    if (process.platform !== "darwin") {
        throw new Error('Desktop mode requires macOS. Set launch.mode = "static" in rex-ui-harness.toml.');
    }
    process.env.REX_ROOT = cfg.rexRoot;
    process.env.REX_SIDECAR_HARNESS = "direct";
    process.env.TAURI_PLAYWRIGHT_SOCKET = cfg.desktopSocket;
    await stopHarnessDesktopApps(cfg.repoRoot);
    await resetProbeDaemon(cfg.rexRoot);
    await ensureWebUiServer(cfg.repoRoot);
    desktopRepoRoot = cfg.repoRoot;
    const harnessCwd = harnessDesktopCwd();
    const rexBin = resolveRexBinary(cfg.repoRoot);
    const desktopBin = resolveDesktopBinary(cfg.repoRoot);
    process.env.CARGO_BIN_EXE_rex = rexBin;
    process.env.REX_BIN = rexBin;
    processManager = new TauriProcessManager({
        command: desktopBin,
        args: [],
        cwd: harnessCwd,
        socketPath: cfg.desktopSocket,
        startTimeout: cfg.desktopStartTimeoutSecs,
    });
    await processManager.start();
    pluginClient = new PluginClient(cfg.desktopSocket);
    await pluginClient.connect();
    const tauriPage = new TauriPage(pluginClient);
    tauriPage.setDefaultTimeout(60_000);
    session = { mode: "desktop", page: tauriPage, motionFrames: [], recording: false };
    await pageWaitForSelector(session, '[data-testid="shell"]', 60_000);
    await pageWaitForText(session, "Ready", 60_000);
    return session;
}
export async function closeSession() {
    pluginClient?.disconnect();
    pluginClient = null;
    processManager?.stop();
    processManager = null;
    await stopWebUiServer();
    if (desktopRepoRoot) {
        await stopHarnessDesktopApps(desktopRepoRoot);
        desktopRepoRoot = null;
    }
    if (staticContext) {
        await staticContext.close();
        staticContext = null;
    }
    if (staticBrowser) {
        await staticBrowser.close();
        staticBrowser = null;
    }
    session = null;
}
export async function gotoScenario(scenario) {
    const s = getSession();
    if (s.mode === "static") {
        await s.page.evaluate((name) => {
            const probe = window
                .__rexProbe;
            if (!probe)
                throw new Error("Probe harness not loaded");
            return probe.gotoScenario(name);
        }, scenario);
        await s.page.waitForTimeout(50);
        return;
    }
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
        case "error":
        case "history-fetch":
            throw new Error(`${scenario} scenario is static-fixture only`);
        default:
            throw new Error(`Unknown scenario: ${scenario}`);
    }
}
