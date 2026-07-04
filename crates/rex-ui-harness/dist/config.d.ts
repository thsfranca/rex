export type HarnessMode = "desktop" | "static";
export interface HarnessConfig {
    mode: HarnessMode;
    repoRoot: string;
    baseUrl: string;
    viewport: {
        width: number;
        height: number;
    };
    baselineDir: string;
    staticRoot: string;
    rexRoot: string;
    workspaceDir: string;
    desktopSocket: string;
    desktopStartTimeoutSecs: number;
}
export declare function loadConfig(repoRoot: string): HarnessConfig;
export declare function findRepoRoot(start: string): string;
