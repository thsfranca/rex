export type HarnessMode = "desktop" | "build";
export interface HarnessConfig {
    mode: HarnessMode;
    repoRoot: string;
    viewport: {
        width: number;
        height: number;
    };
    baselineDir: string;
    rexRoot: string;
    workspaceDir: string;
    desktopSocket: string;
    desktopStartTimeoutSecs: number;
}
export declare function loadConfig(repoRoot: string): HarnessConfig;
export declare function findRepoRoot(start: string): string;
