import { spawn, type ChildProcessByStdio } from "node:child_process";
import { setTimeout as delay } from "node:timers/promises";
import type { Readable } from "node:stream";

export interface StatusSnapshot {
  readonly daemonVersion: string;
  readonly uptimeSeconds: number;
  readonly activeModelId: string;
  readonly capturedAt: number;
}

export interface CliBridgeOptions {
  readonly cliPath: string;
  /**
   * Working directory for the child process. Optional; defaults to process cwd.
   */
  readonly cwd?: string;
  /**
   * Extra environment variables merged on top of `process.env`.
   */
  readonly env?: Readonly<Record<string, string>>;
  /**
   * Hard upper bound for any single CLI invocation.
   */
  readonly timeoutMs?: number;
}

export type CliChildProcess = ChildProcessByStdio<null, Readable, Readable>;

export interface SpawnedProcess {
  readonly child: CliChildProcess;
  readonly dispose: () => void;
}

const DEFAULT_TIMEOUT_MS = 15_000;

/**
 * Spawn `rex-cli status` and parse the three text lines it emits.
 *
 * Uses the stable text output (not NDJSON) because the MVP contract exposes
 * only the NDJSON stream for `complete`. If the upstream status output ever
 * changes format, update both this parser and the contract doc in the same
 * change.
 */
export async function fetchStatus(
  options: CliBridgeOptions,
  signal?: AbortSignal,
): Promise<StatusSnapshot> {
  const stdout = await runCollect(options, ["status"], signal);
  return parseStatusOutput(stdout);
}

export function spawnCompleteStream(
  options: CliBridgeOptions,
  prompt: string,
): SpawnedProcess {
  const env = buildEnv(options.env);
  const child = spawn(options.cliPath, ["complete", prompt, "--format", "ndjson"], {
    cwd: options.cwd,
    env,
    stdio: ["ignore", "pipe", "pipe"],
  });
  const dispose = () => {
    if (!child.killed) {
      child.kill("SIGTERM");
    }
  };
  return { child, dispose };
}

async function runCollect(
  options: CliBridgeOptions,
  args: readonly string[],
  signal?: AbortSignal,
): Promise<string> {
  const env = buildEnv(options.env);
  const child = spawn(options.cliPath, [...args], {
    cwd: options.cwd,
    env,
    stdio: ["ignore", "pipe", "pipe"],
  });
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const controller = new AbortController();
  const onExternalAbort = () => controller.abort();
  signal?.addEventListener("abort", onExternalAbort, { once: true });
  const timer = delay(timeoutMs, undefined, { signal: controller.signal }).then(
    () => {
      if (!child.killed) {
        child.kill("SIGTERM");
      }
      throw new Error(`rex-cli ${args.join(" ")} timed out after ${timeoutMs}ms`);
    },
    () => undefined,
  );

  let stdout = "";
  let stderr = "";
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk: string) => {
    stdout += chunk;
  });
  child.stderr.on("data", (chunk: string) => {
    stderr += chunk;
  });

  try {
    await new Promise<void>((resolve, reject) => {
      child.once("error", reject);
      child.once("close", (code) => {
        if (code === 0) {
          resolve();
        } else {
          const trimmed = stderr.trim() || stdout.trim();
          reject(
            new Error(
              `rex-cli ${args.join(" ")} exited with code ${code}${
                trimmed.length > 0 ? `: ${trimmed}` : ""
              }`,
            ),
          );
        }
      });
    });
    return stdout;
  } finally {
    controller.abort();
    await timer;
    signal?.removeEventListener("abort", onExternalAbort);
  }
}

export function parseStatusOutput(raw: string): StatusSnapshot {
  const fields = new Map<string, string>();
  for (const line of raw.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (trimmed.length === 0) {
      continue;
    }
    const idx = trimmed.indexOf(":");
    if (idx === -1) {
      continue;
    }
    const key = trimmed.slice(0, idx).trim();
    const value = trimmed.slice(idx + 1).trim();
    fields.set(key, value);
  }
  const daemonVersion = fields.get("daemon_version");
  const uptimeRaw = fields.get("uptime_seconds");
  const activeModelId = fields.get("active_model_id");
  if (daemonVersion === undefined || uptimeRaw === undefined || activeModelId === undefined) {
    throw new Error(
      `Unexpected rex-cli status output; missing required fields: ${raw.slice(0, 200)}`,
    );
  }
  const uptimeSeconds = Number.parseInt(uptimeRaw, 10);
  if (!Number.isFinite(uptimeSeconds)) {
    throw new Error(`rex-cli status returned non-numeric uptime_seconds: ${uptimeRaw}`);
  }
  return {
    daemonVersion,
    uptimeSeconds,
    activeModelId,
    capturedAt: Date.now(),
  };
}

function buildEnv(extra: Readonly<Record<string, string>> | undefined): NodeJS.ProcessEnv {
  if (extra === undefined) {
    return process.env;
  }
  return { ...process.env, ...extra };
}
