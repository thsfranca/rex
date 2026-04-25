/**
 * Minimal `vscode` module stub for unit tests.
 *
 * Headless modules under `src/runtime/*` and `src/shared/*` must not depend on
 * this stub. It is only present to satisfy unit tests that import a module
 * touching `vscode` in a controlled way (for example `editor/virtualDocs.ts`
 * or the chat message bus). Integration tests that need a real VS Code host
 * should run under `@vscode/test-electron` instead.
 */

type EventListener<T> = (value: T) => void;

export class EventEmitter<T> {
  private readonly listeners = new Set<EventListener<T>>();
  readonly event = (listener: EventListener<T>): { dispose: () => void } => {
    this.listeners.add(listener);
    return { dispose: () => this.listeners.delete(listener) };
  };
  fire(value: T): void {
    for (const listener of this.listeners) {
      listener(value);
    }
  }
  dispose(): void {
    this.listeners.clear();
  }
}

export class Uri {
  private constructor(
    public readonly scheme: string,
    public readonly authority: string,
    public readonly path: string,
    public readonly query: string,
    public readonly fragment: string,
  ) {}

  static from(components: {
    scheme: string;
    authority?: string;
    path?: string;
    query?: string;
    fragment?: string;
  }): Uri {
    return new Uri(
      components.scheme,
      components.authority ?? "",
      components.path ?? "",
      components.query ?? "",
      components.fragment ?? "",
    );
  }

  static joinPath(base: Uri, ...segments: string[]): Uri {
    const joined = [base.path, ...segments].join("/").replace(/\/+/, "/");
    return Uri.from({ scheme: base.scheme, path: joined });
  }

  toString(): string {
    return `${this.scheme}://${this.authority}${this.path}`;
  }
}

export class ThemeColor {
  constructor(public readonly id: string) {}
}

export const StatusBarAlignment = {
  Left: 1,
  Right: 2,
} as const;

export const ColorThemeKind = {
  Light: 1,
  Dark: 2,
  HighContrast: 3,
  HighContrastLight: 4,
} as const;

export const window = {
  activeTextEditor: undefined,
  activeColorTheme: { kind: ColorThemeKind.Dark } as { kind: number },
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
  onDidChangeTextEditorSelection: (_listener: (event: unknown) => void) => ({
    dispose: () => undefined,
  }),
  onDidChangeActiveTextEditor: (_listener: (event: unknown) => void) => ({
    dispose: () => undefined,
  }),
  onDidChangeActiveColorTheme: (_listener: (event: unknown) => void) => ({
    dispose: () => undefined,
  }),
  registerWebviewViewProvider: (_id: string, _provider: unknown, _options?: unknown) => ({
    dispose: () => undefined,
  }),
};

export const workspace = {
  getConfiguration: (_section?: string) => ({
    get: (_key: string) => undefined,
  }),
  onDidChangeConfiguration: (_listener: (event: unknown) => void) => ({
    dispose: () => undefined,
  }),
  registerTextDocumentContentProvider: (
    _scheme: string,
    _provider: unknown,
  ) => ({
    dispose: () => undefined,
  }),
  applyEdit: async (_edit: unknown) => true,
  asRelativePath: (uri: { fsPath?: string } | string) =>
    typeof uri === "string" ? uri : uri.fsPath ?? "",
};

export const commands = {
  registerCommand: (
    _id: string,
    _handler: (...args: unknown[]) => unknown,
  ) => ({
    dispose: () => undefined,
  }),
  executeCommand: async (..._args: unknown[]) => undefined,
};

export const env = {
  clipboard: {
    writeText: async (_text: string) => undefined,
  },
};

export class Disposable {
  constructor(private readonly cb?: () => void) {}
  dispose(): void {
    this.cb?.();
  }
}

export class Position {
  constructor(public readonly line: number, public readonly character: number) {}
  isEqual(other: Position): boolean {
    return this.line === other.line && this.character === other.character;
  }
}

export class Range {
  constructor(public readonly start: Position, public readonly end: Position) {}
}

export class Selection extends Range {
  constructor(public readonly anchor: Position, public readonly active: Position) {
    super(anchor, active);
  }
  get isEmpty(): boolean {
    return this.anchor.isEqual(this.active);
  }
}

export class WorkspaceEdit {
  replace(_uri: Uri, _range: Range, _newText: string): void {
    return undefined;
  }
}

export default {
  ThemeColor,
  StatusBarAlignment,
  ColorThemeKind,
  window,
  workspace,
  commands,
  env,
  EventEmitter,
  Uri,
  Disposable,
  Position,
  Range,
  Selection,
  WorkspaceEdit,
};
