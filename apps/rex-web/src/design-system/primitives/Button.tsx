import type { ButtonHTMLAttributes, ReactNode } from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  children: ReactNode;
}

const variantClass: Record<Variant, string> = {
  primary: "rex-btn rex-btn--primary",
  secondary: "rex-btn rex-btn--secondary",
  ghost: "rex-btn rex-btn--ghost",
  danger: "rex-btn rex-btn--danger",
};

export function Button({
  variant = "primary",
  className,
  type = "button",
  children,
  ...rest
}: ButtonProps) {
  const classes = className ? `${variantClass[variant]} ${className}` : variantClass[variant];
  return (
    <button type={type} className={classes} {...rest}>
      {children}
    </button>
  );
}
