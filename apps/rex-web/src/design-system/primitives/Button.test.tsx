import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Button } from "./Button";
import { SegmentedControl } from "./Textarea";

describe("Button", () => {
  it("renders primary variant", () => {
    render(<Button>Send</Button>);
    expect(screen.getByRole("button", { name: "Send" })).toHaveClass("rex-btn--primary");
  });

  it("calls onClick when enabled", async () => {
    const user = userEvent.setup();
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Go</Button>);
    await user.click(screen.getByRole("button", { name: "Go" }));
    expect(onClick).toHaveBeenCalledOnce();
  });
});

describe("SegmentedControl", () => {
  it("switches mode", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <SegmentedControl
        value="agent"
        options={[
          { value: "agent", label: "Agent" },
          { value: "ask", label: "Ask" },
        ]}
        onChange={onChange}
      />
    );
    await user.click(screen.getByRole("button", { name: "Ask" }));
    expect(onChange).toHaveBeenCalledWith("ask");
  });
});
