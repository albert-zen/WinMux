import { type LayoutNode } from "@cmux-win/protocol";

export type Direction = "up" | "down" | "left" | "right";

/**
 * Finds the pane ID of the neighbor in the given direction from the focused pane.
 * Returns null if no neighbor exists in that direction.
 */
export function findDirectionalNeighbor(
  root: LayoutNode,
  focusedPaneId: string,
  direction: Direction,
): string | null {
  // Walk the tree to find the focused pane and track the path
  const path = findPathToPane(root, focusedPaneId);
  if (path === null) {
    return null;
  }

  // Walk back up the path looking for a split that aligns with the direction
  for (let i = path.length - 1; i >= 0; i--) {
    const step = path[i];
    if (step.node.type === "split") {
      const isVertical = step.node.orientation === "vertical";
      const wantsVertical = direction === "left" || direction === "right";
      const wantsForward = direction === "right" || direction === "down";

      // Only consider splits that align with the requested direction
      if (isVertical === wantsVertical) {
        // Check if we came from the first or second child
        const cameFromFirst = step.whichChild === "first";

        // If we want to go forward (right/down) and came from first, go to second
        // If we want to go backward (left/up) and came from second, go to first
        const shouldNavigate =
          (wantsForward && cameFromFirst) || (!wantsForward && !cameFromFirst);

        if (shouldNavigate) {
          const targetSubtree =
            wantsForward && cameFromFirst ? step.node.second : step.node.first;
          // Find the nearest pane in that subtree
          return findFirstLeaf(targetSubtree);
        }
      }
    }
  }

  return null;
}

type PathStep = {
  node: LayoutNode;
  whichChild: "first" | "second" | "root";
};

function findPathToPane(
  node: LayoutNode,
  targetPaneId: string,
  path: PathStep[] = [],
): PathStep[] | null {
  if (node.type === "pane") {
    if (node.paneId === targetPaneId) {
      return path;
    }
    return null;
  }

  // Try first child
  const firstPath = findPathToPane(node.first, targetPaneId, [
    ...path,
    { node, whichChild: "first" },
  ]);
  if (firstPath !== null) {
    return firstPath;
  }

  // Try second child
  const secondPath = findPathToPane(node.second, targetPaneId, [
    ...path,
    { node, whichChild: "second" },
  ]);
  return secondPath;
}

function findFirstLeaf(node: LayoutNode): string {
  if (node.type === "pane") {
    return node.paneId;
  }
  return findFirstLeaf(node.first);
}
