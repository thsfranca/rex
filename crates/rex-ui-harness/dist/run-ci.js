#!/usr/bin/env node
import { findRepoRoot, loadConfig } from "./config.js";
import { ciede2000, parseCssColor } from "./color.js";
import { closeSession, gotoScenario, openSession } from "./session.js";
import { pageCssTokenAssert, pageClick, pageEvaluate, pageFill, pageFocus, pageLayout, pagePress, pageWaitForSelector, pageWaitForText, } from "./page.js";
async function pageEvaluateStatusLabel(session) {
    return pageEvaluate(session, () => document.querySelector("#status-label")?.textContent?.trim() ?? "", null);
}
async function waitForStatusLabel(session, label, timeoutMs) {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
        const current = await pageEvaluateStatusLabel(session);
        if (current === label)
            return;
        await new Promise((resolve) => setTimeout(resolve, 200));
    }
    const current = await pageEvaluateStatusLabel(session);
    throw new Error(`Timed out waiting for status label ${label}; last=${current}`);
}
function emitHarnessFailures(mode, failed) {
    for (const step of failed) {
        console.error(`UI_HARNESS_FAIL step=${JSON.stringify(step.step)}`);
        if (step.detail !== undefined) {
            console.error(`UI_HARNESS_DETAIL ${JSON.stringify(step.detail)}`);
        }
    }
    console.error(`CI_SIGNAL code=UI_FAIL stage=TestExecution result=failure hint=${failed.length} harness step(s) failed in ${mode} mode`);
}
function emitHarnessError(err) {
    const message = err instanceof Error ? err.message : String(err);
    console.error(`UI_HARNESS_ERROR message=${JSON.stringify(message)}`);
    if (err instanceof Error && err.stack) {
        console.error(err.stack.split("\n").slice(0, 8).join("\n"));
    }
    console.error("CI_SIGNAL code=UI_FAIL stage=TestExecution result=failure hint=harness threw before reporting steps");
}
function parseArgs() {
    const args = process.argv.slice(2);
    let mode;
    let socket;
    for (let i = 0; i < args.length; i++) {
        const arg = args[i];
        if (arg === "--mode" && args[i + 1]) {
            const value = args[++i];
            if (value !== "desktop" && value !== "build") {
                throw new Error(`Invalid --mode ${value}; expected desktop or build`);
            }
            mode = value;
        }
        else if (arg === "--socket" && args[i + 1]) {
            socket = args[++i];
        }
        else if (arg === "--help" || arg === "-h") {
            console.log("Usage: run-ci.js --mode desktop|build [--socket PATH]");
            process.exit(0);
        }
    }
    if (!mode) {
        throw new Error("Missing required --mode desktop|build");
    }
    return { mode, socket };
}
async function assertToken(selector, token, property, maxDelta = 2.3) {
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
async function assertLayout(selector, display) {
    const session = await import("./session.js").then((m) => m.getSession());
    const layout = await pageLayout(session, selector);
    const pass = layout.display === display;
    return {
        step: `assert_layout ${selector} display=${display}`,
        pass,
        detail: { layout },
    };
}
async function assertMotion(region) {
    const session = await import("./session.js").then((m) => m.getSession());
    const pass = await pageEvaluate(session, (sel) => {
        const el = document.querySelector(sel);
        if (!(el instanceof HTMLElement))
            return false;
        return el.classList.contains("working") && el.dataset.motionTier === "ambient";
    }, region);
    return { step: `assert_motion ${region}`, pass: Boolean(pass) };
}
async function runBuildSuite() {
    return [{ step: "build-only gate", pass: true }];
}
async function runDesktopSuite(cfg) {
    const results = [];
    await openSession(cfg, { mode: "desktop" });
    results.push({ step: "open desktop", pass: true });
    const session = await import("./session.js").then((m) => m.getSession());
    results.push(await assertLayout('[data-testid="shell"]', "grid"));
    await pageWaitForSelector(session, '[data-testid="composer-input"]', 60_000);
    await pageFocus(session, '[data-testid="composer-input"]');
    await pageFill(session, '[data-testid="composer-input"]', "hello");
    await pagePress(session, "Enter");
    results.push({ step: "send hello", pass: true });
    await pageWaitForSelector(session, "#status-dot.working", 30_000);
    results.push(await assertMotion("#status-dot"));
    await pageWaitForText(session, "mock: hello", 60_000);
    results.push({ step: "wait transcript mock hello", pass: true });
    await waitForStatusLabel(session, "Ready", 60_000);
    results.push({ step: "wait status ready after hello", pass: true });
    results.push(await assertToken("#status-dot", "--rex-status-success", "background-color"));
    await gotoScenario("approval_required");
    results.push({ step: "goto approval_required", pass: true });
    await pageWaitForSelector(session, '[data-testid="modal"]', 30_000);
    await pageClick(session, '[data-testid="approval-approve"]');
    results.push({ step: "approve modal", pass: true });
    await closeSession();
    results.push({ step: "close", pass: true });
    return results;
}
async function main() {
    const { mode, socket } = parseArgs();
    const repoRoot = findRepoRoot(process.cwd());
    const base = loadConfig(repoRoot);
    const cfg = {
        ...base,
        mode,
        repoRoot,
        ...(socket ? { desktopSocket: socket } : {}),
    };
    if (socket) {
        process.env.TAURI_PLAYWRIGHT_SOCKET = socket;
    }
    if (mode === "build") {
        console.log(JSON.stringify({ mode, pass: true, steps: await runBuildSuite() }, null, 2));
        return;
    }
    try {
        const results = await runDesktopSuite(cfg);
        const failed = results.filter((r) => !r.pass);
        if (failed.length > 0) {
            emitHarnessFailures(mode, failed);
        }
        console.log(JSON.stringify({ mode, pass: failed.length === 0, steps: results }, null, 2));
        if (failed.length > 0) {
            process.exit(1);
        }
    }
    finally {
        await closeSession();
    }
}
main().catch((err) => {
    emitHarnessError(err);
    process.exit(1);
});
