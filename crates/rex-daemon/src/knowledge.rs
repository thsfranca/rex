//! Budgeted knowledge retrieval from manifest summary sections (ADR 0015 v1).

use std::path::Path;

use crate::doc_manifest::{extract_section, knowledge_entries};
use crate::plugins::estimate_tokens;

const KNOWLEDGE_BUDGET_FRACTION: f32 = 0.15;
const BUNDLE_ID: &str = "rex-docs-v1";

pub struct KnowledgeResult {
    pub text: String,
    pub tokens: usize,
    pub status: &'static str,
}

pub fn load_knowledge_context(workspace_root: &Path, total_budget_tokens: usize) -> KnowledgeResult {
    let budget = ((total_budget_tokens as f32) * KNOWLEDGE_BUDGET_FRACTION).floor() as usize;
    if budget == 0 {
        return KnowledgeResult {
            text: String::new(),
            tokens: 0,
            status: "off",
        };
    }
    let entries = knowledge_entries(workspace_root);
    if entries.is_empty() {
        return KnowledgeResult {
            text: String::new(),
            tokens: 0,
            status: "miss",
        };
    }
    let mut lines: Vec<String> = Vec::new();
    let mut used = 0usize;
    for (rel, heading) in entries {
        let path = workspace_root.join(&rel);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Some(section) = extract_section(&raw, &heading) else {
            continue;
        };
        let line = format!("[{rel}] {section}");
        let line_tokens = estimate_tokens(&line);
        if used + line_tokens > budget {
            break;
        }
        lines.push(line);
        used += line_tokens;
    }
    if lines.is_empty() {
        return KnowledgeResult {
            text: String::new(),
            tokens: 0,
            status: "miss",
        };
    }
    let header = format!("<<KNOWLEDGE:{BUNDLE_ID}>>");
    let mut text = header.clone();
    text.push('\n');
    text.push_str(&lines.join("\n"));
    KnowledgeResult {
        tokens: used + estimate_tokens(&header),
        text,
        status: "hit",
    }
}

pub fn knowledge_metric(status: &str) -> String {
    format!("knowledge={BUNDLE_ID}:{status}")
}
