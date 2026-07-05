import type { HTMLAttributes, ReactNode } from "react";

type Tone = "primary" | "secondary" | "success" | "error";

export interface TextProps extends HTMLAttributes<HTMLSpanElement> {
  as?: "span" | "p" | "label";
  tone?: Tone;
  mono?: boolean;
  bold?: boolean;
  children: ReactNode;
}

const toneClass: Record<Tone, string> = {
  primary: "rex-text rex-text--primary",
  secondary: "rex-text rex-text--secondary",
  success: "rex-text rex-text--success",
  error: "rex-text rex-text--error",
};

export function Text({
  as: Tag = "span",
  tone = "primary",
  mono = false,
  bold = false,
  className,
  children,
  ...rest
}: TextProps) {
  const parts = [toneClass[tone]];
  if (mono) parts.push("rex-text--mono");
  if (bold) parts.push("rex-text--bold");
  if (className) parts.push(className);
  return (
    <Tag className={parts.join(" ")} {...rest}>
      {children}
    </Tag>
  );
}

export function Heading({
  level = 2,
  className,
  children,
  ...rest
}: {
  level?: 1 | 2 | 3;
  className?: string;
  children: ReactNode;
} & HTMLAttributes<HTMLHeadingElement>) {
  const Tag = `h${level}` as "h1" | "h2" | "h3";
  const classes = className ? `rex-heading rex-heading--${level} ${className}` : `rex-heading rex-heading--${level}`;
  return (
    <Tag className={classes} {...rest}>
      {children}
    </Tag>
  );
}
