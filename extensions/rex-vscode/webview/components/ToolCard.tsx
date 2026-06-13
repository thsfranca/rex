import * as React from "react";

export interface ToolCardProps {
  readonly phase: string;
  readonly summary: string;
  readonly kind?: string;
  readonly detail?: string;
}

export function ToolCard(props: ToolCardProps): React.ReactElement {
  const [open, setOpen] = React.useState(false);
  return (
    <details
      className="rex-tool-card"
      open={open}
      onToggle={(event) => setOpen((event.target as HTMLDetailsElement).open)}
    >
      <summary className="rex-tool-card__summary">
        <span className="rex-tool-card__phase">{props.phase}</span>
        <span className="rex-tool-card__label">{props.summary}</span>
      </summary>
      {props.detail !== undefined && props.detail.length > 0 ? (
        <pre className="rex-tool-card__detail">{props.detail}</pre>
      ) : null}
    </details>
  );
}
