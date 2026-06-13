# REX chat — browser Design Mode harness

Local preview of the chat webview React UI for Cursor **Design Mode**. The harness mocks `acquireVsCodeApi` and seeds host messages so header, messages, code blocks, composer, timeline, approval card, plan card, and session bar render without the extension host.

## Quick start

```bash
cd extensions/rex-vscode
npm install
npm run dev:webview
```

Open **http://127.0.0.1:3456/** in the Cursor integrated browser:

1. **Agents Window** — `Cmd+Shift+P` → “Agents Window”
2. Navigate to **http://127.0.0.1:3456/**
3. **Design Mode** — `Cmd+Shift+P` → “Design Mode” (do not rely on `Cmd+Shift+D` in IDE layout)
4. Click or multi-select elements; ask the agent to edit `webview/components/*.tsx` and `webview/theme/themeVars.css`
5. After changes, verify in the real extension with `npm run watch:webview` and reload the window

## Controls

- **Toggle light/dark theme** — updates `data-rex-theme` CSS variables and posts a `theme` host message
- **Reseed UI state** — replays sample host messages (useful after clearing chat in the UI)

## Files

| File | Role |
|---|---|
| `index.html` | Shell; loads mock before `dist/webview.js` |
| `mockHost.ts` | Mock VS Code API + seed data (`ExtensionToWebview`) |
| `themeVars.css` | `--vscode-*` token stand-ins for dark/light |
| `serve.mjs` | Static server on port **3456** + esbuild watch |

Generated (not committed): `mock-host.js` from `esbuild.dev.mjs`.

## Out of scope

- No daemon or REX CLI integration
- Not included in the VSIX (see `.vscodeignore`)
