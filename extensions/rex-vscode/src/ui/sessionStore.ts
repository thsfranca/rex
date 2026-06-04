import type { ExtensionContext } from "vscode";

import type { InteractionMode } from "../shared/messages";

const STORAGE_KEY = "rex.chatSessions.v1";

export interface StoredChatMessage {
  readonly id: string;
  readonly role: "user" | "assistant";
  readonly buffer: string;
  readonly errorMessage?: string;
}

export interface ChatSessionRecord {
  readonly id: string;
  readonly title: string;
  readonly updatedAt: number;
  readonly mode: InteractionMode;
  readonly messages: ReadonlyArray<StoredChatMessage>;
}

export interface SessionStoreSnapshot {
  readonly sessions: ReadonlyArray<ChatSessionRecord>;
  readonly activeSessionId: string;
}

export class SessionStore {
  constructor(private readonly context: ExtensionContext) {}

  load(): SessionStoreSnapshot {
    const raw = this.context.workspaceState.get<SessionStoreSnapshot>(STORAGE_KEY);
    if (raw === undefined || raw.sessions.length === 0) {
      const initial = createDefaultSession();
      return { sessions: [initial], activeSessionId: initial.id };
    }
    return raw;
  }

  async save(snapshot: SessionStoreSnapshot): Promise<void> {
    await this.context.workspaceState.update(STORAGE_KEY, snapshot);
  }
}

export function createDefaultSession(): ChatSessionRecord {
  return {
    id: "session-default",
    title: "Chat",
    updatedAt: Date.now(),
    mode: "ask",
    messages: [],
  };
}

export function deriveSessionTitle(firstUserLine: string): string {
  const trimmed = firstUserLine.trim();
  if (trimmed.length === 0) {
    return "New chat";
  }
  return trimmed.length > 48 ? `${trimmed.slice(0, 48)}…` : trimmed;
}
