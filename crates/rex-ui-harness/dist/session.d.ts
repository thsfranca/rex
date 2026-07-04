import type { Page, Browser, BrowserContext } from "playwright";
import type { HarnessConfig } from "./config.js";
export interface SessionState {
    browser: Browser;
    context: BrowserContext;
    page: Page;
    motionFrames: Buffer[];
    recording: boolean;
}
export declare function getSession(): SessionState;
export declare function openSession(cfg: HarnessConfig, launch?: {
    headless?: boolean;
}): Promise<SessionState>;
export declare function closeSession(): Promise<void>;
export declare function gotoScenario(scenario: string): Promise<void>;
