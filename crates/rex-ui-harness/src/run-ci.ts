#!/usr/bin/env node
import path from "node:path";
import type { Page } from "playwright";
import { findRepoRoot, loadConfig, type HarnessConfig, type HarnessMode } from "./config.js";
import { ciede2000, parseCssColor } from "./color.js";
import { closeSession, gotoScenario, openSession } from "./session.js";
import {
  pageCssTokenAssert,
  pageEvaluate,
  pageLayout,
  pageLocatorScreenshot,
  pageFill,
  pagePress,
  pageWaitForSelector,
  pageWaitForText,
} from "./page.js";

interface StepResult {
  step: string;
  pass: boolean;
  detail?: unknown;
}

function parseArgs(): { mode: HarnessMode; socket?: string } {
  const args = process.argv.slice(2);
  let mode: HarnessMode | undefined;
  let socket: string | undefined;
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === "--mode" && args[i + 1]) {
      const value = args[++i];
      if (value !== "static" && value !== "desktop") {
        throw new Error(`Invalid --mode ${value}; expected static or desktop`);
      }
      mode = value;
    } else if (arg === "--socket" && args[i + 1]) {
      socket = args[++i];
    } else if (arg === "--help" || arg === "-h") {
      console.log("Usage: run-ci.js --mode static|desktop [--socket PATH]");
      process.exit(0);
    }
  }
  if (!mode) {
    throw new Error("Missing required --mode static|desktop");
  }
  return { mode, socket };
}

async function assertToken(
  selector: string,
  token: string,
  property: "color" | "background-color" | "border-color",
  maxDelta = 2.3
): Promise<StepResult> {
  const session = await import("./session.js").then((m) => m.getSession());
  const { actual, expected } = await pageCssTokenAssert(session, selector, token, property);
  const delta = ciede2000(parseCssColor(actual), parseCssColor(expected));
  const pass = delta <= maxDelta;
  return {
    step: `assert_token ${selector} ${token}`,
    pass,
    detail: { delta_e: delta, actual, expected, max_delta_e: maxDelta },
  };
}

async function assertLayout(
  selector: string,
  display: string
): Promise<StepResult> {
  const session = await import("./session.js").then((m) => m.getSession());
  const layout = await pageLayout(session, selector);
  const pass = layout.display === display;
  return {
    step: `assert_layout ${selector} display=${display}`,
    pass,
    detail: { layout },
  };
}

async function assertMotion(region: string): Promise<StepResult> {
  const session = await import("./session.js").then((m) => m.getSession());
  if (session.mode === "static") {
    const page = session.page as Page;
    const before = await pageLocatorScreenshot(session, region);
    await page.clock.fastForward(150);
    const mid = await pageLocatorScreenshot(session, region);
    await page.clock.fastForward(350);
    const after = await pageLocatorScreenshot(session, region);
    const pass = !before.equals(mid) || !mid.equals(after);
    return { step: `assert_motion ${region}`, pass };
  }

  const pass = await pageEvaluate(
    session,
    (sel) => {
      const el = document.querySelector(sel as string);
      if (!(el instanceof HTMLElement)) return false;
      return el.classList.contains("working") && el.dataset.motionTier === "ambient";
    },
    region
  );
  return { step: `assert_motion ${region}`, pass: Boolean(pass) };
}

async function runStaticSuite(cfg: HarnessConfig): Promise<StepResult[]> {
  const results: StepResult[] = [];
  await openSession(cfg, { mode: "static", headless: true });
  results.push({ step: "open static", pass: true });

  await gotoScenario("idle");
  results.push({ step: "goto idle", pass: true });

  results.push(
    await assertToken("#status-dot", "--rex-status-success", "background-color")
  );
  results.push(await assertLayout('[data-testid="shell"]', "grid"));

  await gotoScenario("streaming");
  results.push({ step: "goto streaming", pass: true });
  results.push(await assertMotion("#status-dot"));

  await closeSession();
  results.push({ step: "close", pass: true });
  return results;
}

async function runDesktopSuite(cfg: HarnessConfig): Promise<StepResult[]> {
  const results: StepResult[] = [];
  await openSession(cfg, { mode: "desktop" });
  results.push({ step: "open desktop", pass: true });

  const session = await import("./session.js").then((m) => m.getSession());
  results.push(await assertLayout('[data-testid="shell"]', "grid"));

  await pageWaitForSelector(session, '[data-testid="composer-input"]', 60_000);
  await pageFill(session, '[data-testid="composer-input"]', "hello");
  await pagePress(session, "Enter");
  results.push({ step: "send hello", pass: true });

  await pageWaitForSelector(session, "#status-dot.working", 30_000).catch(() => {});
  results.push(await assertMotion("#status-dot"));

  await pageWaitForText(session, "hello", 60_000);
  results.push({ step: "wait transcript hello", pass: true });

  await pageWaitForText(session, "Ready", 60_000);

  results.push(
    await assertToken("#status-dot", "--rex-status-success", "background-color")
  );

  await closeSession();
  results.push({ step: "close", pass: true });
  return results;
}

async function main(): Promise<void> {
  const { mode, socket } = parseArgs();
  const repoRoot = findRepoRoot(process.cwd());
  const base = loadConfig(repoRoot);
  const staticIndex = path.join(repoRoot, "fixtures/ui_probe/static/index.html");
  const cfg: HarnessConfig = {
    ...base,
    mode,
    repoRoot,
    baseUrl: `file://${staticIndex}`,
    ...(socket ? { desktopSocket: socket } : {}),
  };
  if (socket) {
    process.env.TAURI_PLAYWRIGHT_SOCKET = socket;
  }

  try {
    const results = mode === "static" ? await runStaticSuite(cfg) : await runDesktopSuite(cfg);
    const failed = results.filter((r) => !r.pass);

    console.log(JSON.stringify({ mode, pass: failed.length === 0, steps: results }, null, 2));

    if (failed.length > 0) {
      process.exit(1);
    }
  } finally {
    await closeSession();
  }
}

main().catch((err) => {
  console.error(err instanceof Error ? err.message : String(err));
  process.exit(1);
});
