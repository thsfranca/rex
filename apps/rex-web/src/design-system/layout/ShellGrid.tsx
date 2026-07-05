import type { ReactNode } from "react";

export interface ShellGridProps {
  header: ReactNode;
  transcript: ReactNode;
  timeline: ReactNode;
  composer: ReactNode;
  footer: ReactNode;
  testId?: string;
}

export function ShellGrid({
  header,
  transcript,
  timeline,
  composer,
  footer,
  testId = "shell",
}: ShellGridProps) {
  return (
    <div className="rex-shell shell" data-testid={testId}>
      <header className="rex-shell__header header" data-testid="header">
        {header}
      </header>
      <main className="rex-shell__transcript transcript">{transcript}</main>
      <aside className="rex-shell__timeline timeline">{timeline}</aside>
      <div className="rex-shell__composer composer">{composer}</div>
      <footer className="rex-shell__footer footer">{footer}</footer>
    </div>
  );
}
