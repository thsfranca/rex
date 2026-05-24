import * as fs from "node:fs/promises";
import * as path from "node:path";
import { afterEach, describe, expect, it } from "vitest";

import {
  DaemonLifecycle,
  type DaemonLifecycleOptions,
  type DaemonLifecycleState,
} from "../runtime/daemonLifecycle";
import { EXTENSION_LOCAL_E2E_DOC_PATH } from "../runtime/spawnExecutableHints";

const FIXTURES_DIR = path.resolve(__dirname, "fixtures");
const FIXTURE_CLI_STATUS_OK = path.join(FIXTURES_DIR, "cli_status_ok.sh");
const FIXTURE_CLI_STATUS_FAIL = path.join(FIXTURES_DIR, "cli_status_fail.sh");
const FIXTURE_DAEMON_SLEEP = path.join(FIXTURES_DIR, "daemon_sleep.sh");
const FIXTURE_DAEMON_EXITS = path.join(FIXTURES_DIR, "daemon_exits.sh");
const FIXTURE_CLI_FLAKY = path.join(FIXTURES_DIR, "cli_status_fail_twice_then_ok.sh");
const FIXTURE_CLI_OK_THEN_FAIL = path.join(FIXTURES_DIR, "cli_status_ok_then_fail.sh");
const FIXTURE_CLI_FAIL_THEN_OK = path.join(FIXTURES_DIR, "cli_status_fail_then_ok.sh");

async function makeWorkspaceTmp(): Promise<string> {
  const base = path.resolve(__dirname, "..", "..", ".vitest-tmp");
  await fs.mkdir(base, { recursive: true });
  return fs.mkdtemp(path.join(base, "daemon-lifecycle-"));
}

function makeLifecycle(
  overrides: Partial<DaemonLifecycleOptions>,
  transitions?: DaemonLifecycleState[],
): DaemonLifecycle {
  const base: DaemonLifecycleOptions = {
    cli: { cliPath: FIXTURE_CLI_STATUS_OK, timeoutMs: 2_000 },
    daemonBinaryPath: FIXTURE_DAEMON_SLEEP,
    readyTimeoutMs: 1_500,
    pollIntervalMs: 50,
    onState: transitions ? (state) => transitions.push(state) : undefined,
  };
  return new DaemonLifecycle({ ...base, ...overrides });
}

describe("DaemonLifecycle.ensureRunning", () => {
  const lifecycles: DaemonLifecycle[] = [];

  afterEach(async () => {
    while (lifecycles.length > 0) {
      const lifecycle = lifecycles.pop();
      if (lifecycle) {
        await lifecycle.shutdown();
      }
    }
  });

  it("probe transitions from unavailable to ready when status recovers", async () => {
    const tmpDir = await makeWorkspaceTmp();
    const phaseFile = path.join(tmpDir, "phase");
    await fs.writeFile(phaseFile, "0", "utf8");
    const kinds: DaemonLifecycleState["kind"][] = [];
    const lifecycle = new DaemonLifecycle({
      cli: {
        cliPath: FIXTURE_CLI_FAIL_THEN_OK,
        env: { REX_TEST_STATUS_PHASE_FILE: phaseFile },
        timeoutMs: 2_000,
      },
      daemonBinaryPath: FIXTURE_DAEMON_SLEEP,
      onState: (s) => kinds.push(s.kind),
    });
    try {
      const first = await lifecycle.probe();
      expect(first.kind).toBe("unavailable");
      const second = await lifecycle.probe();
      expect(second.kind).toBe("ready");
      if (second.kind === "ready") {
        expect(second.status.activeModelId).toBe("recovered");
      }
      expect(kinds).toEqual(["unavailable", "ready"]);
    } finally {
      await lifecycle.shutdown();
      await fs.rm(tmpDir, { recursive: true, force: true });
    }
  });

  it("probe transitions from ready to unavailable when status later fails", async () => {
    const tmpDir = await makeWorkspaceTmp();
    const phaseFile = path.join(tmpDir, "phase");
    await fs.writeFile(phaseFile, "0", "utf8");
    const kinds: DaemonLifecycleState["kind"][] = [];
    const lifecycle = new DaemonLifecycle({
      cli: {
        cliPath: FIXTURE_CLI_OK_THEN_FAIL,
        env: { REX_TEST_STATUS_PHASE_FILE: phaseFile },
        timeoutMs: 2_000,
      },
      daemonBinaryPath: FIXTURE_DAEMON_SLEEP,
      onState: (s) => kinds.push(s.kind),
    });
    try {
      const first = await lifecycle.probe();
      expect(first.kind).toBe("ready");
      const second = await lifecycle.probe();
      expect(second.kind).toBe("unavailable");
      expect(kinds).toEqual(["ready", "unavailable"]);
    } finally {
      await lifecycle.shutdown();
      await fs.rm(tmpDir, { recursive: true, force: true });
    }
  });

  it("returns ready without spawning the daemon when probe succeeds", async () => {
    const transitions: DaemonLifecycleState[] = [];
    const lifecycle = makeLifecycle(
      { cli: { cliPath: FIXTURE_CLI_STATUS_OK, timeoutMs: 2_000 } },
      transitions,
    );
    lifecycles.push(lifecycle);

    const state = await lifecycle.ensureRunning();

    expect(state.kind).toBe("ready");
    if (state.kind === "ready") {
      expect(state.status.daemonVersion).toBe("0.1.0-test");
      expect(state.status.activeModelId).toBe("test-model");
    }
    expect(transitions.map((t) => t.kind)).toEqual(["ready"]);
  });

  it("includes onboarding hint when rex daemon executable is missing", async () => {
    const lifecycle = makeLifecycle({
      cli: { cliPath: FIXTURE_CLI_STATUS_FAIL, timeoutMs: 2_000 },
      daemonBinaryPath: "/__rex_vitest_nonexistent__/rex",
      readyTimeoutMs: 3_000,
      pollIntervalMs: 50,
    });
    lifecycles.push(lifecycle);

    const state = await lifecycle.ensureRunning();

    expect(state.kind).toBe("unavailable");
    if (state.kind === "unavailable") {
      expect(state.reason).toMatch(/failed to spawn daemon/);
      expect(state.reason).toContain(EXTENSION_LOCAL_E2E_DOC_PATH);
      expect(state.reason).toContain("rex.daemonBinaryPath");
    }
  });

  it("reports unavailable when spawning the daemon fails immediately", async () => {
    const transitions: DaemonLifecycleState[] = [];
    const lifecycle = makeLifecycle(
      {
        cli: { cliPath: FIXTURE_CLI_STATUS_FAIL, timeoutMs: 2_000 },
        daemonBinaryPath: FIXTURE_DAEMON_EXITS,
        readyTimeoutMs: 800,
        pollIntervalMs: 50,
      },
      transitions,
    );
    lifecycles.push(lifecycle);

    const state = await lifecycle.ensureRunning();

    expect(state.kind).toBe("unavailable");
    if (state.kind === "unavailable") {
      expect(state.reason).toMatch(/exited with code|did not become ready/);
    }
    const kinds = transitions.map((t) => t.kind);
    expect(kinds[0]).toBe("unavailable");
    expect(kinds).toContain("starting");
    expect(kinds[kinds.length - 1]).toBe("unavailable");
  });

  it("shutdown terminates a daemon that this lifecycle started", async () => {
    const lifecycle = makeLifecycle({
      cli: { cliPath: FIXTURE_CLI_STATUS_FAIL, timeoutMs: 2_000 },
      daemonBinaryPath: FIXTURE_DAEMON_SLEEP,
      readyTimeoutMs: 400,
      pollIntervalMs: 50,
    });
    lifecycles.push(lifecycle);

    const state = await lifecycle.ensureRunning();
    expect(state.kind).toBe("unavailable");

    await lifecycle.shutdown();
    expect(lifecycle.getState().kind).toBe("unavailable");
  });
});

describe("DaemonLifecycle.ensureRunning single-flight", () => {
  it("becomes ready when status flips after failures (one start cycle)", async () => {
    const tmpDir = await makeWorkspaceTmp();
    const stateFile = path.join(tmpDir, "status_count");
    await fs.writeFile(stateFile, "0", "utf8");
    const kinds: DaemonLifecycleState["kind"][] = [];

    const lifecycle = new DaemonLifecycle({
      cli: {
        cliPath: FIXTURE_CLI_FLAKY,
        env: { REX_TEST_STATUS_STATE_FILE: stateFile },
        timeoutMs: 5_000,
      },
      daemonBinaryPath: FIXTURE_DAEMON_SLEEP,
      readyTimeoutMs: 15_000,
      pollIntervalMs: 50,
      onState: (s) => kinds.push(s.kind),
    });
    try {
      const state = await lifecycle.ensureRunning();
      expect(state.kind).toBe("ready");
      if (state.kind === "ready") {
        expect(state.status.daemonVersion).toBe("1.0.0-flaky");
      }
      expect(kinds.filter((k) => k === "starting").length).toBe(1);
      expect(kinds[kinds.length - 1]).toBe("ready");
    } finally {
      await lifecycle.shutdown();
      await fs.rm(tmpDir, { recursive: true, force: true });
    }
  });

  it("serializes concurrent ensureRunning into one start cycle", async () => {
    const tmpDir = await makeWorkspaceTmp();
    const stateFile = path.join(tmpDir, "status_count");
    await fs.writeFile(stateFile, "0", "utf8");
    const kinds: DaemonLifecycleState["kind"][] = [];

    const lifecycle = new DaemonLifecycle({
      cli: {
        cliPath: FIXTURE_CLI_FLAKY,
        env: { REX_TEST_STATUS_STATE_FILE: stateFile },
        timeoutMs: 5_000,
      },
      daemonBinaryPath: FIXTURE_DAEMON_SLEEP,
      readyTimeoutMs: 15_000,
      pollIntervalMs: 50,
      onState: (s) => kinds.push(s.kind),
    });
    try {
      const [a, b] = await Promise.all([lifecycle.ensureRunning(), lifecycle.ensureRunning()]);
      expect(a.kind).toBe("ready");
      expect(b.kind).toBe("ready");
      expect(kinds.filter((k) => k === "starting").length).toBe(1);
    } finally {
      await lifecycle.shutdown();
      await fs.rm(tmpDir, { recursive: true, force: true });
    }
  });
});
