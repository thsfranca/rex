import { useState } from "react";
import { submitPrompt } from "../ipc";
import { useAppStore } from "../store";

interface Props {
  disabled: boolean;
}

export function Composer({ disabled }: Props) {
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);

  async function onSubmit() {
    const prompt = value.trim();
    if (!prompt || busy) return;
    setBusy(true);
    useAppStore.getState().setComposerBusy(true);
    setValue("");
    try {
      await submitPrompt(prompt);
    } finally {
      setBusy(false);
      useAppStore.getState().setComposerBusy(false);
    }
  }

  return (
    <div className="composer" data-testid="composer">
      <div className="composer-row">
        <textarea
          id="composer-input"
          data-testid="composer-input"
          rows={2}
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
        <button type="button" disabled={disabled || busy} onClick={() => void onSubmit()}>
          Send
        </button>
      </div>
    </div>
  );
}
