import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

const DEFAULT_EXECUTABLE = "rex";

const WELL_KNOWN_REX_PATHS = [
  path.join(os.homedir(), ".cargo", "bin", "rex"),
];

type ExistsFn = (candidate: string) => boolean;

/**
 * Resolve the Rex CLI binary when settings still use the default lookup name.
 * GUI-launched editors often omit ~/.cargo/bin from PATH even when rex is installed.
 */
export function resolveRexExecutable(
  configured: string,
  exists: ExistsFn = fs.existsSync,
): string {
  const trimmed = configured.trim() || DEFAULT_EXECUTABLE;
  if (trimmed !== DEFAULT_EXECUTABLE) {
    return trimmed;
  }
  for (const candidate of WELL_KNOWN_REX_PATHS) {
    if (exists(candidate)) {
      return candidate;
    }
  }
  return trimmed;
}
