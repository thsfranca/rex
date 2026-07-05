import { execFile, execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { promisify } from "node:util";
const execFileAsync = promisify(execFile);
let cachedTargetDir = null;
export function cargoTargetDir(repoRoot) {
    if (cachedTargetDir)
        return cachedTargetDir;
    const json = execFileSync("cargo", ["metadata", "--format-version=1", "--no-deps"], {
        cwd: repoRoot,
        encoding: "utf8",
    });
    cachedTargetDir = JSON.parse(json).target_directory;
    return cachedTargetDir;
}
function targetBinary(repoRoot, name, profile = "debug") {
    return path.join(cargoTargetDir(repoRoot), profile, name);
}
function requireBinary(binPath, packageName) {
    if (!fs.existsSync(binPath)) {
        throw new Error(`Missing ${binPath}; run cargo build -p ${packageName}`);
    }
    return binPath;
}
export function resolveRexBinary(repoRoot) {
    return requireBinary(targetBinary(repoRoot, "rex"), "rex");
}
export function resolveDesktopBinary(repoRoot) {
    return requireBinary(targetBinary(repoRoot, "rex-desktop"), "rex-desktop --features e2e-testing");
}
export function harnessDesktopCwd() {
    const dir = path.join(os.tmpdir(), "rex-ui-harness-desktop");
    fs.mkdirSync(dir, { recursive: true });
    return dir;
}
function probeDaemonSocket(rexRoot) {
    const raw = fs.readFileSync(path.join(rexRoot, "config.json"), "utf8");
    const cfg = JSON.parse(raw);
    return cfg.daemon?.socket ?? "/tmp/rex-ui-probe.sock";
}
export async function resetProbeDaemon(rexRoot) {
    const socketPath = probeDaemonSocket(rexRoot);
    try {
        const { stdout } = await execFileAsync("lsof", ["-t", socketPath]);
        for (const pid of stdout.trim().split(/\s+/)) {
            if (pid) {
                process.kill(Number(pid), "SIGTERM");
            }
        }
    }
    catch {
        // No listener on the probe socket.
    }
    await new Promise((resolve) => setTimeout(resolve, 300));
    try {
        fs.unlinkSync(socketPath);
    }
    catch {
        // Socket already gone.
    }
}
export async function stopHarnessDesktopApps(repoRoot) {
    if (process.platform !== "darwin")
        return;
    const patterns = [
        targetBinary(repoRoot, "rex-desktop"),
        "rex-desktop __rex_internal_daemon",
    ];
    for (const pattern of patterns) {
        try {
            await execFileAsync("pkill", ["-f", pattern]);
        }
        catch {
            // No matching processes.
        }
    }
}
