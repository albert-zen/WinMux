import { useState } from "react";

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (config: { name: string; rootDir: string; shellProfile: string }) => void;
  error: string | null;
  defaultRootDir: string;
  defaultShellProfile: string;
}

export function CreateWorkspaceModal({
  isOpen,
  onClose,
  onCreate,
  error,
  defaultRootDir,
  defaultShellProfile,
}: Props) {
  const [name, setName] = useState("");
  const [rootDir, setRootDir] = useState(defaultRootDir);
  const [shellProfile, setShellProfile] = useState(defaultShellProfile);

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmedName = name.trim();
    const trimmedRootDir = rootDir.trim();
    const trimmedShellProfile = shellProfile.trim();
    if (!trimmedName || !trimmedRootDir || !trimmedShellProfile) return;
    onCreate({ name: trimmedName, rootDir: trimmedRootDir, shellProfile: trimmedShellProfile });
    setName("");
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      onClose();
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal-content"
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
              onChange={(e) => setName(e.target.value)}
              placeholder="my-project"
            />
          </label>
          <label>
            <span>Directory</span>
            <input
              value={rootDir}
              onChange={(e) => setRootDir(e.target.value)}
              placeholder="/path/to/project"
            />
          </label>
          <label>
            <span>Shell</span>
            <input
              value={shellProfile}
              onChange={(e) => setShellProfile(e.target.value)}
              placeholder="cmd.exe"
            />
          </label>
          <div className="modal-actions">
            <button type="button" className="btn-secondary" onClick={onClose}>
              Cancel
            </button>
            <button type="submit" className="btn-primary" disabled={!name.trim()}>
              Create
            </button>
          </div>
          {error ? <p className="modal-error">{error}</p> : null}
        </form>
      </div>
    </div>
  );
}
