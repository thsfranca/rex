import type { Page } from "playwright";

export interface HarnessSession {
  mode: "desktop";
  page: Page;
  motionFrames: Buffer[];
  recording: boolean;
}

function page(session: HarnessSession): Page {
  return session.page;
}

export async function pageEvaluate<T>(
  session: HarnessSession,
  fn: (arg: unknown) => T,
  arg: unknown
): Promise<T> {
  return page(session).evaluate(fn, arg);
}

export async function pageClick(session: HarnessSession, selector: string): Promise<void> {
  await page(session).click(selector);
}

export async function pageFocus(session: HarnessSession, selector: string): Promise<void> {
  await page(session).focus(selector);
}

export async function pageType(session: HarnessSession, text: string): Promise<void> {
  await page(session).keyboard.type(text);
}

export async function pagePress(session: HarnessSession, key: string): Promise<void> {
  await page(session).keyboard.press(key);
}

const WAIT_CHUNK_MS = 25_000;

async function waitWithChunks(
  totalMs: number,
  run: (chunkMs: number) => Promise<void>
): Promise<void> {
  const deadline = Date.now() + totalMs;
  let lastError: unknown;
  while (Date.now() < deadline) {
    const remaining = deadline - Date.now();
    const chunk = Math.min(WAIT_CHUNK_MS, remaining);
    try {
      await run(chunk);
      return;
    } catch (err) {
      lastError = err;
      if (Date.now() >= deadline) {
        break;
      }
    }
  }
  throw lastError instanceof Error
    ? lastError
    : new Error(`Timed out after ${totalMs}ms`);
}

export async function pageWaitForSelector(
  session: HarnessSession,
  selector: string,
  timeout?: number
): Promise<void> {
  const total = timeout ?? 60_000;
  await waitWithChunks(total, (chunk) =>
    page(session).waitForSelector(selector, { timeout: chunk }).then(() => undefined)
  );
}

export async function pageWaitForText(
  session: HarnessSession,
  text: string,
  timeout?: number
): Promise<void> {
  const total = timeout ?? 60_000;
  await waitWithChunks(total, (chunk) =>
    page(session).getByText(text).waitFor({ timeout: chunk }).then(() => undefined)
  );
}

export async function pageSnapshotTree(session: HarnessSession): Promise<string> {
  return page(session).evaluate(() => {
    const shell = document.querySelector('[data-testid=shell]');
    return shell ? (shell as HTMLElement).innerText : document.body.innerText;
  });
}

export async function pageScreenshot(session: HarnessSession): Promise<Buffer> {
  return page(session).screenshot();
}

export async function pageLocatorScreenshot(
  session: HarnessSession,
  selector: string
): Promise<Buffer> {
  return page(session).locator(selector).screenshot();
}

export async function pageEmulateReducedMotion(
  session: HarnessSession,
  enabled: boolean
): Promise<void> {
  await page(session).evaluate((on) => {
    document.documentElement.style.setProperty(
      "animation-duration",
      on ? "0.001ms" : "",
      "important"
    );
  }, enabled);
}

export async function pageClockStep(_session: HarnessSession, durationMs: number): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, durationMs));
}

export async function pageLayout(session: HarnessSession, selector: string) {
  return pageEvaluate(
    session,
    (sel) => {
      const el = document.querySelector(sel as string);
      if (!el) throw new Error(`Missing: ${sel}`);
      const s = getComputedStyle(el);
      return {
        display: s.display,
        flexDirection: s.flexDirection,
        gridTemplateColumns: s.gridTemplateColumns,
      };
    },
    selector
  );
}

export async function pageCssTokenAssert(
  session: HarnessSession,
  selector: string,
  token: string,
  property: string
): Promise<{ actual: string; expected: string }> {
  const actual = await pageEvaluate(
    session,
    (arg) => {
      const { sel, prop } = arg as { sel: string; prop: string };
      const el = document.querySelector(sel);
      if (!el) throw new Error(`Missing selector: ${sel}`);
      const style = getComputedStyle(el);
      if (prop === "color") return style.color;
      if (prop === "background-color") return style.backgroundColor;
      if (prop === "border-color") return style.borderColor;
      return style.color;
    },
    { sel: selector, prop: property }
  );
  const expected = await pageEvaluate(
    session,
    (t) => getComputedStyle(document.documentElement).getPropertyValue(t as string).trim(),
    token
  );
  return { actual: String(actual), expected: String(expected) };
}

export async function pageFill(session: HarnessSession, selector: string, text: string): Promise<void> {
  await page(session).fill(selector, text);
}

export async function pageCanvasHash(session: HarnessSession, selector: string): Promise<string> {
  return pageEvaluate(
    session,
    (sel) => {
      const c = document.querySelector(sel as string) as HTMLCanvasElement | null;
      if (!c) return "";
      const ctx2d = c.getContext("2d");
      if (ctx2d) {
        return ctx2d.getImageData(0, 0, c.width, c.height).data.join(",").slice(0, 500);
      }
      const gl = c.getContext("webgl2") ?? c.getContext("webgl");
      if (!gl) return "";
      const chunks: number[] = [];
      for (let i = 0; i < 8; i += 1) {
        const sample = new Uint8Array(4);
        const x = Math.max(0, Math.floor((c.width * (i + 1)) / 9));
        const y = Math.max(0, Math.floor((c.height * (i + 1)) / 9));
        gl.readPixels(x, y, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, sample);
        chunks.push(...sample);
      }
      return chunks.join(",");
    },
    selector
  );
}

export async function pageCanvasMeta(
  session: HarnessSession,
  selector: string
): Promise<{ renderer: string; motionTier: string; webgl: boolean }> {
  return pageEvaluate(
    session,
    (sel) => {
      const el = document.querySelector(sel as string);
      if (!(el instanceof HTMLCanvasElement)) {
        return { renderer: "", motionTier: "", webgl: false };
      }
      const webgl = Boolean(el.getContext("webgl2") ?? el.getContext("webgl"));
      return {
        renderer: el.dataset.renderer ?? "",
        motionTier: el.dataset.motionTier ?? "",
        webgl,
      };
    },
    selector
  );
}

export async function readObservabilitySnapshot(
  session: HarnessSession
): Promise<Record<string, unknown>> {
  return pageEvaluate(
    session,
    () => {
      const el = document.querySelector('[data-testid="ui-observability"]');
      const globalSnapshot = (
        window as Window & { __REX_UI_OBSERVABILITY__?: Record<string, unknown> }
      ).__REX_UI_OBSERVABILITY__;
      if (globalSnapshot) return globalSnapshot;
      if (!el) return { error: "missing ui-observability node" };
      return {
        phase: el.getAttribute("data-phase"),
        status: el.getAttribute("data-status"),
        pendingApproval: el.getAttribute("data-pending-approval"),
        error: el.getAttribute("data-error"),
        submitError: el.getAttribute("data-submit-error"),
        sessionId: el.getAttribute("data-session-id"),
        composerBusy: el.getAttribute("data-composer-busy"),
        streamEvents: el.getAttribute("data-stream-events"),
        summary: el.textContent?.trim() ?? "",
      };
    },
    null
  );
}
