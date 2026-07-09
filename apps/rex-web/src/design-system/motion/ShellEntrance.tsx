import type { ReactNode } from "react";

interface Props {
  children: ReactNode;
}

export function ShellEntrance({ children }: Props) {
  return (
    <div className="rex-shell-entrance" data-shell-revealed="yes">
      {children}
    </div>
  );
}
