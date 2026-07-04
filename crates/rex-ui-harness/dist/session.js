let session = null;
export function getSession() {
    if (!session)
        throw new Error("No active session — call ui_open first");
    return session;
}
export async function openSession(cfg, launch = {}) {
    if (session)
        await closeSession();
    const { chromium } = await import("playwright");
    const browser = await chromium.launch({ headless: launch.headless ?? true });
    const context = await browser.newContext({
        viewport: cfg.viewport,
    });
    const page = await context.newPage();
    await page.clock.install({ time: new Date("2026-01-01T00:00:00Z") });
    await page.goto(cfg.baseUrl, { waitUntil: "domcontentloaded" });
    session = { browser, context, page, motionFrames: [], recording: false };
    return session;
}
export async function closeSession() {
    if (!session)
        return;
    await session.context.close();
    await session.browser.close();
    session = null;
}
export async function gotoScenario(scenario) {
    const { page } = getSession();
    await page.evaluate((name) => {
        const probe = window
            .__rexProbe;
        if (!probe)
            throw new Error("Probe harness not loaded");
        return probe.gotoScenario(name);
    }, scenario);
    await page.waitForTimeout(50);
}
