import { Button, Heading } from "../design-system";
import type { SystemStatus } from "../types";

interface Props {
  status: SystemStatus | null;
  onClose: () => void;
}

export function StatusPanel({ status, onClose }: Props) {
  if (!status) return null;

  return (
    <aside className="rex-status-panel" data-testid="status-panel" role="dialog" aria-label="System status">
      <Heading level={3}>System status</Heading>
      <dl>
        <dt>Daemon</dt>
        <dd>{status.daemonVersion}</dd>
        <dt>Model</dt>
        <dd>{status.activeModelId}</dd>
        <dt>Lifecycle</dt>
        <dd>{status.lifecycleState}</dd>
        <dt>Uptime</dt>
        <dd>{status.uptimeSeconds}s</dd>
        <dt>Workspace</dt>
        <dd>{status.workspaceRoot}</dd>
      </dl>
      <div style={{ marginTop: "var(--rex-space-md)", display: "flex", justifyContent: "flex-end" }}>
        <Button type="button" variant="secondary" data-testid="status-panel-close" onClick={onClose}>
          Close
        </Button>
      </div>
    </aside>
  );
}
