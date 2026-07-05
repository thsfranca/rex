import { Badge, Text } from "../design-system";
import type { ComposerMode } from "../types";
import { MotionStatusDot } from "./Motion";

interface Props {
  workspaceRoot: string | null;
  mode: ComposerMode;
  statusLabel: string;
  working: boolean;
  hasError: boolean;
  onOpenCommands: () => void;
}

function workspaceLabel(root: string | null): string {
  if (!root) return "Workspace";
  const parts = root.split(/[/\\]/).filter(Boolean);
  return parts[parts.length - 1] ?? root;
}

export function AppHeader({
  workspaceRoot,
  mode,
  statusLabel,
  working,
  hasError,
  onOpenCommands,
}: Props) {
  return (
    <div className={`rex-app-header${hasError ? " rex-app-header--error" : ""}`} data-testid="app-header">
      <div className="rex-app-header__brand">
        <span className="rex-app-header__mark">Rex</span>
        <span className="rex-app-header__workspace" title={workspaceRoot ?? undefined}>
          {workspaceLabel(workspaceRoot)}
        </span>
        <Badge testId="header-mode-badge">{mode}</Badge>
      </div>
      <div className="rex-app-header__status">
        <MotionStatusDot working={working} id="status-dot" testId="status-dot" />
        <Text as="span" id="status-label" data-testid="status-label">
          {statusLabel}
        </Text>
        <button
          type="button"
          className="rex-icon-btn"
          data-testid="header-command-trigger"
          aria-label="Open command palette"
          onClick={onOpenCommands}
        >
          ⌘K
        </button>
      </div>
    </div>
  );
}
