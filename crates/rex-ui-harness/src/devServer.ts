import { spawn, type ChildProcess } from "node:child_process";
import path from "node:path";

const VITE_URL = "http://127.0.0.1:5173";

let viteProcess: ChildProcess | null = null;
let startedByHarness = false;

async function viteResponds(): Promise<boolean> {
  try {
    const res = await fetch(VITE_URL, { signal: AbortSignal.timeout(2_000) });
    return res.ok;
  } catch {
    return false;
  }
}

export async function ensureViteDevServer(repoRoot: string): Promise<void> {
  if (await viteResponds()) return;

  const webDir = path.join(repoRoot, "apps/rex-web");
  viteProcess = spawn(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", "5173", "--strictPort"],
    {
      cwd: webDir,
      stdio: "ignore",
      env: process.env,
    }
  );
  startedByHarness = true;

  for (let attempt = 0; attempt < 120; attempt++) {
    if (await viteResponds()) return;
    await new Promise((resolve) => setTimeout(resolve, 500));
  }

  await stopViteDevServer();
  throw new Error(
    "Vite dev server did not start on http://127.0.0.1:5173. Rex desktop loads devUrl in cargo-run builds."
  );
}

export async function stopViteDevServer(): Promise<void> {
  if (!viteProcess || !startedByHarness) return;
  viteProcess.kill("SIGTERM");
  viteProcess = null;
  startedByHarness = false;
}
