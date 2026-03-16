import { describe, expect, it } from "vitest";
import {
  APP_NAME,
  METADATA_REFRESH_POLICY,
  PROTOCOL_VERSION,
  STARTER_PANE_ID,
  STARTER_WORKSPACE_NAME,
  applyScrollbackCap,
  createStarterWorkspaceSnapshot
} from "./index";

describe("protocol constants", () => {
  it("exposes the repository app name", () => {
    expect(APP_NAME).toBe("cmux-win");
  });

  it("starts on protocol version one", () => {
    expect(PROTOCOL_VERSION).toBe(1);
  });

  it("describes the default workspace snapshot contract", () => {
    const snapshot = createStarterWorkspaceSnapshot();

    expect(snapshot.layout.paneCount).toBe(1);
    expect(snapshot.layout.splitCount).toBe(0);
    expect(snapshot.restore.lastFocusedPaneId).toBe(STARTER_PANE_ID);
    expect(snapshot.scrollback).toEqual([]);
  });

  it("caps scrollback from the end without changing restore metadata", () => {
    const snapshot = createStarterWorkspaceSnapshot();
    const capped = applyScrollbackCap(
      {
        ...snapshot,
        scrollback: ["one", "two", "three", "four"]
      },
      2
    );

    expect(capped.scrollback).toEqual(["three", "four"]);
    expect(capped.restore.lastFocusedPaneId).toBe(STARTER_PANE_ID);
  });

  it("declares hybrid metadata refresh as the starter policy", () => {
    expect(METADATA_REFRESH_POLICY.strategy).toBe("hybrid");
    expect(METADATA_REFRESH_POLICY.fallbackIntervalMs).toBeGreaterThan(0);
    expect(STARTER_WORKSPACE_NAME).toBe("inbox");
  });
});
