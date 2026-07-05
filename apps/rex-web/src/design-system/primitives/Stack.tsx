import type { CSSProperties, HTMLAttributes, ReactNode } from "react";

type Direction = "row" | "column";

export interface StackProps extends HTMLAttributes<HTMLDivElement> {
  direction?: Direction;
  gap?: "xs" | "sm" | "md" | "lg" | "xl";
  align?: CSSProperties["alignItems"];
  justify?: CSSProperties["justifyContent"];
  children: ReactNode;
}

const gapToken: Record<NonNullable<StackProps["gap"]>, string> = {
  xs: "var(--rex-space-xs)",
  sm: "var(--rex-space-sm)",
  md: "var(--rex-space-md)",
  lg: "var(--rex-space-lg)",
  xl: "var(--rex-space-xl)",
};

export function Stack({
  direction = "column",
  gap = "md",
  align,
  justify,
  style,
  className,
  children,
  ...rest
}: StackProps) {
  const stackStyle: CSSProperties = {
    display: "flex",
    flexDirection: direction,
    gap: gapToken[gap],
    alignItems: align,
    justifyContent: justify,
    ...style,
  };
  const classes = className ? `rex-stack ${className}` : "rex-stack";
  return (
    <div className={classes} style={stackStyle} {...rest}>
      {children}
    </div>
  );
}

export function Grid({
  className,
  style,
  children,
  ...rest
}: HTMLAttributes<HTMLDivElement> & { children: ReactNode }) {
  const classes = className ? `rex-grid ${className}` : "rex-grid";
  return (
    <div className={classes} style={style} {...rest}>
      {children}
    </div>
  );
}
