export async function pageEvaluate(session, fn, arg) {
    const { page, mode } = session;
    if (mode === "static") {
        return page.evaluate(fn, arg);
    }
    const script = `(${fn.toString()})(${JSON.stringify(arg)})`;
    return page.evaluate(script);
}
export async function pageClick(session, selector) {
    if (session.mode === "static") {
        await session.page.click(selector);
    }
    else {
        await session.page.click(selector);
    }
}
export async function pageFocus(session, selector) {
    if (session.mode === "static") {
        await session.page.focus(selector);
    }
    else {
        await session.page.focus(selector);
    }
}
export async function pageType(session, text) {
    if (session.mode === "static") {
        await session.page.keyboard.type(text);
    }
    else {
        await session.page.keyboard.type(text);
    }
}
export async function pagePress(session, key) {
    if (session.mode === "static") {
        await session.page.keyboard.press(key);
    }
    else {
        await session.page.keyboard.press(key);
    }
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
    if (session.mode === "static") {
        await session.page.waitForSelector(selector, { timeout: total });
        return;
    }
    await waitWithChunks(total, (chunk) => session.page.waitForSelector(selector, chunk));
}
export async function pageWaitForText(session, text, timeout) {
    const total = timeout ?? 60_000;
    if (session.mode === "static") {
        await session.page.getByText(text).waitFor({ timeout: total });
        return;
    }
    await waitWithChunks(total, (chunk) => session.page.getByText(text).waitFor(chunk));
}
export async function pageSnapshotTree(session) {
    if (session.mode === "static") {
        return session.page.locator("body").ariaSnapshot();
    }
    return session.page.evaluate(`(() => {
      const shell = document.querySelector('[data-testid=shell]');
      return shell ? shell.innerText : document.body.innerText;
    })()`);
}
export async function pageScreenshot(session) {
    if (session.mode === "static") {
        return session.page.screenshot();
    }
    return session.page.screenshot();
}
export async function pageLocatorScreenshot(session, selector) {
    if (session.mode === "static") {
        return session.page.locator(selector).screenshot();
    }
    return session.page.screenshot();
}
export async function pageEmulateReducedMotion(session, enabled) {
    if (session.mode !== "static") {
        throw new Error("ui_set_prefers_reduced_motion requires static fixture mode");
    }
    await session.page.emulateMedia({
        reducedMotion: enabled ? "reduce" : "no-preference",
    });
}
export async function pageClockStep(session, durationMs) {
    if (session.mode !== "static") {
        throw new Error("ui_clock_step requires static fixture mode (no Playwright clock in desktop)");
    }
    await session.page.clock.fastForward(durationMs);
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
    if (session.mode === "static") {
        await session.page.fill(selector, text);
        return;
    }
    const page = session.page;
    await page.evaluate(`(() => {
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
        const ctx = c.getContext("2d");
        return ctx?.getImageData(0, 0, c.width, c.height).data.join(",").slice(0, 500) ?? "";
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
