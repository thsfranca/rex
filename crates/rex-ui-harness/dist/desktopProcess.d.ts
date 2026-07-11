export declare function cargoTargetDir(repoRoot: string): string;
export declare function resolveRexBinary(repoRoot: string): string;
/** Electron app directory (apps/rex-desktop). */
export declare function resolveDesktopAppDir(repoRoot: string): string;
export declare function harnessDesktopCwd(): string;
export declare function resetProbeDaemon(rexRoot: string): Promise<void>;
export declare function stopHarnessDesktopApps(repoRoot: string): Promise<void>;
