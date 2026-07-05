import type { HarnessConfig } from "./config.js";
import type { HarnessSession } from "./page.js";
export type { HarnessSession };
export declare function getSession(): HarnessSession;
export declare function openSession(cfg: HarnessConfig, launch?: {
    mode?: "desktop";
}): Promise<HarnessSession>;
export declare function closeSession(): Promise<void>;
export declare function dumpObservability(label: string): Promise<string>;
export declare function gotoScenario(scenario: string): Promise<void>;
