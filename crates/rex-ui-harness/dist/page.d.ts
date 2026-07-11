import type { Page } from "playwright";
export interface HarnessSession {
    mode: "desktop";
    page: Page;
    motionFrames: Buffer[];
    recording: boolean;
}
export declare function pageEvaluate<T>(session: HarnessSession, fn: (arg: unknown) => T, arg: unknown): Promise<T>;
export declare function pageClick(session: HarnessSession, selector: string): Promise<void>;
export declare function pageFocus(session: HarnessSession, selector: string): Promise<void>;
export declare function pageType(session: HarnessSession, text: string): Promise<void>;
export declare function pagePress(session: HarnessSession, key: string): Promise<void>;
export declare function pageWaitForSelector(session: HarnessSession, selector: string, timeout?: number): Promise<void>;
export declare function pageWaitForText(session: HarnessSession, text: string, timeout?: number): Promise<void>;
export declare function pageSnapshotTree(session: HarnessSession): Promise<string>;
export declare function pageScreenshot(session: HarnessSession): Promise<Buffer>;
export declare function pageLocatorScreenshot(session: HarnessSession, selector: string): Promise<Buffer>;
export declare function pageEmulateReducedMotion(session: HarnessSession, enabled: boolean): Promise<void>;
export declare function pageClockStep(_session: HarnessSession, durationMs: number): Promise<void>;
export declare function pageLayout(session: HarnessSession, selector: string): Promise<{
    display: string;
    flexDirection: string;
    gridTemplateColumns: string;
}>;
export declare function pageCssTokenAssert(session: HarnessSession, selector: string, token: string, property: string): Promise<{
    actual: string;
    expected: string;
}>;
export declare function pageFill(session: HarnessSession, selector: string, text: string): Promise<void>;
export declare function pageCanvasHash(session: HarnessSession, selector: string): Promise<string>;
export declare function pageCanvasMeta(session: HarnessSession, selector: string): Promise<{
    renderer: string;
    motionTier: string;
    webgl: boolean;
}>;
export declare function readObservabilitySnapshot(session: HarnessSession): Promise<Record<string, unknown>>;
