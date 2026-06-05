/** Mirrors daemon `plan.save` path policy for extension-local Save. */

export function normalizePlanSavePath(path: string): string {
  const trimmed = path.trim().replace(/^\/+/, "");
  if (trimmed.startsWith(".rex/plans/")) {
    return trimmed;
  }
  const name = trimmed.replace(/^\.rex\/plans\//, "");
  return `.rex/plans/${name}`;
}

export type PlanPathValidation =
  | { readonly ok: true; readonly normalized: string }
  | { readonly ok: false; readonly message: string };

export function validatePlanSavePath(path: string): PlanPathValidation {
  const trimmed = path.trim();
  if (trimmed.length === 0) {
    return { ok: false, message: "plan.save path must not be empty" };
  }
  const normalized = normalizePlanSavePath(trimmed);
  if (!normalized.startsWith(".rex/plans/") || !normalized.endsWith(".md")) {
    return {
      ok: false,
      message: "plan.save path must be under .rex/plans/ and end with .md",
    };
  }
  if (normalized.includes("..")) {
    return { ok: false, message: "plan.save path must not contain .." };
  }
  return { ok: true, normalized };
}

export function defaultPlanSavePath(title: string): string {
  const slug = title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 48);
  const name = slug.length > 0 ? slug : "plan";
  return `.rex/plans/${name}.md`;
}
