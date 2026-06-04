import type { ApplyResultPayload } from "../src/shared/messages";

export type MessageRole = "user" | "assistant" | "system";

export interface RenderedMessage {
  readonly id: string;
  readonly role: MessageRole;
  readonly buffer: string;
  readonly trailingRaw: string;
  readonly streaming: boolean;
  readonly errorMessage?: string;
  readonly applyResults?: ReadonlyMap<string, ApplyResultPayload>;
}
