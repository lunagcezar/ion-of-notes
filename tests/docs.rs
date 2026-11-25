//! Portable documentation gate (spec-128).
//!
//! Runs without PowerShell: `cargo test --test docs` checks relative Markdown
//! links, table column counts and Mermaid fence/type validity. The PowerShell
//! scripts remain as a fast pre-check, but this test is the cross-platform
//! source of truth.

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn markdown_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "target" || name == ".git" || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            markdown_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }
}

fn relative(path: &Path) -> String {
    path.strip_prefix(root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn links(line: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = line;
    while let Some(index) = rest.find("](") {
        let after = &rest[index + 2..];
        let Some(end) = after.find(')') else {
            break;
        };
        found.push(&after[..end]);
        rest = &after[end + 1..];
    }
    found
}

fn pipe_count(line: &str) -> usize {
    line.replace("\\|", "").matches('|').count()
}

fn is_table_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|')
}

fn allowed_mermaid(first: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "flowchart",
        "graph",
        "sequenceDiagram",
        "classDiagram",
        "erDiagram",
        "stateDiagram",
        "gantt",
        "pie",
        "journey",
        "gitGraph",
        "mindmap",
        "timeline",
        "quadrantChart",
        "requirementDiagram",
        "C4Context",
    ];
    PREFIXES.iter().any(|prefix| first.starts_with(prefix))
}

#[test]
fn documentation_links_tables_and_mermaid_are_consistent() {
    let mut files = Vec::new();
    markdown_files(&root(), &mut files);
    files.sort();
    assert!(!files.is_empty(), "no Markdown files found");

    let mut issues = Vec::new();
    for path in files {
        let Ok(text) = fs::read_to_string(&path) else {
            issues.push(format!("{}: unreadable", relative(&path)));
            continue;
        };
        let mut in_table = false;
        let mut expected = 0usize;
        let mut in_mermaid = false;
        let mut mermaid_start = 0usize;
        let mut mermaid_first = String::new();

        for (index, line) in text.lines().enumerate() {
            let line_number = index + 1;

            for target in links(line) {
                let target = target.trim();
                if target.starts_with("http:")
                    || target.starts_with("https:")
                    || target.starts_with("mailto:")
                    || target.starts_with('#')
                {
                    continue;
                }
                let target = target.split('#').next().unwrap_or("").trim();
                if target.is_empty() {
                    continue;
                }
                let resolved = path.parent().unwrap_or(Path::new(".")).join(target);
                if !resolved.exists() {
                    issues.push(format!(
                        "link: {}:{} -> {}",
                        relative(&path),
                        line_number,
                        target
                    ));
                }
            }

            if line.trim_start().starts_with("```mermaid") {
                in_mermaid = true;
                mermaid_start = line_number;
                mermaid_first.clear();
                continue;
            }
            if in_mermaid && line.trim() == "```" {
                if !allowed_mermaid(&mermaid_first) {
                    issues.push(format!(
                        "mermaid: {}:{}: unrecognized type '{}'",
                        relative(&path),
                        mermaid_start,
                        mermaid_first
                    ));
                }
                in_mermaid = false;
                continue;
            }
            if in_mermaid && mermaid_first.is_empty() && !line.trim().is_empty() {
                mermaid_first = line.trim().to_string();
            }

            if is_table_line(line) {
                let pipes = pipe_count(line);
                if !in_table {
                    in_table = true;
                    expected = pipes;
                } else if pipes != expected {
                    issues.push(format!(
                        "table: {}:{}: {} pipes, expected {}",
                        relative(&path),
                        line_number,
                        pipes,
                        expected
                    ));
                }
            } else {
                in_table = false;
            }
        }

        if in_mermaid {
            issues.push(format!(
                "mermaid: {}: unclosed fence at line {}",
                relative(&path),
                mermaid_start
            ));
        }
    }

    assert!(
        issues.is_empty(),
        "documentation issues:\n{}",
        issues.join("\n")
    );
}
