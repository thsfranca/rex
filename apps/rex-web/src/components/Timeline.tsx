import { Text } from "../design-system";
import type { TimelineTask, TurnPhase } from "../types";
import { HairlineFlux } from "./HairlineFlux";
import { MotionTimelineItem } from "./Motion";

interface Props {
  tasks: TimelineTask[];
  phase: TurnPhase;
}

export function Timeline({ tasks, phase }: Props) {
  return (
    <div className="timeline-panel" data-testid="timeline">
      <HairlineFlux phase={phase} testId="timeline-hairline" />
      <div className="timeline-panel__rail" aria-hidden>
        {tasks.slice(0, 5).map((task) => (
          <span key={task.id} className="timeline-panel__dot" title={task.label} />
        ))}
      </div>
      <div className="timeline-panel__body">
        <Text as="p" tone="secondary" style={{ marginTop: 0 }}>
          Timeline
        </Text>
        {tasks.length === 0 ? (
          <Text tone="secondary">{phase === "generating" ? "Working…" : "No tasks"}</Text>
        ) : (
          <ul style={{ paddingLeft: "1rem", margin: 0 }}>
            {tasks.map((task, index) => (
              <MotionTimelineItem key={task.id} index={index}>
                <div className="timeline-row">
                  <span>{task.label}</span>
                  <div className="timeline-row__detail">{task.id}</div>
                </div>
              </MotionTimelineItem>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
