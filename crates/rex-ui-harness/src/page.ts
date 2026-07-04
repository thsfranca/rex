import type { Page } from "playwright";
import type { TauriPage } from "@srsholmes/tauri-playwright";
import type { HarnessMode } from "./config.js";

export interface HarnessSession {
  mode: HarnessMode;
  page: Page | TauriPage;
  motionFrames: Buffer[];
  recording: boolean;
}

export async function pageEvaluate<T>(
  session: HarnessSession,
  fn: (arg: unknown) => T,
  arg: unknown
): Promise<T> {
  const { page, mode } = session;
  if (mode === "static") {
    return (page as Page).evaluate(fn, arg);
  }
  const script = `(${fn.toString()})(${JSON.stringify(arg)})`;
  return (page as TauriPage).evaluate(script) as Promise<T>;
}

export async function pageClick(session: HarnessSession, selector: string): Promise<void> {
  if (session.mode === "static") {
    await (session.page as Page).click(selector);
  } else {
    await (session.page as TauriPage).click(selector);
  }
}

export async function pageFocus(session: HarnessSession, selector: string): Promise<void> {
  if (session.mode === "static") {
    await (session.page as Page).focus(selector);
  } else {
    await (session.page as TauriPage).focus(selector);
  }
}

export async function pageType(session: HarnessSession, text: string): Promise<void> {
  if (session.mode === "static") {
    await (session.page as Page).keyboard.type(text);
  } else {
    await (session.page as TauriPage).keyboard.type(text);
  }
}

export async function pagePress(session: HarnessSession, key: string): Promise<void> {
  if (session.mode === "static") {
    await (session.page as Page).keyboard.press(key);
  } else {
    await (session.page as TauriPage).keyboard.press(key);
  }
}

export async function pageWaitForSelector(
  session: HarnessSession,
  selector: string,
  timeout?: number
): Promise<void> {
  if (session.mode === "static") {
    await (session.page as Page).waitForSelector(selector, { timeout });
  } else {
    await (session.page as TauriPage).waitForSelector(selector, timeout);
  }
}

export async function pageWaitForText(
  session: HarnessSession,
  text: string,
  timeout?: number
): Promise<void> {
  if (session.mode === "static") {
    await (session.page as Page).getByText(text).waitFor({ timeout });
  } else {
    await (session.page as TauriPage).getByText(text).waitFor(timeout);
  }
}

export async function pageSnapshotTree(session: HarnessSession): Promise<string> {
  if (session.mode === "static") {
    return (session.page as Page).locator("body").ariaSnapshot();
  }
  return (session.page as TauriPage).evaluate(
    `(() => {
      const shell = document.querySelector('[data-testid=shell]');
      return shell ? shell.innerText : document.body.innerText;
    })()`
  );
}

export async function pageScreenshot(session: HarnessSession): Promise<Buffer> {
  if (session.mode === "static") {
    return (session.page as Page).screenshot();
  }
  return (session.page as TauriPage).screenshot();
}

export async function pageLocatorScreenshot(
  session: HarnessSession,
  selector: string
): Promise<Buffer> {
  if (session.mode === "static") {
    return (session.page as Page).locator(selector).screenshot();
  }
  return (session.page as TauriPage).screenshot();
}

export async function pageEmulateReducedMotion(
  session: HarnessSession,
  enabled: boolean
): Promise<void> {
  if (session.mode !== "static") {
    throw new Error("ui_set_prefers_reduced_motion requires static fixture mode");
  }
  await (session.page as Page).emulateMedia({
    reducedMotion: enabled ? "reduce" : "no-preference",
  });
}

export async function pageClockStep(session: HarnessSession, durationMs: number): Promise<void> {
  if (session.mode !== "static") {
    throw new Error("ui_clock_step requires static fixture mode (no Playwright clock in desktop)");
  }
  await (session.page as Page).clock.fastForward(durationMs);
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
  if (session.mode === "static") {
    await (session.page as Page).fill(selector, text);
  } else {
    await (session.page as TauriPage).fill(selector, text);
  }
}

export async function pageCanvasHash(session: HarnessSession, selector: string): Promise<string> {
  return pageEvaluate(
    session,
    (sel) => {
      const c = document.querySelector(sel as string) as HTMLCanvasElement | null;
      if (!c) return "";
      const ctx = c.getContext("2d");
      return ctx?.getImageData(0, 0, c.width, c.height).data.join(",").slice(0, 500) ?? "";
    },
    selector
  );
}
