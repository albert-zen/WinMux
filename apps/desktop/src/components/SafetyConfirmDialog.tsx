import { useEffect, useMemo, useRef } from "react";

interface SafetyConfirmDialogProps {
  isOpen: boolean;
  title: string;
  description: string;
  confirmLabel: string;
  cancelLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function SafetyConfirmDialog({
  isOpen,
  title,
  description,
  confirmLabel,
  cancelLabel = "Cancel",
  onConfirm,
  onCancel,
}: SafetyConfirmDialogProps) {
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const previouslyFocusedRef = useRef<HTMLElement | null>(null);
  const focusableSelector = useMemo(
    () =>
      [
        "button:not([disabled])",
        "[href]",
        "input:not([disabled])",
        "select:not([disabled])",
        "textarea:not([disabled])",
        "[tabindex]:not([tabindex='-1'])",
      ].join(", "),
    [],
  );

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    previouslyFocusedRef.current = document.activeElement as HTMLElement | null;
    const focusable = dialogRef.current?.querySelectorAll<HTMLElement>(focusableSelector) ?? [];
    const initialTarget = focusable[0] ?? dialogRef.current;
    initialTarget?.focus();

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        onCancel();
        return;
      }

      if (event.key !== "Tab") {
        return;
      }

      const dialog = dialogRef.current;
      if (!dialog) {
        return;
      }

      const targets = Array.from(dialog.querySelectorAll<HTMLElement>(focusableSelector));
      if (targets.length === 0) {
        event.preventDefault();
        dialog.focus();
        return;
      }

      const first = targets[0];
      const last = targets[targets.length - 1];
      const active = document.activeElement as HTMLElement | null;

      if (!dialog.contains(active)) {
        event.preventDefault();
        (event.shiftKey ? last : first).focus();
        return;
      }

      event.preventDefault();
      const currentIndex = Math.max(targets.indexOf(active ?? first), 0);
      const nextIndex = event.shiftKey
        ? (currentIndex - 1 + targets.length) % targets.length
        : (currentIndex + 1) % targets.length;
      targets[nextIndex]?.focus();
    };

    document.addEventListener("keydown", handleKeyDown, true);
    return () => {
      document.removeEventListener("keydown", handleKeyDown, true);
      previouslyFocusedRef.current?.focus();
    };
  }, [focusableSelector, isOpen, onCancel]);

  if (!isOpen) {
    return null;
  }

  return (
    <div className="confirm-dialog-overlay">
      <div
        aria-label={title}
        aria-modal="true"
        className="confirm-dialog"
        role="dialog"
        ref={dialogRef}
        tabIndex={-1}
      >
        <h3 className="confirm-dialog-title">{title}</h3>
        <p className="confirm-dialog-message">{description}</p>
        <div className="confirm-dialog-actions">
          <button type="button" className="btn-primary" onClick={onConfirm}>
            {confirmLabel}
          </button>
          <button type="button" className="btn-secondary" onClick={onCancel}>
            {cancelLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
