#!/usr/bin/env node
/**
 * Compositor proof against the sole product UI (apps/rex-web):
 * chrome + fullscreen ambient WebGL co-visible for ≥5s.
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
const WEB_DIST = path.resolve(APP_ROOT, "..", "rex-web", "dist", "index.html");
const SAMPLE_MS = [0, 1000, 3000, 5000];
const CHROME_SELECTORS = ['[data-testid="app-header"]', '[data-testid="composer"]'];

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

function brightFraction(png) {
  let bright = 0;
  let n = 0;
  for (let i = 0; i < png.data.length; i += 4) {
    const lum =
      0.2126 * png.data[i] +
      0.7152 * png.data[i + 1] +
      0.0722 * png.data[i + 2];
    if (lum > 60) bright += 1;
    n += 1;
  }
  return n === 0 ? 0 : bright / n;
}

async function sampleChrome(page, selector) {
  const buf = await page.locator(selector).screenshot({ type: "png" });
  const png = PNG.sync.read(buf);
  return { mean: meanRgb(png), brightFrac: brightFraction(png), pixels: meanRgb(png).n };
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

  const ambient = await page.locator('[data-testid="ambient"]').count();
  if (ambient !== 1) {
    errors.push(`${label}: ambient canvas missing`);
  }

  const tier = await page
    .locator('[data-testid="ambient"]')
    .getAttribute("data-motion-tier");
  if (tier !== "cinematic") {
    errors.push(`${label}: ambient tier=${tier} (expected cinematic)`);
  }

  for (const sel of CHROME_SELECTORS) {
    const hit = await hitTest(page, sel);
    if (!hit.ok) {
      errors.push(
        `${label}: hit-test ${sel} failed (${hit.reason}, top=${hit.top})`,
      );
    }

    const chrome = await sampleChrome(page, sel);
    // Chrome must show painted text/controls, not a flat buried background.
    if (chrome.brightFrac < 0.002) {
      errors.push(
        `${label}: ${sel} luminance is background-only (brightFrac=${chrome.brightFrac.toFixed(4)})`,
      );
    }
  }

  return errors;
}

async function main() {
  const { bury, expectFail } = parseArgs(process.argv);
  if (!require("node:fs").existsSync(WEB_DIST)) {
    console.error(
      `compositor-proof: missing ${WEB_DIST}; build apps/rex-web first`,
    );
    process.exitCode = 1;
    return;
  }

  const electronPath = require("electron");
  const args = [APP_ROOT, "--", "--compositor-proof"];

  console.log(
    `compositor-proof: launch apps/rex-web bury=${bury} expectFail=${expectFail}`,
  );

  const app = await electron.launch({
    executablePath: electronPath,
    args,
    env: {
      ...process.env,
      REX_COMPOSITOR_PROOF: "1",
      REX_DESKTOP_HOST: "electron",
    },
  });

  try {
    const page = await app.firstWindow();
    await page.waitForSelector('[data-testid="shell"]', { timeout: 30_000 });
    await page.waitForSelector('[data-testid="app-header"]', { timeout: 15_000 });
    await page.waitForSelector('[data-testid="composer"]', { timeout: 15_000 });
    await page.waitForSelector(
      '[data-testid="ambient"][data-motion-tier="cinematic"]',
      { timeout: 15_000 },
    );

    if (bury) {
      await page.addStyleTag({
        content: `
          canvas#ambient {
            z-index: 9999 !important;
            pointer-events: auto !important;
            opacity: 1 !important;
          }
        `,
      });
    }

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

    console.log(
      "compositor-proof: PASSED (apps/rex-web chrome + ambient WebGL co-visible ≥5s)",
    );
    process.exitCode = 0;
  } finally {
    await app.close();
  }
}

main().catch((err) => {
  console.error("compositor-proof: fatal", err);
  process.exitCode = 1;
});
