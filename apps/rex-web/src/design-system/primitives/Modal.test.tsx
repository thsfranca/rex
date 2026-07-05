import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Modal } from "./Modal";

describe("Modal", () => {
  it("renders when open", () => {
    render(
      <Modal open title="Confirm" testId="test-modal">
        Body
      </Modal>
    );
    expect(screen.getByTestId("test-modal")).toHaveClass("rex-modal-backdrop--open");
    expect(screen.getByText("Confirm")).toBeInTheDocument();
  });

  it("hides when closed", () => {
    render(
      <Modal open={false} title="Confirm" testId="closed-modal">
        Body
      </Modal>
    );
    expect(screen.queryByTestId("closed-modal")).not.toBeInTheDocument();
  });
});
