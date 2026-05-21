/**
 * Appends onboarding hints when Node reports the executable could not be found.
 * Messages are plain text; they reference the repo doc path for contributors.
 */
export const EXTENSION_LOCAL_E2E_DOC_PATH = "docs/EXTENSION_LOCAL_E2E.md";

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
  return `${message} Set rex.cliPath to the absolute path to rex-cli (see ${EXTENSION_LOCAL_E2E_DOC_PATH} in the REX repository).`;
}

export function appendDaemonExecutableNotFoundHint(err: unknown, message: string): string {
  if (!isExecutableNotFoundError(err)) {
    return message;
  }
  return `${message} Set rex.daemonBinaryPath to the absolute path to rex-daemon (see ${EXTENSION_LOCAL_E2E_DOC_PATH} in the REX repository).`;
}

/**
 * Appends MVP operator hints for sidecar/HTTP/daemon configuration failures.
 */
export function appendStreamSetupHint(message: string): string {
  const lower = message.toLowerCase();
  if (lower.includes("sidecar required") || lower.includes("sidecar unavailable")) {
    return `${message} Enable REX_SIDECAR_ENABLED=1 and ensure rex-sidecar-stub is on PATH (see ${EXTENSION_LOCAL_E2E_DOC_PATH} §3 in the REX repository).`;
  }
  if (lower.includes("inference runtime") || lower.includes("rex_openai_compat")) {
    return `${message} Configure brokered HTTP: REX_OPENAI_COMPAT_BASE_URL and REX_OPENAI_COMPAT_MODEL (see ${EXTENSION_LOCAL_E2E_DOC_PATH} §3 in the REX repository).`;
  }
  if (lower.includes("daemon is unavailable") || lower.includes("daemon unavailable")) {
    return `${message} Start rex-daemon with sidecar and HTTP env (see ${EXTENSION_LOCAL_E2E_DOC_PATH} in the REX repository).`;
  }
  return message;
}
