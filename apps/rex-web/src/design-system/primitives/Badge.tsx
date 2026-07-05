import type { ReactNode } from "react";

export interface BadgeProps {
  children: ReactNode;
  testId?: string;
}

export function Badge({ children, testId }: BadgeProps) {
  return (
    <span className="rex-badge" data-testid={testId}>
      {children}
    </span>
  );
}
