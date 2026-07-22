#!/usr/bin/env node
import { findRepoRoot, loadConfig } from "./config.js";
import { ciede2000, parseCssColor } from "./color.js";
import { closeSession, gotoScenario, openSession } from "./session.js";
import { pageCssTokenAssert, pageClick, pageCanvasHash, pageCanvasMeta, pageEvaluate, pageFill, pageFocus, pageLayout, pagePress, pageScreenshot, pageWaitForSelector, pageWaitForText, } from "./page.js";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const sharp = require("sharp");
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
/** Fails when a decorative canvas buries the shell (DOM opacity alone is insufficient). */
async function assertComposerReachable() {
    const session = await import("./session.js").then((m) => m.getSession());
    const detail = await pageEvaluate(session, () => {
        const shell = document.querySelector('[data-testid="shell"]');
        const composer = document.querySelector('[data-testid="composer-input"]');
        if (!(shell instanceof HTMLElement) || !(composer instanceof HTMLElement)) {
            return { reachable: false, hitTag: null, hitId: null, reason: "missing" };
        }
        const rect = composer.getBoundingClientRect();
        const hit = document.elementFromPoint(rect.left + 20, rect.top + 10);
        const reachable = Boolean(hit && shell.contains(hit));
        return {
            reachable,
            hitTag: hit?.tagName ?? null,
            hitId: hit instanceof HTMLElement ? hit.id || hit.getAttribute("data-testid") : null,
            reason: reachable ? "ok" : "buried",
        };
    }, null);
    return {
        step: "assert_composer_reachable",
        pass: Boolean(detail.reachable),
        detail,
    };
}
/**
 * Pixel gate: background-only paint is dark and low-variance; chrome has bright text/controls.
 * Catches compositor blanks that leave DOM opacity at 1 while chrome is buried.
 */
async function assertShellChromePainted() {
    const session = await import("./session.js").then((m) => m.getSession());
    const png = await pageScreenshot(session);
    const { data, info } = await sharp(png)
        .ensureAlpha()
        .raw()
        .toBuffer({ resolveWithObject: true });
    const w = info.width;
    const h = info.height;
    const channels = info.channels;
    let bright = 0;
    let total = 0;
    let maxLum = 0;
    // Sparse white title/controls on a dark shell — sample a grid, not single voids.
    for (let y = 0; y < h; y += 8) {
        for (let x = 0; x < w; x += 8) {
            const i = (y * w + x) * channels;
            const lum = 0.2126 * (data[i] ?? 0) + 0.7152 * (data[i + 1] ?? 0) + 0.0722 * (data[i + 2] ?? 0);
            total += 1;
            if (lum > 60)
                bright += 1;
            if (lum > maxLum)
                maxLum = lum;
        }
    }
    const brightFrac = total > 0 ? bright / total : 0;
    const pass = maxLum >= 120 && brightFrac >= 0.004;
    return {
        step: "assert_shell_chrome_painted",
        pass,
        detail: { width: w, height: h, maxLum, brightFrac, bright, total },
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
async function assertCanvasTier(expected) {
    const session = await import("./session.js").then((m) => m.getSession());
    const tier = await pageEvaluate(session, () => document.querySelector('[data-testid="ambient"]')?.getAttribute("data-motion-tier") ?? "", null);
    const pass = tier === expected;
    return { step: `assert_canvas_tier ${expected}`, pass, detail: { tier } };
}
async function assertParticleRegl() {
    const session = await import("./session.js").then((m) => m.getSession());
    const meta = await pageCanvasMeta(session, '[data-testid="particles"]');
    const pass = (meta.renderer === "regl" && meta.webgl) || meta.renderer === "canvas2d";
    return {
        step: "assert_particle_regl",
        pass,
        detail: meta,
    };
}
async function assertModalParticleRegl() {
    const session = await import("./session.js").then((m) => m.getSession());
    const meta = await pageCanvasMeta(session, '[data-testid="modal-particles"]');
    const pass = (meta.renderer === "regl" && meta.webgl) || meta.renderer === "canvas2d";
    return {
        step: "assert_modal_particle_regl",
        pass,
        detail: meta,
    };
}
async function assertParticleCanvasTier(expected) {
    const session = await import("./session.js").then((m) => m.getSession());
    const meta = await pageCanvasMeta(session, '[data-testid="particles"]');
    const pass = meta.motionTier === expected;
    return {
        step: `assert_particle_canvas_tier ${expected}`,
        pass,
        detail: meta,
    };
}
async function assertCanvasAnimating(selector) {
    const session = await import("./session.js").then((m) => m.getSession());
    const hash1 = await pageCanvasHash(session, selector);
    await new Promise((resolve) => setTimeout(resolve, 120));
    const hash2 = await pageCanvasHash(session, selector);
    const pass = hash1.length > 0 && hash2.length > 0 && hash1 !== hash2;
    return {
        step: `assert_canvas_animating ${selector}`,
        pass,
        detail: { hash1_preview: hash1.slice(0, 32), hash2_preview: hash2.slice(0, 32) },
    };
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
    results.push(await assertComposerReachable());
    results.push(await assertShellChromePainted());
    // Past connectFade decay (~400ms) — shell must still be hit-testable and painted.
    await new Promise((resolve) => setTimeout(resolve, 500));
    results.push(await assertComposerReachable());
    results.push(await assertShellChromePainted());
    await pageWaitForSelector(session, '[data-testid="composer-input"]', 60_000);
    await pageFocus(session, '[data-testid="composer-input"]');
    await pageFill(session, '[data-testid="composer-input"]', "hello");
    await pagePress(session, "Enter");
    results.push({ step: "send hello", pass: true });
    await pageWaitForSelector(session, "#status-dot.working", 30_000);
    results.push(await assertMotion("#status-dot"));
    results.push(await assertLayout('[data-testid="status-orbit"]', "block"));
    await pageWaitForSelector(session, '[data-testid="timeline-hairline"]', 10_000);
    results.push(await assertLayout('[data-testid="timeline-hairline"]', "block"));
    results.push(await assertLayout('[data-testid="transcript-hairline"]', "block"));
    results.push(await assertLayout('[data-testid="edge-glow"]', "block"));
    results.push(await assertCanvasTier("cinematic"));
    results.push(await assertParticleRegl());
    results.push(await assertParticleCanvasTier("cinematic"));
    results.push(await assertCanvasAnimating('[data-testid="ambient"]'));
    results.push(await assertCanvasAnimating('[data-testid="particles"]'));
    results.push(await assertComposerReachable());
    results.push(await assertShellChromePainted());
    await pageWaitForText(session, "mock: hello", 60_000);
    results.push({ step: "wait transcript mock hello", pass: true });
    await waitForStatusLabel(session, "Ready", 60_000);
    results.push({ step: "wait status ready after hello", pass: true });
    results.push(await assertComposerReachable());
    results.push(await assertToken("#status-dot", "--rex-status-success", "background-color"));
    await pageEvaluate(session, () => {
        window.resizeTo(600, 800);
        window.dispatchEvent(new Event("resize"));
    }, null);
    results.push(await assertLayout('[data-testid="shell"]', "grid"));
    results.push(await assertComposerReachable());
    await pagePress(session, "Meta+k");
    await pageWaitForSelector(session, '[data-testid="command-palette"]', 10_000);
    results.push({ step: "open command palette", pass: true });
    await pagePress(session, "Escape");
    await gotoScenario("streaming");
    results.push({ step: "goto streaming", pass: true });
    await pageWaitForSelector(session, "#status-dot.working", 30_000);
    results.push(await assertComposerReachable());
    await gotoScenario("approval_required");
    results.push({ step: "goto approval_required", pass: true });
    await pageWaitForSelector(session, '[data-testid="modal"]', 30_000);
    results.push(await assertModalParticleRegl());
    results.push(await assertCanvasAnimating('[data-testid="modal-particles"]'));
    await pageClick(session, '[data-testid="approval-approve"]');
    results.push({ step: "approve modal", pass: true });
    await gotoScenario("error");
    results.push({ step: "goto error", pass: true });
    await pageClick(session, '[data-testid="error-banner-dismiss"]');
    results.push({ step: "dismiss error banner", pass: true });
    results.push(await assertComposerReachable());
    await closeSession();
    results.push({ step: "close", pass: true });
    return results;
}
async function main() {
    const { mode, socket: _socket } = parseArgs();
    const repoRoot = findRepoRoot(process.cwd());
    const base = loadConfig(repoRoot);
    const cfg = {
        ...base,
        mode,
        repoRoot,
    };
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
