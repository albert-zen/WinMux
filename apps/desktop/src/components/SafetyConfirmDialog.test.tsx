import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SafetyConfirmDialog } from "./SafetyConfirmDialog";

describe("SafetyConfirmDialog", () => {
  it("renders when open", () => {
    render(
      <SafetyConfirmDialog
        isOpen={true}
        title="Close pane p1?"
        description="This will stop the session."
        confirmLabel="Confirm close pane"
        onConfirm={() => {}}
        onCancel={() => {}}
      />,
    );

    expect(screen.getByRole("dialog", { name: "Close pane p1?" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Confirm close pane" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeTruthy();
  });

  it("calls onCancel when Escape is pressed", () => {
    const onCancel = vi.fn();
    render(
      <SafetyConfirmDialog
        isOpen={true}
        title="Close pane p1?"
        description="This will stop the session."
        confirmLabel="Confirm close pane"
        onConfirm={() => {}}
        onCancel={onCancel}
      />,
    );

    fireEvent.keyDown(document, { key: "Escape" });

    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("keeps Tab navigation trapped inside the dialog", () => {
    render(
      <div>
        <button type="button">Outside action</button>
        <SafetyConfirmDialog
          isOpen={true}
          title="Close pane p1?"
          description="This will stop the session."
          confirmLabel="Confirm close pane"
          onConfirm={() => {}}
          onCancel={() => {}}
        />
      </div>,
    );

    const confirmButton = screen.getByRole("button", { name: "Confirm close pane" });
    const cancelButton = screen.getByRole("button", { name: "Cancel" });

    expect(document.activeElement).toBe(confirmButton);

    fireEvent.keyDown(document, { key: "Tab" });
    expect(document.activeElement).toBe(cancelButton);

    fireEvent.keyDown(document, { key: "Tab" });
    expect(document.activeElement).toBe(confirmButton);

    fireEvent.keyDown(document, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(cancelButton);
  });
});
