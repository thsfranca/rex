export interface StatusDotProps {
  working?: boolean;
  error?: boolean;
  id?: string;
  testId?: string;
}

export function StatusDot({ working = false, error = false, id, testId }: StatusDotProps) {
  const stateClass = error ? "rex-status-dot--error" : working ? "rex-status-dot--working" : "rex-status-dot--idle";
  return (
    <span
      id={id}
      data-testid={testId}
      data-motion-tier={working ? "ambient" : "idle"}
      className={`rex-status-dot ${stateClass}`}
      aria-hidden="true"
    />
  );
}
