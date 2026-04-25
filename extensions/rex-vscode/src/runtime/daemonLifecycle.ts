import { spawn, type ChildProcessByStdio } from "node:child_process";
import { setTimeout as delay } from "node:timers/promises";
import type { Readable } from "node:stream";

import { fetchStatus, type CliBridgeOptions, type StatusSnapshot } from "./cliBridge";

type DaemonChildProcess = ChildProcessByStdio<null, Readable, Readable>;

export type DaemonLifecycleState =
  | { readonly kind: "unavailable"; readonly reason: string }
  | { readonly kind: "starting" }
  | { readonly kind: "ready"; readonly status: StatusSnapshot };

export interface DaemonLifecycleOptions {
  readonly cli: CliBridgeOptions;
  readonly daemonBinaryPath: string;
  /**
   * Total time budget when waiting for the daemon to become ready during
   * auto-start. Defaults to 10 seconds.
   */
  readonly readyTimeoutMs?: number;
  /**
   * Poll interval while waiting for readiness. Defaults to 250ms.
   */
  readonly pollIntervalMs?: number;
  /**
   * Hook invoked on every state transition. Useful for status bar updates and
   * output channel logging.
   */
  readonly onState?: (state: DaemonLifecycleState) => void;
}

const DEFAULT_READY_TIMEOUT_MS = 10_000;
const DEFAULT_POLL_INTERVAL_MS = 250;

export class DaemonLifecycle {
  private options: DaemonLifecycleOptions;
  private ownedChild: DaemonChildProcess | undefined;
  private lastState: DaemonLifecycleState = {
    kind: "unavailable",
    reason: "not probed yet",
  };

  constructor(options: DaemonLifecycleOptions) {
    this.options = options;
  }

  getState(): DaemonLifecycleState {
    return this.lastState;
  }

  async probe(signal?: AbortSignal): Promise<DaemonLifecycleState> {
    try {
      const status = await fetchStatus(this.options.cli, signal);
      this.transition({ kind: "ready", status });
    } catch (err) {
      this.transition({
        kind: "unavailable",
        reason: toErrorMessage(err),
      });
    }
    return this.lastState;
  }

  /**
   * Spawn `rex-daemon` and poll `rex-cli status` until ready or the timeout
   * elapses. Caller owns the lifecycle result; failures return `unavailable`
   * with a reason.
   */
  async ensureRunning(signal?: AbortSignal): Promise<DaemonLifecycleState> {
    const probeState = await this.probe(signal);
    if (probeState.kind === "ready") {
      return probeState;
    }
    if (this.ownedChild !== undefined && !this.ownedChild.killed) {
      return this.waitForReady(signal);
    }
    return this.startAndWait(signal);
  }

  /**
   * Terminate the daemon if this lifecycle instance owns it.
   */
  async shutdown(signal?: AbortSignal): Promise<void> {
    const child = this.ownedChild;
    if (child === undefined || child.killed) {
      return;
    }
    child.kill("SIGTERM");
    const deadline = Date.now() + 5_000;
    while (!child.killed && Date.now() < deadline) {
      if (signal?.aborted === true) {
        break;
      }
      await delay(100);
    }
    if (!child.killed) {
      child.kill("SIGKILL");
    }
    this.ownedChild = undefined;
  }

  private async startAndWait(signal?: AbortSignal): Promise<DaemonLifecycleState> {
    this.transition({ kind: "starting" });
    try {
      this.ownedChild = spawn(this.options.daemonBinaryPath, [], {
        stdio: ["ignore", "pipe", "pipe"],
        detached: false,
      });
    } catch (err) {
      this.transition({
        kind: "unavailable",
        reason: `failed to spawn daemon: ${toErrorMessage(err)}`,
      });
      return this.lastState;
    }
    this.ownedChild.once("exit", (code) => {
      if (this.lastState.kind !== "ready") {
        this.transition({
          kind: "unavailable",
          reason: `rex-daemon exited with code ${code}`,
        });
      }
      this.ownedChild = undefined;
    });
    return this.waitForReady(signal);
  }

  private async waitForReady(signal?: AbortSignal): Promise<DaemonLifecycleState> {
    const timeoutMs = this.options.readyTimeoutMs ?? DEFAULT_READY_TIMEOUT_MS;
    const pollMs = this.options.pollIntervalMs ?? DEFAULT_POLL_INTERVAL_MS;
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
      if (signal?.aborted === true) {
        this.transition({ kind: "unavailable", reason: "cancelled" });
        return this.lastState;
      }
      try {
        const status = await fetchStatus(this.options.cli, signal);
        this.transition({ kind: "ready", status });
        return this.lastState;
      } catch {
        await delay(pollMs);
      }
    }
    this.transition({
      kind: "unavailable",
      reason: `daemon did not become ready within ${timeoutMs}ms`,
    });
    return this.lastState;
  }

  private transition(next: DaemonLifecycleState): void {
    this.lastState = next;
    this.options.onState?.(next);
  }
}

function toErrorMessage(err: unknown): string {
  if (err instanceof Error) {
    return err.message;
  }
  return String(err);
}
