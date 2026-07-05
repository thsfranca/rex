function page(session) {
    return session.page;
}
export async function pageEvaluate(session, fn, arg) {
    const script = `(${fn.toString()})(${JSON.stringify(arg)})`;
    return page(session).evaluate(script);
}
export async function pageClick(session, selector) {
    await page(session).click(selector);
}
export async function pageFocus(session, selector) {
    await page(session).focus(selector);
}
export async function pageType(session, text) {
    await page(session).keyboard.type(text);
}
export async function pagePress(session, key) {
    await page(session).keyboard.press(key);
}
const TAURI_WAIT_CHUNK_MS = 25_000;
async function waitWithChunks(totalMs, run) {
    const deadline = Date.now() + totalMs;
    let lastError;
    while (Date.now() < deadline) {
        const remaining = deadline - Date.now();
        const chunk = Math.min(TAURI_WAIT_CHUNK_MS, remaining);
        try {
            await run(chunk);
            return;
        }
        catch (err) {
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
export async function pageWaitForSelector(session, selector, timeout) {
    const total = timeout ?? 60_000;
    await waitWithChunks(total, (chunk) => page(session).waitForSelector(selector, chunk));
}
export async function pageWaitForText(session, text, timeout) {
    const total = timeout ?? 60_000;
    await waitWithChunks(total, (chunk) => page(session).getByText(text).waitFor(chunk));
}
export async function pageSnapshotTree(session) {
    return page(session).evaluate(`(() => {
      const shell = document.querySelector('[data-testid=shell]');
      return shell ? shell.innerText : document.body.innerText;
    })()`);
}
export async function pageScreenshot(session) {
    return page(session).screenshot();
}
export async function pageLocatorScreenshot(session, selector) {
    const b64 = await page(session).evaluate(`(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) throw new Error('Missing selector');
      const rect = el.getBoundingClientRect();
      const canvas = document.createElement('canvas');
      canvas.width = Math.max(1, Math.floor(rect.width));
      canvas.height = Math.max(1, Math.floor(rect.height));
      const ctx = canvas.getContext('2d');
      if (!ctx) return '';
      ctx.fillStyle = getComputedStyle(el).backgroundColor || 'transparent';
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      return canvas.toDataURL('image/png').split(',')[1];
    })()`);
    return Buffer.from(String(b64), "base64");
}
export async function pageEmulateReducedMotion(session, enabled) {
    await page(session).evaluate(`(() => {
      document.documentElement.style.setProperty(
        'animation-duration',
        ${JSON.stringify(enabled ? "0.001ms" : "")},
        'important'
      );
    })()`);
}
export async function pageClockStep(_session, durationMs) {
    await new Promise((resolve) => setTimeout(resolve, durationMs));
}
export async function pageLayout(session, selector) {
    return pageEvaluate(session, (sel) => {
        const el = document.querySelector(sel);
        if (!el)
            throw new Error(`Missing: ${sel}`);
        const s = getComputedStyle(el);
        return {
            display: s.display,
            flexDirection: s.flexDirection,
            gridTemplateColumns: s.gridTemplateColumns,
        };
    }, selector);
}
export async function pageCssTokenAssert(session, selector, token, property) {
    const actual = await pageEvaluate(session, (arg) => {
        const { sel, prop } = arg;
        const el = document.querySelector(sel);
        if (!el)
            throw new Error(`Missing selector: ${sel}`);
        const style = getComputedStyle(el);
        if (prop === "color")
            return style.color;
        if (prop === "background-color")
            return style.backgroundColor;
        if (prop === "border-color")
            return style.borderColor;
        return style.color;
    }, { sel: selector, prop: property });
    const expected = await pageEvaluate(session, (t) => getComputedStyle(document.documentElement).getPropertyValue(t).trim(), token);
    return { actual: String(actual), expected: String(expected) };
}
export async function pageFill(session, selector, text) {
    await pageFocus(session, selector);
    await page(session).evaluate(`(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!(el instanceof HTMLTextAreaElement || el instanceof HTMLInputElement)) {
        throw new Error("fill requires input or textarea");
      }
      const prototype =
        el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
      const setter = Object.getOwnPropertyDescriptor(prototype, "value")?.set;
      setter?.call(el, ${JSON.stringify(text)});
      el.dispatchEvent(new Event("input", { bubbles: true }));
    })()`);
}
export async function pageCanvasHash(session, selector) {
    return pageEvaluate(session, (sel) => {
        const c = document.querySelector(sel);
        if (!c)
            return "";
        const ctx2d = c.getContext("2d");
        if (ctx2d) {
            return ctx2d.getImageData(0, 0, c.width, c.height).data.join(",").slice(0, 500);
        }
        const gl = c.getContext("webgl2") ?? c.getContext("webgl");
        if (!gl)
            return "";
        const chunks = [];
        for (let i = 0; i < 8; i += 1) {
            const sample = new Uint8Array(4);
            const x = Math.max(0, Math.floor((c.width * (i + 1)) / 9));
            const y = Math.max(0, Math.floor((c.height * (i + 1)) / 9));
            gl.readPixels(x, y, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, sample);
            chunks.push(...sample);
        }
        return chunks.join(",");
    }, selector);
}
export async function pageCanvasMeta(session, selector) {
    return pageEvaluate(session, (sel) => {
        const el = document.querySelector(sel);
        if (!(el instanceof HTMLCanvasElement)) {
            return { renderer: "", motionTier: "", webgl: false };
        }
        const webgl = Boolean(el.getContext("webgl2") ?? el.getContext("webgl"));
        return {
            renderer: el.dataset.renderer ?? "",
            motionTier: el.dataset.motionTier ?? "",
            webgl,
        };
    }, selector);
}
export async function readObservabilitySnapshot(session) {
    return pageEvaluate(session, () => {
        const el = document.querySelector('[data-testid="ui-observability"]');
        const globalSnapshot = window.__REX_UI_OBSERVABILITY__;
        if (globalSnapshot)
            return globalSnapshot;
        if (!el)
            return { error: "missing ui-observability node" };
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
    }, null);
}
