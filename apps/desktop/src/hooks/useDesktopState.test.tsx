import { render, screen } from "@testing-library/react";
import { act } from "react";
import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { useDesktopState } from "./useDesktopState";

function HookProbe() {
  const { state, error } = useDesktopState();

  return (
    <div>
      <span data-testid="state">{state ? JSON.stringify(state) : "null"}</span>
      <span data-testid="error">{error ?? ""}</span>
    </div>
  );
}

async function flushMicrotasks() {
  await act(async () => {
    await Promise.resolve();
  });
}

describe("useDesktopState", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("invokes desktop_state on mount", async () => {
    vi.mocked(invoke).mockResolvedValue({
      protocolVersion: 1,
      workspaces: [],
    });

    render(<HookProbe />);
    await flushMicrotasks();

    expect(invoke).toHaveBeenCalledWith("desktop_state");
  });

  it("polls desktop_state on the refresh interval", async () => {
    vi.mocked(invoke).mockResolvedValue({
      protocolVersion: 1,
      workspaces: [],
    });

    render(<HookProbe />);
    await flushMicrotasks();

    expect(invoke).toHaveBeenCalledTimes(1);

    await act(async () => {
      vi.advanceTimersByTime(5_000);
    });
    await flushMicrotasks();

    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it("surfaces invoke errors", async () => {
    vi.mocked(invoke).mockRejectedValue(new Error("desktop state failed"));

    render(<HookProbe />);
    await flushMicrotasks();

    expect(screen.getByTestId("error").textContent).toContain("desktop state failed");
  });

  it("stops polling after unmount", async () => {
    vi.mocked(invoke).mockResolvedValue({
      protocolVersion: 1,
      workspaces: [],
    });

    const view = render(<HookProbe />);
    await flushMicrotasks();
    expect(invoke).toHaveBeenCalledTimes(1);

    view.unmount();

    await act(async () => {
      vi.advanceTimersByTime(10_000);
    });
    await flushMicrotasks();

    expect(invoke).toHaveBeenCalledTimes(1);
  });
});
