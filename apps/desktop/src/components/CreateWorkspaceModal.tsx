import { useEffect, useState } from "react";
import { pickWorkspaceDirectory, workspaceDirectoryExists } from "../lib/desktopClient";

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (config: {
    name: string;
    rootDir: string;
    shellProfile: string;
  }) => void | Promise<void>;
  error: string | null;
  defaultRootDir: string;
  defaultShellProfile: string;
}

const SHELL_PRESETS = ["cmd.exe", "pwsh", "bash"] as const;
type ShellPreset = (typeof SHELL_PRESETS)[number];
type ShellMode = ShellPreset | "custom";
const BROWSE_ERROR_MESSAGE = "Unable to open the folder picker. Enter a path manually or try again.";

function suggestWorkspaceName(path: string): string {
  const trimmed = path.trim().replace(/[\\/]+$/g, "");
  const parts = trimmed.split(/[\\/]/).filter(Boolean);
  return parts.at(-1) ?? "";
}

function getShellMode(shellProfile: string): ShellMode {
  return SHELL_PRESETS.includes(shellProfile as ShellPreset)
    ? (shellProfile as ShellPreset)
    : "custom";
}

function getBrowseErrorMessage(reason: unknown): string {
  if (reason instanceof Error && reason.message.trim()) {
    return reason.message;
  }

  if (typeof reason === "string" && reason.trim()) {
    return reason;
  }

  return BROWSE_ERROR_MESSAGE;
}

export function CreateWorkspaceModal({
  isOpen,
  onClose,
  onCreate,
  error,
  defaultRootDir,
  defaultShellProfile,
}: Props) {
  const [name, setName] = useState(() => suggestWorkspaceName(defaultRootDir));
  const [rootDir, setRootDir] = useState(defaultRootDir);
  const [shellMode, setShellMode] = useState<ShellMode>(() => getShellMode(defaultShellProfile));
  const [customShellProfile, setCustomShellProfile] = useState(() =>
    getShellMode(defaultShellProfile) === "custom" ? defaultShellProfile : "",
  );
  const [hasManualName, setHasManualName] = useState(false);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    setRootDir(defaultRootDir);
    setName(suggestWorkspaceName(defaultRootDir));
    setHasManualName(false);
    setShellMode(getShellMode(defaultShellProfile));
    setCustomShellProfile(getShellMode(defaultShellProfile) === "custom" ? defaultShellProfile : "");
    setValidationError(null);
    setIsSubmitting(false);
  }, [defaultRootDir, defaultShellProfile, isOpen]);

  const resolvedShellProfile = shellMode === "custom" ? customShellProfile : shellMode;

  if (!isOpen) return null;
  const submitDisabled =
    isSubmitting || !name.trim() || !rootDir.trim() || !resolvedShellProfile.trim();
  const visibleError = validationError ?? error;

  const handleRootDirChange = (nextRootDir: string) => {
    setRootDir(nextRootDir);
    setValidationError(null);
    if (!hasManualName) {
      setName(suggestWorkspaceName(nextRootDir));
    }
  };

  const handleBrowse = async () => {
    setValidationError(null);

    try {
      const selectedDir = await pickWorkspaceDirectory(rootDir.trim() || defaultRootDir);
      if (!selectedDir) {
        return;
      }

      handleRootDirChange(selectedDir);
    } catch (reason) {
      setValidationError(getBrowseErrorMessage(reason));
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmedName = name.trim();
    const trimmedRootDir = rootDir.trim();
    const trimmedShellProfile = resolvedShellProfile.trim();
    if (!trimmedName || !trimmedRootDir || !trimmedShellProfile) return;

    setValidationError(null);
    setIsSubmitting(true);

    try {
      const directoryExists = await workspaceDirectoryExists(trimmedRootDir);
      if (!directoryExists) {
        setValidationError("Choose an existing folder before creating a workspace.");
        return;
      }

      await onCreate({
        name: trimmedName,
        rootDir: trimmedRootDir,
        shellProfile: trimmedShellProfile,
      });
      onClose();
    } catch (reason) {
      setValidationError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal-content"
        role="dialog"
        aria-modal="true"
        aria-label="New Workspace"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <h2>New Workspace</h2>
        <form onSubmit={handleSubmit}>
          <label>
            <span>Name</span>
            <input
              autoFocus
              value={name}
              onChange={(e) => {
                setName(e.target.value);
                setHasManualName(true);
                setValidationError(null);
              }}
              placeholder="my-project"
            />
          </label>
          <label>
            <span>Directory</span>
            <div className="modal-directory-field">
              <input
                value={rootDir}
                onChange={(e) => handleRootDirChange(e.target.value)}
                placeholder="/path/to/project"
              />
              <button type="button" className="btn-secondary" onClick={() => void handleBrowse()}>
                Browse
              </button>
            </div>
          </label>
          <label>
            <span>Shell preset</span>
            <select
              aria-label="Shell preset"
              value={shellMode}
              onChange={(e) => {
                const nextMode = e.target.value as ShellMode;
                setShellMode(nextMode);
                setValidationError(null);
              }}
            >
              <option value="cmd.exe">cmd.exe</option>
              <option value="pwsh">pwsh</option>
              <option value="bash">bash</option>
              <option value="custom">Custom</option>
            </select>
          </label>
          {shellMode === "custom" ? (
            <label>
              <span>Custom shell</span>
              <input
                aria-label="Custom shell"
                value={customShellProfile}
                onChange={(e) => {
                  setCustomShellProfile(e.target.value);
                  setValidationError(null);
                }}
                placeholder="C:\\path\\to\\shell.exe"
              />
            </label>
          ) : null}
          <div className="modal-actions">
            <button type="button" className="btn-secondary" onClick={onClose}>
              Cancel
            </button>
            <button type="submit" className="btn-primary" disabled={submitDisabled}>
              Create
            </button>
          </div>
          {visibleError ? <p className="modal-error">{visibleError}</p> : null}
        </form>
      </div>
    </div>
  );
}
