import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CreateWorkspaceModal } from "./CreateWorkspaceModal";
import { pickWorkspaceDirectory, workspaceDirectoryExists } from "../lib/desktopClient";

vi.mock("../lib/desktopClient", () => ({
  pickWorkspaceDirectory: vi.fn(),
  workspaceDirectoryExists: vi.fn(),
}));

describe("CreateWorkspaceModal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(pickWorkspaceDirectory).mockResolvedValue(null);
    vi.mocked(workspaceDirectoryExists).mockResolvedValue(true);
  });

  it("suggests the workspace name from the directory until the user edits it", async () => {
    const user = userEvent.setup();

    render(
      <CreateWorkspaceModal
        isOpen
        onClose={() => {}}
        onCreate={() => {}}
        error={null}
        defaultRootDir="D:\\dev\\starter"
        defaultShellProfile="cmd.exe"
      />,
    );

    const nameInput = screen.getByLabelText("Name");
    const directoryInput = screen.getByLabelText("Directory");

    expect((nameInput as HTMLInputElement).value).toBe("starter");

    await user.clear(directoryInput);
    await user.type(directoryInput, "D:\\dev\\api");
    expect((nameInput as HTMLInputElement).value).toBe("api");

    await user.clear(nameInput);
    await user.type(nameInput, "manual-name");
    await user.clear(directoryInput);
    await user.type(directoryInput, "D:\\dev\\web");

    expect((nameInput as HTMLInputElement).value).toBe("manual-name");
  });

  it("uses the native folder picker and submits a custom shell profile on Enter", async () => {
    const user = userEvent.setup();
    const onCreate = vi.fn().mockResolvedValue(undefined);
    const onClose = vi.fn();
    vi.mocked(pickWorkspaceDirectory).mockResolvedValue("D:\\dev\\api");

    render(
      <CreateWorkspaceModal
        isOpen
        onClose={onClose}
        onCreate={onCreate}
        error={null}
        defaultRootDir="D:\\dev\\starter"
        defaultShellProfile="cmd.exe"
      />,
    );

    await user.click(screen.getByRole("button", { name: "Browse" }));

    await waitFor(() => {
      expect((screen.getByLabelText("Directory") as HTMLInputElement).value).toBe("D:\\dev\\api");
    });
    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe("api");

    await user.selectOptions(screen.getByLabelText("Shell preset"), "custom");
    const customShellInput = screen.getByLabelText("Custom shell");
    await user.clear(customShellInput);
    await user.type(customShellInput, "C:\\Program Files\\Git\\bin\\bash.exe{enter}");

    await waitFor(() => {
      expect(workspaceDirectoryExists).toHaveBeenCalledWith("D:\\dev\\api");
    });
    expect(onCreate).toHaveBeenCalledWith({
      name: "api",
      rootDir: "D:\\dev\\api",
      shellProfile: "C:\\Program Files\\Git\\bin\\bash.exe",
    });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("shows a validation error and blocks submit when the directory is missing", async () => {
    const user = userEvent.setup();
    const onCreate = vi.fn();
    vi.mocked(workspaceDirectoryExists).mockResolvedValue(false);

    render(
      <CreateWorkspaceModal
        isOpen
        onClose={() => {}}
        onCreate={onCreate}
        error={null}
        defaultRootDir="D:\\dev\\missing"
        defaultShellProfile="pwsh"
      />,
    );

    await user.click(screen.getByRole("button", { name: "Create" }));

    expect(await screen.findByText("Choose an existing folder before creating a workspace.")).toBeTruthy();
    expect(onCreate).not.toHaveBeenCalled();
  });

  it("closes on Escape", () => {
    const onClose = vi.fn();

    render(
      <CreateWorkspaceModal
        isOpen
        onClose={onClose}
        onCreate={() => {}}
        error={null}
        defaultRootDir="D:\\dev\\starter"
        defaultShellProfile="cmd.exe"
      />,
    );

    fireEvent.keyDown(screen.getByRole("dialog", { name: "New Workspace" }), { key: "Escape" });

    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
