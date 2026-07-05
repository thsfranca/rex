import { spawn } from "node:child_process";
import path from "node:path";
const WEB_UI_URL = "http://127.0.0.1:5173";
let previewProcess = null;
let startedByHarness = false;
async function webUiResponds() {
    try {
        const res = await fetch(WEB_UI_URL, { signal: AbortSignal.timeout(2_000) });
        return res.ok;
    }
    catch {
        return false;
    }
}
/** Serve the production `apps/rex-web/dist` bundle (vite preview), not the dev server. */
export async function ensureWebUiServer(repoRoot) {
    if (await webUiResponds())
        return;
    const webDir = path.join(repoRoot, "apps/rex-web");
    previewProcess = spawn("npm", ["run", "preview", "--", "--host", "127.0.0.1", "--port", "5173", "--strictPort"], {
        cwd: webDir,
        stdio: "ignore",
        env: process.env,
    });
    startedByHarness = true;
    for (let attempt = 0; attempt < 120; attempt++) {
        if (await webUiResponds())
            return;
        await new Promise((resolve) => setTimeout(resolve, 500));
    }
    await stopWebUiServer();
    throw new Error("Production web UI preview did not start on http://127.0.0.1:5173. Run npm run build in apps/rex-web first.");
}
export async function stopWebUiServer() {
    if (!previewProcess || !startedByHarness)
        return;
    previewProcess.kill("SIGTERM");
    previewProcess = null;
    startedByHarness = false;
}
/** @deprecated Use ensureWebUiServer */
export const ensureViteDevServer = ensureWebUiServer;
/** @deprecated Use stopWebUiServer */
export const stopViteDevServer = stopWebUiServer;
