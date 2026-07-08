import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { motionOrchestrator } from "./orchestrator";
import { ShellEntrance } from "./ShellEntrance";

describe("ShellEntrance", () => {
  beforeEach(() => {
    window.matchMedia = ((query: string) => ({
      matches: false,
      media: query,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
      addListener: () => undefined,
      removeListener: () => undefined,
      dispatchEvent: () => false,
      onchange: null,
    })) as typeof window.matchMedia;
  });

  afterEach(() => {
    motionOrchestrator.stop();
  });

  it("stays revealed after connectFade pulse decays", async () => {
    motionOrchestrator.signalDaemonReady();
    render(
      <ShellEntrance>
        <div data-testid="shell-child">Shell</div>
      </ShellEntrance>
    );

    expect(screen.getByTestId("shell-child")).toBeInTheDocument();

    await new Promise((resolve) => setTimeout(resolve, 600));
    motionOrchestrator.stop();

    expect(motionOrchestrator.getSnapshot().connectFade).toBe(0);
    expect(document.querySelector(".rex-shell-entrance")).toHaveAttribute(
      "data-shell-revealed",
      "yes"
    );
  });
});
