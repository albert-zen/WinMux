interface InlineFeedbackProps {
  message: string;
  tone?: "error" | "info";
  className?: string;
  role?: "status" | "alert";
  testId?: string;
  dismissLabel?: string;
  dismissTestId?: string;
  onDismiss?: () => void;
}

export function InlineFeedback({
  message,
  tone = "error",
  className,
  role = "status",
  testId,
  dismissLabel,
  dismissTestId,
  onDismiss,
}: InlineFeedbackProps) {
  return (
    <div
      className={`inline-feedback inline-feedback--${tone}${className ? ` ${className}` : ""}`}
      data-testid={testId}
      role={role}
    >
      <span>{message}</span>
      {onDismiss && dismissLabel ? (
        <button
          type="button"
          aria-label={dismissLabel}
          className="inline-feedback__dismiss"
          data-testid={dismissTestId}
          onClick={(event) => {
            event.stopPropagation();
            onDismiss();
          }}
        >
          ×
        </button>
      ) : null}
    </div>
  );
}
