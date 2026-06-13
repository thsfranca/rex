import * as React from "react";

export function SendIcon(): React.ReactElement {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path
        d="M8 2.5L13.5 8L8 13.5V9.5H3V6.5H8V2.5Z"
        fill="currentColor"
      />
    </svg>
  );
}

export function StopIcon(): React.ReactElement {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <rect x="4" y="4" width="8" height="8" rx="1" fill="currentColor" />
    </svg>
  );
}
