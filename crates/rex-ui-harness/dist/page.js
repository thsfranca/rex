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
export async function pageWaitForSelector(session, selector, timeout) {
    if (session.mode === "static") {
        await session.page.waitForSelector(selector, { timeout });
    }
    else {
        await session.page.waitForSelector(selector, timeout);
    }
}
export async function pageWaitForText(session, text, timeout) {
    if (session.mode === "static") {
        await session.page.getByText(text).waitFor({ timeout });
    }
    else {
        await session.page.getByText(text).waitFor(timeout);
    }
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
    }
    else {
        await session.page.fill(selector, text);
    }
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
