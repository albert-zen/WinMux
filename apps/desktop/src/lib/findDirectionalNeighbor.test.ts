import { describe, it, expect } from "vitest";
import {
  findDirectionalNeighbor,
  type Direction,
} from "../lib/findDirectionalNeighbor";
import type { LayoutNode } from "@cmux-win/protocol";

describe("findDirectionalNeighbor", () => {
  // Helper to create a simple vertical split
  function createVerticalSplit(
    firstPaneId: string,
    secondPaneId: string,
    ratio = 0.5,
  ): LayoutNode {
    return {
      type: "split",
      orientation: "vertical",
      ratio,
      first: { type: "pane", paneId: firstPaneId, sessionKind: "freshShell" },
      second: { type: "pane", paneId: secondPaneId, sessionKind: "freshShell" },
    };
  }

  // Helper to create a simple horizontal split
  function createHorizontalSplit(
    firstPaneId: string,
    secondPaneId: string,
    ratio = 0.5,
  ): LayoutNode {
    return {
      type: "split",
      orientation: "horizontal",
      ratio,
      first: { type: "pane", paneId: firstPaneId, sessionKind: "freshShell" },
      second: { type: "pane", paneId: secondPaneId, sessionKind: "freshShell" },
    };
  }

  describe("single pane", () => {
    it("returns null for all directions when there is only one pane", () => {
      const layout: LayoutNode = {
        type: "pane",
        paneId: "pane-1",
        sessionKind: "freshShell",
      };
      const directions: Direction[] = ["up", "down", "left", "right"];

      for (const dir of directions) {
        expect(findDirectionalNeighbor(layout, "pane-1", dir)).toBeNull();
      }
    });
  });

  describe("vertical split (two panes)", () => {
    it("finds right neighbor from left pane", () => {
      const layout = createVerticalSplit("pane-1", "pane-2");

      expect(findDirectionalNeighbor(layout, "pane-1", "right")).toBe("pane-2");
    });

    it("finds left neighbor from right pane", () => {
      const layout = createVerticalSplit("pane-1", "pane-2");

      expect(findDirectionalNeighbor(layout, "pane-2", "left")).toBe("pane-1");
    });

    it("returns null for up/down in vertical split", () => {
      const layout = createVerticalSplit("pane-1", "pane-2");

      expect(findDirectionalNeighbor(layout, "pane-1", "up")).toBeNull();
      expect(findDirectionalNeighbor(layout, "pane-1", "down")).toBeNull();
      expect(findDirectionalNeighbor(layout, "pane-2", "up")).toBeNull();
      expect(findDirectionalNeighbor(layout, "pane-2", "down")).toBeNull();
    });
  });

  describe("horizontal split (two panes)", () => {
    it("finds down neighbor from top pane", () => {
      const layout = createHorizontalSplit("pane-1", "pane-2");

      expect(findDirectionalNeighbor(layout, "pane-1", "down")).toBe("pane-2");
    });

    it("finds up neighbor from bottom pane", () => {
      const layout = createHorizontalSplit("pane-1", "pane-2");

      expect(findDirectionalNeighbor(layout, "pane-2", "up")).toBe("pane-1");
    });

    it("returns null for left/right in horizontal split", () => {
      const layout = createHorizontalSplit("pane-1", "pane-2");

      expect(findDirectionalNeighbor(layout, "pane-1", "left")).toBeNull();
      expect(findDirectionalNeighbor(layout, "pane-1", "right")).toBeNull();
      expect(findDirectionalNeighbor(layout, "pane-2", "left")).toBeNull();
      expect(findDirectionalNeighbor(layout, "pane-2", "right")).toBeNull();
    });
  });

  describe("nested splits", () => {
    it("finds neighbor across nested vertical splits", () => {
      // [pane-1 | [pane-2 | pane-3]]
      const innerSplit = createVerticalSplit("pane-2", "pane-3");
      const layout: LayoutNode = {
        type: "split",
        orientation: "vertical",
        ratio: 0.33,
        first: { type: "pane", paneId: "pane-1", sessionKind: "freshShell" },
        second: innerSplit,
      };

      // From pane-1, right should go to pane-2 (first leaf of inner split)
      expect(findDirectionalNeighbor(layout, "pane-1", "right")).toBe("pane-2");

      // From pane-2, left should go to pane-1
      expect(findDirectionalNeighbor(layout, "pane-2", "left")).toBe("pane-1");

      // From pane-3, left should go to pane-2
      expect(findDirectionalNeighbor(layout, "pane-3", "left")).toBe("pane-2");
    });

    it("finds neighbor across mixed orientation splits", () => {
      // [pane-1]
      // -------
      // [pane-2 | pane-3]
      const horizontalSplit: LayoutNode = {
        type: "split",
        orientation: "horizontal",
        ratio: 0.5,
        first: { type: "pane", paneId: "pane-1", sessionKind: "freshShell" },
        second: createVerticalSplit("pane-2", "pane-3"),
      };

      // From pane-1, down should go to pane-2 (first leaf of second)
      expect(findDirectionalNeighbor(horizontalSplit, "pane-1", "down")).toBe(
        "pane-2",
      );

      // From pane-2, up should go to pane-1
      expect(findDirectionalNeighbor(horizontalSplit, "pane-2", "up")).toBe(
        "pane-1",
      );
    });

    it("returns null when no neighbor in direction", () => {
      // [pane-1 | pane-2]
      // ----------------
      // [    pane-3    ]
      const topSplit = createVerticalSplit("pane-1", "pane-2");
      const layout: LayoutNode = {
        type: "split",
        orientation: "horizontal",
        ratio: 0.5,
        first: topSplit,
        second: { type: "pane", paneId: "pane-3", sessionKind: "freshShell" },
      };

      // pane-3 is at the bottom, no neighbor below
      expect(findDirectionalNeighbor(layout, "pane-3", "down")).toBeNull();

      // pane-1 is top-left, no neighbor above or left
      expect(findDirectionalNeighbor(layout, "pane-1", "up")).toBeNull();
      expect(findDirectionalNeighbor(layout, "pane-1", "left")).toBeNull();
    });
  });

  describe("edge cases", () => {
    it("returns null when focused pane not found", () => {
      const layout = createVerticalSplit("pane-1", "pane-2");

      expect(
        findDirectionalNeighbor(layout, "non-existent", "right"),
      ).toBeNull();
    });
  });
});
