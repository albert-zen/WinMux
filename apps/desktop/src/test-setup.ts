import { afterEach, vi } from "vitest";
import { cleanup } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
