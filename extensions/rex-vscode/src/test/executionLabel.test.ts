import { describe, expect, it } from "vitest";

import {
  formatExecutionLabel,
  isTimelineNoise,
  shouldShowExecutionDetail,
} from "../../webview/timeline/executionLabel";

describe("formatExecutionLabel", () => {
  it("formats read_file with basename", () => {
    expect(
      formatExecutionLabel({
        summary: "read_file",
        detail: "src/ui/chatPanel.ts",
        phase: "running",
        kind: "tool",
      }),
    ).toBe("Reading chatPanel.ts");
  });

  it("formats terminal commands with human verbs", () => {
    expect(
      formatExecutionLabel({
        summary: "run_terminal_cmd",
        detail: "npm test -- --run extensions/rex-vscode",
        phase: "completed",
        kind: "tool",
      }),
    ).toBe("Ran npm test -- --run extensions/rex-vscode");
  });

  it("uses past tense when completed", () => {
    expect(
      formatExecutionLabel({
        summary: "grep",
        detail: "executionStep",
        phase: "completed",
        kind: "tool",
      }),
    ).toBe("Searched for executionStep");
  });
});

describe("isTimelineNoise", () => {
  it("hides generic lifecycle steps", () => {
    expect(
      isTimelineNoise({
        summary: "Execution started.",
        phase: "running",
        kind: "step",
      }),
    ).toBe(true);
  });

  it("keeps tool entries", () => {
    expect(
      isTimelineNoise({
        summary: "read_file",
        detail: "chatPanel.ts",
        phase: "running",
        kind: "tool",
      }),
    ).toBe(false);
  });
});

describe("shouldShowExecutionDetail", () => {
  it("shows raw command for long shell invocations", () => {
    expect(
      shouldShowExecutionDetail(
        "run_terminal_cmd",
        "cd extensions/rex-vscode && npm test -- --run extensions/rex-vscode",
      ),
    ).toBe(true);
  });

  it("hides detail for short grep queries", () => {
    expect(shouldShowExecutionDetail("grep", "executionStep")).toBe(false);
  });

  it("shows full path when label uses basename only", () => {
    expect(shouldShowExecutionDetail("read_file", "src/ui/chatPanel.ts")).toBe(true);
  });
});
