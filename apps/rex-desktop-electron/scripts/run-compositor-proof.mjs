#!/usr/bin/env node
/**
 * Compositor proof: chrome + fullscreen WebGL co-visible for ≥5s.
 * Samples at t=0,1,3,5s — luminance (not background-only) + hit-test.
 *
 * Usage:
 *   node scripts/run-compositor-proof.mjs
 *   node scripts/run-compositor-proof.mjs --bury --expect-fail
 */
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { PNG } from "pngjs";

const require = createRequire(import.meta.url);
const { _electron: electron } = require("playwright");

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const APP_ROOT = path.join(__dirname, "..");
const SAMPLE_MS = [0, 1000, 3000, 5000];

function parseArgs(argv) {
  const flags = new Set(argv.slice(2));
  return {
    bury: flags.has("--bury"),
    expectFail: flags.has("--expect-fail"),
  };
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function meanRgb(png) {
  let r = 0;
  let g = 0;
  let b = 0;
  let n = 0;
  for (let i = 0; i < png.data.length; i += 4) {
    r += png.data[i];
    g += png.data[i + 1];
    b += png.data[i + 2];
    n += 1;
  }
  if (n === 0) {
    return { r: 0, g: 0, b: 0, n: 0 };
  }
  return { r: r / n, g: g / n, b: b / n, n };
}

function fractionNearWebglClear(png) {
  // WebGL clear ≈ rgb(5, 199, 224) with small pulse on G.
  let near = 0;
  let n = 0;
  for (let i = 0; i < png.data.length; i += 4) {
    const r = png.data[i];
    const g = png.data[i + 1];
    const b = png.data[i + 2];
    const dr = Math.abs(r - 5);
    const dg = Math.abs(g - 199);
    const db = Math.abs(b - 224);
    if (dr < 40 && dg < 50 && db < 40) {
      near += 1;
    }
    n += 1;
  }
  return n === 0 ? 1 : near / n;
}

async function sampleChrome(page, selector) {
  const buf = await page.locator(selector).screenshot({ type: "png" });
  const png = PNG.sync.read(buf);
  const mean = meanRgb(png);
  const webglFrac = fractionNearWebglClear(png);
  return { mean, webglFrac, pixels: mean.n };
}

async function hitTest(page, selector) {
  return page.evaluate((sel) => {
    const el = document.querySelector(sel);
    if (!el) {
      return { ok: false, reason: "missing" };
    }
    const rect = el.getBoundingClientRect();
    const x = rect.left + rect.width / 2;
    const y = rect.top + rect.height / 2;
    const top = document.elementFromPoint(x, y);
    if (!top) {
      return { ok: false, reason: "null-top", x, y };
    }
    const hit = top.closest(sel) != null;
    return {
      ok: hit,
      reason: hit ? "hit" : "miss",
      top: top.id || top.className || top.tagName,
      x,
      y,
    };
  }, selector);
}

async function assertSample(page, label) {
  const errors = [];

  const webglReady = await page.locator("body[data-webgl-ready='1']").count();
  if (webglReady !== 1) {
    errors.push(`${label}: WebGL not ready`);
  }

  const frames = Number(
    (await page.locator("body").getAttribute("data-webgl-frames")) || "0",
  );
  if (frames < 1) {
    errors.push(`${label}: WebGL frames=${frames}`);
  }

  for (const sel of ["#proof-header", "#proof-composer"]) {
    const hit = await hitTest(page, sel);
    if (!hit.ok) {
      errors.push(
        `${label}: hit-test ${sel} failed (${hit.reason}, top=${hit.top})`,
      );
    }

    const chrome = await sampleChrome(page, sel);
    if (chrome.webglFrac > 0.85) {
      errors.push(
        `${label}: ${sel} luminance is background-only (webglFrac=${chrome.webglFrac.toFixed(3)})`,
      );
    }
    // Chrome panel is dark; mean luminance should not look like cyan WebGL.
    const lum = 0.2126 * chrome.mean.r + 0.7152 * chrome.mean.g + 0.0722 * chrome.mean.b;
    if (lum > 160 && chrome.mean.g > 150 && chrome.mean.b > 150) {
      errors.push(
        `${label}: ${sel} mean looks like WebGL clear (lum=${lum.toFixed(1)})`,
      );
    }
  }

  return errors;
}

async function main() {
  const { bury, expectFail } = parseArgs(process.argv);
  const electronPath = require("electron");

  const args = [APP_ROOT, "--", "--proof"];
  if (bury) {
    args.push("--bury");
  }

  console.log(
    `compositor-proof: launch electron bury=${bury} expectFail=${expectFail}`,
  );

  const app = await electron.launch({
    executablePath: electronPath,
    args,
    env: {
      ...process.env,
      REX_ELECTRON_PROOF: "1",
      ...(bury ? { REX_COMPOSITOR_PROOF_BURY: "1" } : {}),
    },
  });

  try {
    const page = await app.firstWindow();
    await page.waitForSelector("body[data-webgl-ready='1']", {
      timeout: 15_000,
    });

    const t0 = Date.now();
    const allErrors = [];

    for (const delay of SAMPLE_MS) {
      const elapsed = Date.now() - t0;
      const wait = Math.max(0, delay - elapsed);
      if (wait > 0) {
        await sleep(wait);
      }
      const label = `t=${delay}ms`;
      const errors = await assertSample(page, label);
      if (errors.length) {
        allErrors.push(...errors);
        console.error(`FAIL ${label}`);
        for (const e of errors) {
          console.error(`  - ${e}`);
        }
      } else {
        console.log(`PASS ${label}`);
      }
    }

    const failed = allErrors.length > 0;
    if (expectFail) {
      if (failed) {
        console.log(
          `compositor-proof: bury regression detected as expected (${allErrors.length} errors)`,
        );
        process.exitCode = 0;
        return;
      }
      console.error(
        "compositor-proof: expected bury failure but all samples passed",
      );
      process.exitCode = 1;
      return;
    }

    if (failed) {
      console.error(
        `compositor-proof: FAILED with ${allErrors.length} error(s)`,
      );
      process.exitCode = 1;
      return;
    }

    console.log("compositor-proof: PASSED (chrome + WebGL co-visible ≥5s)");
    process.exitCode = 0;
  } finally {
    await app.close();
  }
}

main().catch((err) => {
  console.error("compositor-proof: fatal", err);
  process.exitCode = 1;
});
