import type { Page } from "playwright";
import {
  PluginClient,
  TauriPage,
  TauriProcessManager,
} from "@srsholmes/tauri-playwright";
import type { HarnessConfig } from "./config.js";
import {
  harnessDesktopCwd,
  resolveDesktopBinary,
  resolveRexBinary,
  resetProbeDaemon,
  stopHarnessDesktopApps,
} from "./desktopProcess.js";
import { ensureWebUiServer, stopWebUiServer } from "./devServer.js";
import type { HarnessSession } from "./page.js";
import {
  pageFill,
  pagePress,
  pageWaitForSelector,
  pageWaitForText,
} from "./page.js";

export type { HarnessSession };

let session: HarnessSession | null = null;
let staticBrowser: import("playwright").Browser | null = null;
let staticContext: import("playwright").BrowserContext | null = null;
let processManager: TauriProcessManager | null = null;
let pluginClient: PluginClient | null = null;
let desktopRepoRoot: string | null = null;

export function getSession(): HarnessSession {
  if (!session) throw new Error("No active session — call ui_open first");
  return session;
}

export async function openSession(
  cfg: HarnessConfig,
  launch: { headless?: boolean; mode?: "desktop" | "static" } = {}
): Promise<HarnessSession> {
  if (session) await closeSession();
  const mode = launch.mode ?? cfg.mode;
  if (mode === "desktop") {
    return openDesktopSession(cfg);
  }
  return openStaticSession(cfg, launch.headless !== false);
}

async function openStaticSession(cfg: HarnessConfig, headless: boolean): Promise<HarnessSession> {
  const { chromium } = await import("playwright");
  staticBrowser = await chromium.launch({ headless });
  staticContext = await staticBrowser.newContext({ viewport: cfg.viewport });
  const page = await staticContext.newPage();
  await page.clock.install({ time: new Date("2026-01-01T00:00:00Z") });
  await page.goto(cfg.baseUrl, { waitUntil: "domcontentloaded" });
  session = { mode: "static", page, motionFrames: [], recording: false };
  return session;
}

async function openDesktopSession(cfg: HarnessConfig): Promise<HarnessSession> {
  if (process.platform !== "darwin") {
    throw new Error(
      'Desktop mode requires macOS. Set launch.mode = "static" in rex-ui-harness.toml.'
    );
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

export async function closeSession(): Promise<void> {
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

export async function gotoScenario(scenario: string): Promise<void> {
  const s = getSession();
  if (s.mode === "static") {
    await (s.page as Page).evaluate((name) => {
      const probe = (window as unknown as { __rexProbe?: { gotoScenario: (n: string) => string } })
        .__rexProbe;
      if (!probe) throw new Error("Probe harness not loaded");
      return probe.gotoScenario(name);
    }, scenario);
    await (s.page as Page).waitForTimeout(50);
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
    case "approval_required":
      throw new Error(
        "approval_required needs agent.approvals_enabled in probe config — use static fixture"
      );
    case "error":
    case "history-fetch":
      throw new Error(`${scenario} scenario is static-fixture only`);
    default:
      throw new Error(`Unknown scenario: ${scenario}`);
  }
}
