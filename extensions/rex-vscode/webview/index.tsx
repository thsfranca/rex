/**
 * Webview entry point.
 *
 * PR 1 ships an intentionally empty webview bundle. The chat UI, React
 * component tree, and streaming renderer land in PR 2. Keeping the entry in
 * place means the dual-esbuild build graph, CSP-capable wiring, and VSIX
 * packaging are validated from day one.
 */

export {};
