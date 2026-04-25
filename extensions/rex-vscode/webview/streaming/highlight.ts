import type { BundledLanguage, BundledTheme, Highlighter } from "shiki";

import type { ThemeKind } from "../../src/shared/messages";

/**
 * Lazy-loaded shiki highlighter wrapper.
 *
 * The first call pulls in `shiki` and asks for the subset of languages we
 * expect to render. Subsequent calls reuse the same highlighter instance.
 * Any language that isn't preloaded falls back to plain-text rendering so
 * unknown grammars never break the UI.
 */
const COMMON_LANGUAGES: BundledLanguage[] = [
  "bash",
  "css",
  "diff",
  "go",
  "html",
  "javascript",
  "json",
  "markdown",
  "python",
  "rust",
  "shell",
  "toml",
  "tsx",
  "typescript",
  "yaml",
];

let highlighterPromise: Promise<Highlighter> | undefined;

export async function highlight(code: string, lang: string, theme: ThemeKind): Promise<string> {
  const shikiTheme = mapTheme(theme);
  const highlighter = await getHighlighter(shikiTheme);
  const language = normalizeLanguage(lang);
  try {
    return highlighter.codeToHtml(code, { lang: language, theme: shikiTheme });
  } catch {
    return `<pre class="rex-code-fallback"><code>${escapeHtml(code)}</code></pre>`;
  }
}

function mapTheme(theme: ThemeKind): BundledTheme {
  switch (theme) {
    case "light":
    case "high-contrast-light":
      return "github-light";
    case "dark":
    case "high-contrast":
    default:
      return "github-dark";
  }
}

async function getHighlighter(theme: BundledTheme): Promise<Highlighter> {
  if (highlighterPromise === undefined) {
    highlighterPromise = loadHighlighter(theme);
  }
  const highlighter = await highlighterPromise;
  if (!highlighter.getLoadedThemes().includes(theme)) {
    await highlighter.loadTheme(theme);
  }
  return highlighter;
}

async function loadHighlighter(theme: BundledTheme): Promise<Highlighter> {
  const { createHighlighter } = await import("shiki");
  return createHighlighter({
    themes: [theme],
    langs: COMMON_LANGUAGES,
  });
}

function normalizeLanguage(lang: string): BundledLanguage {
  const lower = lang.trim().toLowerCase();
  if ((COMMON_LANGUAGES as readonly string[]).includes(lower)) {
    return lower as BundledLanguage;
  }
  return "markdown";
}

function escapeHtml(input: string): string {
  return input
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}
