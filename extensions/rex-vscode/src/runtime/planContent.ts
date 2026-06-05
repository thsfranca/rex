/** Format structured plan JSON into editable markdown for the plan card. */

export function formatPlanDetailMarkdown(title: string, detail: string): string {
  if (detail.trim().length === 0) {
    return `# ${title}\n`;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(detail);
  } catch {
    return `# ${title}\n\n${detail}`;
  }
  if (!isRecord(parsed)) {
    return `# ${title}\n\n${detail}`;
  }

  const lines: string[] = [`# ${title}`];
  const steps = parsed["steps"];
  if (Array.isArray(steps) && steps.length > 0) {
    lines.push("", "## Steps");
    for (const step of steps) {
      if (!isRecord(step)) {
        continue;
      }
      const id = typeof step["id"] === "string" ? step["id"] : "";
      const summary = typeof step["summary"] === "string" ? step["summary"] : "";
      const files = step["files"];
      const fileList =
        Array.isArray(files) && files.length > 0
          ? ` (${files.filter((f) => typeof f === "string").join(", ")})`
          : "";
      if (summary.length > 0) {
        lines.push(`- ${id.length > 0 ? `${id}. ` : ""}${summary}${fileList}`);
      }
    }
  }

  const risks = parsed["risks"];
  if (Array.isArray(risks) && risks.length > 0) {
    lines.push("", "## Risks");
    for (const risk of risks) {
      if (typeof risk === "string" && risk.length > 0) {
        lines.push(`- ${risk}`);
      }
    }
  }

  const openQuestions = parsed["open_questions"];
  if (Array.isArray(openQuestions) && openQuestions.length > 0) {
    lines.push("", "## Open questions");
    for (const question of openQuestions) {
      if (typeof question === "string" && question.length > 0) {
        lines.push(`- ${question}`);
      }
    }
  }

  return `${lines.join("\n")}\n`;
}

export interface ClarifyQuestion {
  readonly id: string;
  readonly prompt: string;
  readonly options?: ReadonlyArray<string>;
}

export function parseClarifyQuestions(detail: string): ClarifyQuestion[] {
  if (detail.trim().length === 0) {
    return [];
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(detail);
  } catch {
    return [];
  }
  if (!Array.isArray(parsed)) {
    return [];
  }
  const questions: ClarifyQuestion[] = [];
  for (const item of parsed) {
    if (!isRecord(item)) {
      continue;
    }
    const prompt = typeof item["prompt"] === "string" ? item["prompt"] : "";
    if (prompt.length === 0) {
      continue;
    }
    const id = typeof item["id"] === "string" ? item["id"] : `q${questions.length + 1}`;
    const optionsRaw = item["options"];
    const options =
      Array.isArray(optionsRaw) && optionsRaw.length > 0
        ? optionsRaw.filter((value): value is string => typeof value === "string")
        : undefined;
    questions.push({ id, prompt, ...(options === undefined ? {} : { options }) });
  }
  return questions;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
