import * as React from "react";

import type { PlanArtifactPayload } from "../../src/shared/messages";

import { parseClarifyQuestions } from "../../src/runtime/planContent";

export interface PlanCardProps {
  readonly artifact: PlanArtifactPayload;
  readonly panel?: boolean;
  readonly onContentChange: (content: string) => void;
  readonly onSavePathChange: (path: string) => void;
  readonly onSave: () => void;
  readonly onBuild: () => void;
}

export function PlanCard(props: PlanCardProps): React.ReactElement {
  const questions =
    props.artifact.phase === "clarify" ? parseClarifyQuestions(props.artifact.detail) : [];
  const canHandoff = props.artifact.phase === "ready";

  return (
    <section
      className={props.panel === true ? "rex-plan-card rex-plan-card--panel" : "rex-plan-card"}
      aria-label="Plan review"
    >
      <header className="rex-plan-card__header">
        <span className="rex-plan-card__phase">{props.artifact.phase}</span>
        <h3 className="rex-plan-card__title">{props.artifact.title}</h3>
      </header>

      {props.artifact.phase === "clarify" ? (
        <div className="rex-plan-card__clarify">
          {questions.length === 0 ? (
            <p className="rex-plan-card__hint">Clarifying questions will appear here.</p>
          ) : (
            <ul>
              {questions.map((question) => (
                <li key={question.id}>
                  <strong>{question.prompt}</strong>
                  {question.options !== undefined && question.options.length > 0 ? (
                    <ul>
                      {question.options.map((option) => (
                        <li key={option}>{option}</li>
                      ))}
                    </ul>
                  ) : null}
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : (
        <>
          <label className="rex-plan-card__path-label">
            Save path
            <input
              className="rex-plan-card__path"
              type="text"
              value={props.artifact.savePath}
              onChange={(event) => props.onSavePathChange(event.target.value)}
              aria-label="Plan save path"
            />
          </label>
          <textarea
            className="rex-plan-card__editor"
            value={props.artifact.content}
            onChange={(event) => props.onContentChange(event.target.value)}
            aria-label="Plan content"
            rows={12}
          />
        </>
      )}

      {canHandoff ? (
        <div className="rex-plan-card__actions">
          <button type="button" onClick={props.onSave} aria-label="Save plan to workspace">
            Save
          </button>
          <button type="button" onClick={props.onBuild} aria-label="Build plan in agent mode">
            Build
          </button>
        </div>
      ) : null}
    </section>
  );
}
