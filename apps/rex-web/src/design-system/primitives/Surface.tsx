import type { HTMLAttributes, ReactNode } from "react";

type Elevation = "base" | "raised" | "overlay";

export interface SurfaceProps extends HTMLAttributes<HTMLDivElement> {
  elevation?: Elevation;
  children: ReactNode;
}

const elevationClass: Record<Elevation, string> = {
  base: "rex-surface rex-surface--base",
  raised: "rex-surface rex-surface--raised",
  overlay: "rex-surface rex-surface--overlay",
};

export function Surface({
  elevation = "raised",
  className,
  children,
  ...rest
}: SurfaceProps) {
  const classes = className
    ? `${elevationClass[elevation]} ${className}`
    : elevationClass[elevation];
  return (
    <div className={classes} {...rest}>
      {children}
    </div>
  );
}

export function Hairline({ className, ...rest }: HTMLAttributes<HTMLHRElement>) {
  const classes = className ? `rex-hairline ${className}` : "rex-hairline";
  return <hr className={classes} {...rest} />;
}
