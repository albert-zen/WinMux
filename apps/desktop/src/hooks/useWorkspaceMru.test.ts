import { describe, expect, it, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useWorkspaceMru } from "./useWorkspaceMru";

const MRU_KEY = "cmux.workspaceMru";

describe("useWorkspaceMru", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("returns empty array initially", () => {
    const { result } = renderHook(() => useWorkspaceMru());
    expect(result.current.mru).toEqual([]);
  });

  it("loads initial MRU from localStorage", () => {
    localStorage.setItem(MRU_KEY, JSON.stringify(["ws-1", "ws-2"]));
    const { result } = renderHook(() => useWorkspaceMru());
    expect(result.current.mru).toEqual(["ws-1", "ws-2"]);
  });

  it("updates MRU when touching a workspace", () => {
    const { result } = renderHook(() => useWorkspaceMru());

    act(() => {
      result.current.touchWorkspace("ws-1");
    });

    expect(result.current.mru).toEqual(["ws-1"]);

    act(() => {
      result.current.touchWorkspace("ws-2");
    });

    expect(result.current.mru).toEqual(["ws-2", "ws-1"]);
  });

  it("moves existing workspace to front when touched again", () => {
    const { result } = renderHook(() => useWorkspaceMru());

    act(() => {
      result.current.touchWorkspace("ws-1");
    });
    act(() => {
      result.current.touchWorkspace("ws-2");
    });
    act(() => {
      result.current.touchWorkspace("ws-3");
    });

    expect(result.current.mru).toEqual(["ws-3", "ws-2", "ws-1"]);

    act(() => {
      result.current.touchWorkspace("ws-1");
    });

    expect(result.current.mru).toEqual(["ws-1", "ws-3", "ws-2"]);
  });

  it("persists MRU to localStorage", () => {
    const { result } = renderHook(() => useWorkspaceMru());

    act(() => {
      result.current.touchWorkspace("ws-1");
    });
    act(() => {
      result.current.touchWorkspace("ws-2");
    });

    const stored = localStorage.getItem(MRU_KEY);
    expect(stored).toBe(JSON.stringify(["ws-2", "ws-1"]));
  });

  it("removes workspaces from MRU", () => {
    const { result } = renderHook(() => useWorkspaceMru());

    act(() => {
      result.current.touchWorkspace("ws-1");
    });
    act(() => {
      result.current.touchWorkspace("ws-2");
    });

    expect(result.current.mru).toEqual(["ws-2", "ws-1"]);

    act(() => {
      result.current.removeWorkspace("ws-1");
    });

    expect(result.current.mru).toEqual(["ws-2"]);
  });

  it("handles remove on non-existent workspace", () => {
    const { result } = renderHook(() => useWorkspaceMru());

    act(() => {
      result.current.touchWorkspace("ws-1");
    });

    act(() => {
      result.current.removeWorkspace("ws-999");
    });

    expect(result.current.mru).toEqual(["ws-1"]);
  });

  it("gets next workspace in MRU order", () => {
    const { result } = renderHook(() => useWorkspaceMru());

    act(() => {
      result.current.touchWorkspace("ws-1");
    });
    act(() => {
      result.current.touchWorkspace("ws-2");
    });
    act(() => {
      result.current.touchWorkspace("ws-3");
    });

    // MRU is ["ws-3", "ws-2", "ws-1"]
    expect(result.current.getNextInMru("ws-3")).toBe("ws-2");
    expect(result.current.getNextInMru("ws-2")).toBe("ws-1");
    expect(result.current.getNextInMru("ws-1")).toBe("ws-3"); // Wraps around
  });

  it("gets previous workspace in MRU order", () => {
    const { result } = renderHook(() => useWorkspaceMru());

    act(() => {
      result.current.touchWorkspace("ws-1");
    });
    act(() => {
      result.current.touchWorkspace("ws-2");
    });
    act(() => {
      result.current.touchWorkspace("ws-3");
    });

    // MRU is ["ws-3", "ws-2", "ws-1"]
    expect(result.current.getPreviousInMru("ws-3")).toBe("ws-1"); // Wraps around
    expect(result.current.getPreviousInMru("ws-2")).toBe("ws-3");
    expect(result.current.getPreviousInMru("ws-1")).toBe("ws-2");
  });

  it("returns null for getNextInMru with unknown workspace", () => {
    const { result } = renderHook(() => useWorkspaceMru());

    act(() => {
      result.current.touchWorkspace("ws-1");
    });

    expect(result.current.getNextInMru("ws-999")).toBeNull();
  });

  it("returns null when MRU is empty", () => {
    const { result } = renderHook(() => useWorkspaceMru());

    expect(result.current.getNextInMru("ws-1")).toBeNull();
    expect(result.current.getPreviousInMru("ws-1")).toBeNull();
  });

  it("returns same workspace when only one in MRU", () => {
    const { result } = renderHook(() => useWorkspaceMru());

    act(() => {
      result.current.touchWorkspace("ws-1");
    });

    expect(result.current.getNextInMru("ws-1")).toBe("ws-1");
    expect(result.current.getPreviousInMru("ws-1")).toBe("ws-1");
  });
});
