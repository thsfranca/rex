import { useState } from "react";
import { Button, SegmentedControl, Stack, Textarea } from "../design-system";
import { submitPrompt } from "../ipc";
import { useAppStore } from "../store";
import type { ComposerMode } from "../types";

export type { ComposerMode };

interface Props {
  disabled: boolean;
}

export function Composer({ disabled }: Props) {
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);
  const mode = useAppStore((s) => s.composerMode);
  const setComposerMode = useAppStore((s) => s.setComposerMode);

  async function onSubmit() {
    const prompt = value.trim();
    if (!prompt || busy) return;
    setBusy(true);
    useAppStore.getState().setComposerBusy(true);
    setValue("");
    try {
      await submitPrompt(prompt, mode);
    } finally {
      setBusy(false);
      useAppStore.getState().setComposerBusy(false);
    }
  }

  return (
    <div data-testid="composer">
      <Stack direction="column" gap="sm">
        <SegmentedControl
          value={mode}
          testId="composer-mode"
          disabled={disabled || busy}
          options={[
            { value: "agent", label: "Agent" },
            { value: "ask", label: "Ask" },
          ]}
          onChange={setComposerMode}
        />
        <div className="composer-row">
          <Textarea
            id="composer-input"
            data-testid="composer-input"
            rows={2}
            autoResize
            placeholder="Message Rex…"
            value={value}
            disabled={disabled || busy}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                void onSubmit();
              }
            }}
          />
          <Button type="button" disabled={disabled || busy} onClick={() => void onSubmit()}>
            Send
          </Button>
        </div>
      </Stack>
    </div>
  );
}
