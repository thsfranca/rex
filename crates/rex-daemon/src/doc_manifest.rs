//! Load `docs/manifest.yaml` for advisory bundles and knowledge indexing.

use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct ManifestFile {
    #[allow(dead_code)]
    version: u32,
    documents: Vec<ManifestDocument>,
}

#[derive(Debug, Clone, Deserialize)]
struct ManifestDocument {
    path: String,
    #[allow(dead_code)]
    role: String,
    status: String,
    #[serde(default)]
    summary_heading: Option<String>,
    #[serde(default)]
    advisory_bundle: bool,
}

pub fn manifest_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join("docs/manifest.yaml")
}

pub fn load_manifest(workspace_root: &Path) -> Option<ManifestFile> {
    let path = manifest_path(workspace_root);
    let raw = std::fs::read_to_string(path).ok()?;
    serde_yaml::from_str(&raw).ok()
}

pub fn advisory_bundle_paths(workspace_root: &Path) -> Vec<String> {
    let Some(manifest) = load_manifest(workspace_root) else {
        return default_advisory_paths();
    };
    let paths: Vec<String> = manifest
        .documents
        .into_iter()
        .filter(|doc| doc.advisory_bundle && doc.status == "active")
        .map(|doc| doc.path)
        .collect();
    if paths.is_empty() {
        default_advisory_paths()
    } else {
        paths
    }
}

fn default_advisory_paths() -> Vec<String> {
    vec![
        "docs/ROADMAP.md".to_string(),
        "docs/PRIORITIZATION.md".to_string(),
        "README.md".to_string(),
    ]
}

pub fn knowledge_entries(workspace_root: &Path) -> Vec<(String, String)> {
    let Some(manifest) = load_manifest(workspace_root) else {
        return Vec::new();
    };
    manifest
        .documents
        .into_iter()
        .filter(|doc| doc.status == "active")
        .filter_map(|doc| {
            let heading = doc.summary_heading?;
            Some((doc.path, heading))
        })
        .collect()
}

pub fn extract_section(content: &str, heading: &str) -> Option<String> {
    let needle = heading.trim();
    let mut lines = content.lines();
    while let Some(line) = lines.next() {
        if line.trim() == needle {
            let level = needle.chars().take_while(|c| *c == '#').count();
            let mut out = Vec::new();
            for next in lines {
                if next.starts_with('#') {
                    let next_level = next.chars().take_while(|c| *c == '#').count();
                    if next_level <= level {
                        break;
                    }
                }
                out.push(next);
            }
            let text = out.join("\n").trim().to_string();
            if text.is_empty() {
                return None;
            }
            return Some(text);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_summary_section() {
        let body = "# Title\n\n## Summary\n\nHello world.\n\n## Next\n\nNo.";
        let section = extract_section(body, "## Summary").expect("summary");
        assert_eq!(section, "Hello world.");
    }
}
