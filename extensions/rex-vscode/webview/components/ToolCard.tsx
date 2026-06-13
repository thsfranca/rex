import * as React from "react";

import { ExecutionStateIcon } from "./ExecutionStateIcon";
import {
  formatExecutionLabel,
  shouldShowExecutionDetail,
  type ExecutionLabelInput,
} from "../timeline/executionLabel";

export interface ToolCardProps extends ExecutionLabelInput {
  readonly id: string;
}

export function ToolCard(props: ToolCardProps): React.ReactElement {
  const label = formatExecutionLabel(props);
  const showDetail = shouldShowExecutionDetail(props.summary, props.detail);
  const [open, setOpen] = React.useState(false);

  return (
    <div className={`rex-exec-log rex-exec-log--${props.phase}`} role="listitem">
      <span className="rex-exec-log__icon" aria-hidden="true">
        <ExecutionStateIcon phase={props.phase} />
      </span>
      <div className="rex-exec-log__body">
        <div className="rex-exec-log__row">
          <span className="rex-exec-log__label">{label}</span>
          {showDetail ? (
            <button
              type="button"
              className="rex-exec-log__toggle"
              aria-expanded={open}
              onClick={() => setOpen((value) => !value)}
            >
              {open ? "Hide" : "Show"}
            </button>
          ) : null}
        </div>
        {showDetail && open ? (
          <pre className="rex-exec-log__detail">{props.detail}</pre>
        ) : null}
      </div>
    </div>
  );
}
