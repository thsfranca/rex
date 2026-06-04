/**
 * Appends onboarding hints when Node reports the executable could not be found.
 * Messages are plain text; they reference the repo doc path for contributors.
 */
export const EXTENSION_LOCAL_E2E_DOC_PATH = "docs/EXTENSION_LOCAL_E2E.md";
export const CONFIGURATION_DOC_PATH = "docs/CONFIGURATION.md";

const SIDECAR_SETUP_HINT =
  "Configure sidecars in $REX_ROOT/config.json and project .rex/config.json: set sidecars.active to agent, binary to rex-agent, and ensure rex-agent is on PATH (pip install -e sidecars/rex-agent; rex proto install). Run rex sidecar doctor. See";
const INFERENCE_SETUP_HINT =
  "Set inference.runtime to http-openai-compat and inference.openai_compat.base_url / model in JSON (rex config init). See";
const DAEMON_SETUP_HINT =
  "Start rex daemon from a project with .rex/config.json (or enable rex.daemonAutoStart). See";

export function isExecutableNotFoundError(err: unknown): boolean {
  if (typeof err !== "object" || err === null) {
    return false;
  }
  return (err as NodeJS.ErrnoException).code === "ENOENT";
}

export function appendCliExecutableNotFoundHint(err: unknown, message: string): string {
  if (!isExecutableNotFoundError(err)) {
    return message;
  }
  return `${message} Set rex.cliPath to the absolute path to rex (see ${EXTENSION_LOCAL_E2E_DOC_PATH} in the REX repository).`;
}

export function appendDaemonExecutableNotFoundHint(err: unknown, message: string): string {
  if (!isExecutableNotFoundError(err)) {
    return message;
  }
  return `${message} Set rex.daemonBinaryPath to the absolute path to rex (see ${EXTENSION_LOCAL_E2E_DOC_PATH} in the REX repository).`;
}

/**
 * Appends MVP operator hints for sidecar/HTTP/daemon configuration failures.
 */
export function appendStreamSetupHint(message: string): string {
  const lower = message.toLowerCase();
  if (lower.includes("sidecar required") || lower.includes("sidecar unavailable")) {
    return `${message} ${SIDECAR_SETUP_HINT} ${EXTENSION_LOCAL_E2E_DOC_PATH} §3 and ${CONFIGURATION_DOC_PATH} in the REX repository.`;
  }
  if (
    lower.includes("inference runtime") ||
    lower.includes("inference_config") ||
    lower.includes("openai_compat") ||
    lower.includes("inference config")
  ) {
    return `${message} ${INFERENCE_SETUP_HINT} ${EXTENSION_LOCAL_E2E_DOC_PATH} §3 and ${CONFIGURATION_DOC_PATH} in the REX repository.`;
  }
  if (lower.includes("daemon is unavailable") || lower.includes("daemon unavailable")) {
    return `${message} ${DAEMON_SETUP_HINT} ${EXTENSION_LOCAL_E2E_DOC_PATH} in the REX repository.`;
  }
  return message;
}
