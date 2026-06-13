import * as React from "react";

import type { FileEditPreview } from "../../src/shared/messages";
import { computeLineDiff, diffStats } from "../diff/lineDiff";

export interface EditDiffPreviewProps {
  readonly edit: FileEditPreview;
}

export function EditDiffPreview(props: EditDiffPreviewProps): React.ReactElement {
  const lines = React.useMemo(
    () => computeLineDiff(props.edit.before, props.edit.after),
    [props.edit.before, props.edit.after],
  );
  const stats = React.useMemo(() => diffStats(lines), [lines]);

  return (
    <div className="rex-edit-diff" role="region" aria-label={`Diff for ${props.edit.filePath}`}>
      <div className="rex-edit-diff__header">
        <span className="rex-edit-diff__path" title={props.edit.filePath}>
          {props.edit.filePath}
        </span>
        {props.edit.languageId !== undefined ? (
          <span className="rex-edit-diff__lang">{props.edit.languageId}</span>
        ) : null}
        <span className="rex-edit-diff__stats" aria-label="Diff statistics">
          <span className="rex-edit-diff__stat rex-edit-diff__stat--add">+{stats.added}</span>
          <span className="rex-edit-diff__stat rex-edit-diff__stat--remove">−{stats.removed}</span>
        </span>
      </div>
      <div className="rex-edit-diff__body" tabIndex={0}>
        <table className="rex-edit-diff__table">
          <tbody>
            {lines.map((line, index) => (
              <tr
                key={`${line.kind}-${index}-${line.oldLineNumber ?? "n"}-${line.newLineNumber ?? "n"}`}
                className={`rex-edit-diff__row rex-edit-diff__row--${line.kind}`}
              >
                <td className="rex-edit-diff__gutter rex-edit-diff__gutter--old" aria-hidden="true">
                  {line.oldLineNumber ?? ""}
                </td>
                <td className="rex-edit-diff__gutter rex-edit-diff__gutter--new" aria-hidden="true">
                  {line.newLineNumber ?? ""}
                </td>
                <td className="rex-edit-diff__sign" aria-hidden="true">
                  {line.kind === "add" ? "+" : line.kind === "remove" ? "−" : " "}
                </td>
                <td className="rex-edit-diff__code">
                  <code>{line.text.length > 0 ? line.text : " "}</code>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
