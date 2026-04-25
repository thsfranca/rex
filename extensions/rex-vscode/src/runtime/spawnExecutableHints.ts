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
