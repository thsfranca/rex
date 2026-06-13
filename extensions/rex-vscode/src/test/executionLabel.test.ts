import { describe, expect, it } from "vitest";

import {
  extractTargetFromResult,
  formatExecutionLabel,
  isTimelineNoise,
  isToolResultBody,
  resolveTimelineTarget,
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

  it("formats Rex fs.read with target instead of file body", () => {
    expect(
      formatExecutionLabel({
        summary: "fs.read",
        target: "README.md",
        detail: "[cached read of README.md]\n# REX\n\nBody text",
        phase: "completed",
        kind: "tool",
      }),
    ).toBe("Read README.md");
  });

  it("formats Rex fs.list as workspace listing", () => {
    expect(
      formatExecutionLabel({
        summary: "fs.list",
        target: "",
        detail: "cliff.toml, CONTRIBUTING.md, config.json",
        phase: "completed",
        kind: "tool",
      }),
    ).toBe("Listed 3 items");
  });

  it("formats terminal commands with human verbs", () => {
    expect(
      formatExecutionLabel({
        summary: "exec.shell",
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

describe("resolveTimelineTarget", () => {
  it("ignores placeholder tool target detail", () => {
    expect(
      resolveTimelineTarget("fs.read", "running", "tool", "{}"),
    ).toBeUndefined();
    expect(
      formatExecutionLabel({
        summary: "fs.read",
        target: "{}",
        detail: "fs.read requires path",
        phase: "failed",
        kind: "tool",
      }),
    ).toBe("Failed to read fs.read requires path");
  });

  it("preserves path from running phase through completion", () => {
    const running = resolveTimelineTarget("fs.read", "running", "tool", "README.md");
    expect(running).toBe("README.md");

    const completed = resolveTimelineTarget(
      "fs.read",
      "completed",
      "tool",
      "[cached read of README.md]\n# Title",
      running,
    );
    expect(completed).toBe("README.md");
  });
});

describe("isToolResultBody", () => {
  it("detects cached read payloads", () => {
    expect(isToolResultBody("[cached read of README.md]\n# REX")).toBe(true);
  });

  it("detects comma-separated directory listings", () => {
    expect(isToolResultBody("a.md, b.md, c.md")).toBe(true);
  });
});

describe("extractTargetFromResult", () => {
  it("pulls path from cached read marker", () => {
    expect(extractTargetFromResult("fs.read", "[cached read of docs/ARCHITECTURE.md]\n# Arch")).toBe(
      "docs/ARCHITECTURE.md",
    );
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

  it("hides orchestrator invoking step noise", () => {
    expect(
      isTimelineNoise({
        summary: "orchestrator invoking fs.read",
        phase: "running",
        kind: "step",
      }),
    ).toBe(true);
  });

  it("keeps tool entries", () => {
    expect(
      isTimelineNoise({
        summary: "fs.read",
        detail: "chatPanel.ts",
        phase: "running",
        kind: "tool",
      }),
    ).toBe(false);
  });
});

describe("shouldShowExecutionDetail", () => {
  it("shows tool result bodies", () => {
    expect(
      shouldShowExecutionDetail(
        "fs.read",
        "[cached read of README.md]\n# REX\n\nBody",
        "README.md",
      ),
    ).toBe(true);
  });

  it("hides detail when only the target path is present", () => {
    expect(shouldShowExecutionDetail("fs.read", "README.md", "README.md")).toBe(false);
  });
});
