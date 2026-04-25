import { describe, expect, it } from "vitest";

import { snapshotActiveEditor } from "../editor/context";
import type { TextEditor } from "vscode";

interface FakeEditor {
  document: {
    uri: { fsPath: string };
    languageId: string;
    getText: (range?: FakeRange) => string;
  };
  selection: FakeRange;
}

interface FakeRange {
  isEmpty: boolean;
  start: { line: number; character: number };
  end: { line: number; character: number };
}

function buildEditor(overrides: Partial<FakeEditor> = {}): FakeEditor {
  const selection: FakeRange = overrides.selection ?? {
    isEmpty: true,
    start: { line: 0, character: 0 },
    end: { line: 0, character: 0 },
  };
  return {
    document: {
      uri: { fsPath: "/workspace/main.ts" },
      languageId: "typescript",
      getText: (_range?: FakeRange) => "",
      ...overrides.document,
    },
    selection,
  };
}

describe("snapshotActiveEditor", () => {
  it("returns undefined when there is no active editor", () => {
    expect(snapshotActiveEditor(undefined)).toBeUndefined();
  });

  it("captures the file path and language id without a selection", () => {
    const editor = buildEditor();
    const snapshot = snapshotActiveEditor(editor as unknown as TextEditor);
    expect(snapshot).toEqual({
      filePath: "/workspace/main.ts",
      languageId: "typescript",
      selectionText: undefined,
      selectionRange: undefined,
    });
  });

  it("captures selection metadata when the user has a non-empty selection", () => {
    const editor = buildEditor({
      selection: {
        isEmpty: false,
        start: { line: 2, character: 4 },
        end: { line: 3, character: 10 },
      },
      document: {
        uri: { fsPath: "/workspace/main.ts" },
        languageId: "typescript",
        getText: () => "selected code",
      },
    });
    const snapshot = snapshotActiveEditor(editor as unknown as TextEditor);
    expect(snapshot).toEqual({
      filePath: "/workspace/main.ts",
      languageId: "typescript",
      selectionText: "selected code",
      selectionRange: {
        startLine: 2,
        startCharacter: 4,
        endLine: 3,
        endCharacter: 10,
      },
    });
  });
});
