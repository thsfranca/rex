/** Serve the production `apps/rex-web/dist` bundle (vite preview), not the dev server. */
export declare function ensureWebUiServer(repoRoot: string): Promise<void>;
export declare function stopWebUiServer(): Promise<void>;
/** @deprecated Use ensureWebUiServer */
export declare const ensureViteDevServer: typeof ensureWebUiServer;
/** @deprecated Use stopWebUiServer */
export declare const stopViteDevServer: typeof stopWebUiServer;
