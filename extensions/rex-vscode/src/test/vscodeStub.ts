/**
 * Minimal `vscode` module stub for unit tests.
 *
 * Headless modules under `src/runtime/*` and `src/shared/*` must not depend on
 * this stub. It is only present to satisfy the rare unit test that imports a
 * module touching `vscode` in a controlled way. Integration tests that need a
 * real VS Code host should run under `@vscode/test-electron` instead (added in
 * PR 2 once the webview lands).
 */

export class ThemeColor {
  constructor(public readonly id: string) {}
}

export const StatusBarAlignment = {
  Left: 1,
  Right: 2,
} as const;

export const window = {
  activeTextEditor: undefined,
  createOutputChannel: (_name: string) => ({
    appendLine: (_line: string) => undefined,
    show: (_preserveFocus?: boolean) => undefined,
    dispose: () => undefined,
  }),
  createStatusBarItem: (_alignment: number, _priority?: number) => ({
    text: "",
    tooltip: "",
    command: "",
    name: "",
    backgroundColor: undefined,
    show: () => undefined,
    hide: () => undefined,
    dispose: () => undefined,
  }),
  showInformationMessage: (_message: string, ..._items: string[]) =>
    Promise.resolve(undefined),
  showWarningMessage: (_message: string, ..._items: string[]) =>
    Promise.resolve(undefined),
};

export const workspace = {
  getConfiguration: (_section?: string) => ({
    get: (_key: string) => undefined,
  }),
  onDidChangeConfiguration: (_listener: (event: unknown) => void) => ({
    dispose: () => undefined,
  }),
};

export const commands = {
  registerCommand: (
    _id: string,
    _handler: (...args: unknown[]) => unknown,
  ) => ({
    dispose: () => undefined,
  }),
};

export default { ThemeColor, StatusBarAlignment, window, workspace, commands };
