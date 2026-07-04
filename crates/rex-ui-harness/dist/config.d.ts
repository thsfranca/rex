export interface HarnessConfig {
    baseUrl: string;
    viewport: {
        width: number;
        height: number;
    };
    baselineDir: string;
    staticRoot: string;
}
export declare function loadConfig(repoRoot: string): HarnessConfig;
export declare function findRepoRoot(start: string): string;
