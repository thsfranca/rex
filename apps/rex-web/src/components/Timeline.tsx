import type { TimelineTask, TurnPhase } from "../types";
import { MotionTimelineItem } from "./Motion";

interface Props {
  tasks: TimelineTask[];
  phase: TurnPhase;
}

export function Timeline({ tasks, phase }: Props) {
  return (
    <aside className="timeline" data-testid="timeline">
      <p style={{ marginTop: 0, color: "var(--rex-text-secondary)" }}>Timeline</p>
      {tasks.length === 0 ? (
        <p>{phase === "generating" ? "Working…" : "No tasks"}</p>
      ) : (
        <ul style={{ paddingLeft: "1rem", margin: 0 }}>
          {tasks.map((task) => (
            <MotionTimelineItem key={task.id}>{task.label}</MotionTimelineItem>
          ))}
        </ul>
      )}
    </aside>
  );
}
