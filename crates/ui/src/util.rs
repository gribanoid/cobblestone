// SPDX-License-Identifier: MIT

use std::cmp::Ordering;

use crate::models::{NoteInfo, VaultNode};
use crate::state::TreeSort;

pub fn note_parent_folder(slug: &str) -> Option<String> {
    slug.rfind('/').map(|idx| slug[..idx].to_string())
}

pub fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    fn val(b: u8) -> Result<u32, String> {
        Ok(match b {
            b'A'..=b'Z' => (b - b'A') as u32,
            b'a'..=b'z' => (b - b'a' + 26) as u32,
            b'0'..=b'9' => (b - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            _ => return Err("Invalid base64 character".into()),
        })
    }
    let cleaned: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !cleaned.len().is_multiple_of(4) {
        return Err("Invalid base64 length".into());
    }
    let mut out = Vec::with_capacity(cleaned.len() / 4 * 3);
    for chunk in cleaned.chunks(4) {
        let a = val(chunk[0])?;
        let b = val(chunk[1])?;
        let pad2 = chunk[2] == b'=';
        let pad3 = chunk[3] == b'=';
        let c = if pad2 { 0 } else { val(chunk[2])? };
        let d = if pad3 { 0 } else { val(chunk[3])? };
        let n = (a << 18) | (b << 12) | (c << 6) | d;
        out.push((n >> 16) as u8);
        if !pad2 {
            out.push((n >> 8) as u8);
        }
        if !pad3 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

/// Collect every folder path in the tree (depth-first).
pub fn collect_folder_paths(nodes: &[VaultNode]) -> Vec<String> {
    let mut out = Vec::new();
    collect_folder_paths_into(nodes, &mut out);
    out
}

fn collect_folder_paths_into(nodes: &[VaultNode], out: &mut Vec<String>) {
    for node in nodes {
        if let VaultNode::Folder { path, children, .. } = node {
            out.push(path.clone());
            collect_folder_paths_into(children, out);
        }
    }
}

/// Recursively sort a vault tree for sidebar display.
pub fn sort_vault_tree(nodes: &mut [VaultNode], mode: TreeSort) {
    for node in nodes.iter_mut() {
        if let VaultNode::Folder { children, .. } = node {
            sort_vault_tree(children, mode);
        }
    }
    nodes.sort_by(|a, b| cmp_vault_nodes(a, b, mode));
}

fn cmp_vault_nodes(a: &VaultNode, b: &VaultNode, mode: TreeSort) -> Ordering {
    // Folders, then notes, then attachment files.
    let rank = |n: &VaultNode| match n {
        VaultNode::Folder { .. } => 0,
        VaultNode::Note { .. } => 1,
        VaultNode::File { .. } => 2,
    };
    match (a, b) {
        _ if rank(a) != rank(b) => rank(a).cmp(&rank(b)),
        (
            VaultNode::File {
                name: a_name,
                modified: a_mod,
                ..
            },
            VaultNode::File {
                name: b_name,
                modified: b_mod,
                ..
            },
        ) => match mode {
            TreeSort::NameDesc => cmp_ci(b_name, a_name),
            TreeSort::ModifiedNew | TreeSort::CreatedNew => {
                b_mod.cmp(a_mod).then_with(|| cmp_ci(a_name, b_name))
            }
            TreeSort::ModifiedOld | TreeSort::CreatedOld => {
                a_mod.cmp(b_mod).then_with(|| cmp_ci(a_name, b_name))
            }
            TreeSort::NameAsc => cmp_ci(a_name, b_name),
        },
        (VaultNode::Folder { name: a_name, .. }, VaultNode::Folder { name: b_name, .. }) => {
            match mode {
                TreeSort::NameDesc => cmp_ci(b_name, a_name),
                _ => cmp_ci(a_name, b_name),
            }
        }
        (
            VaultNode::Note {
                title: a_title,
                modified: a_mod,
                created: a_cre,
                ..
            },
            VaultNode::Note {
                title: b_title,
                modified: b_mod,
                created: b_cre,
                ..
            },
        ) => match mode {
            TreeSort::NameAsc => cmp_ci(a_title, b_title),
            TreeSort::NameDesc => cmp_ci(b_title, a_title),
            TreeSort::ModifiedNew => b_mod.cmp(a_mod).then_with(|| cmp_ci(a_title, b_title)),
            TreeSort::ModifiedOld => a_mod.cmp(b_mod).then_with(|| cmp_ci(a_title, b_title)),
            TreeSort::CreatedNew => b_cre.cmp(a_cre).then_with(|| cmp_ci(a_title, b_title)),
            TreeSort::CreatedOld => a_cre.cmp(b_cre).then_with(|| cmp_ci(a_title, b_title)),
        },
        _ => Ordering::Equal,
    }
}

fn cmp_ci(a: &str, b: &str) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase())
}

pub fn flatten_tree(nodes: &[VaultNode]) -> Vec<NoteInfo> {
    let mut out = Vec::new();
    for node in nodes {
        match node {
            VaultNode::Note {
                slug,
                title,
                created,
                modified,
                size,
                preview,
                tags,
            } => out.push(NoteInfo {
                slug: slug.clone(),
                title: title.clone(),
                created: created.clone(),
                modified: modified.clone(),
                size: *size,
                preview: preview.clone(),
                tags: tags.clone(),
            }),
            VaultNode::Folder { children, .. } => out.extend(flatten_tree(children)),
            VaultNode::File { .. } => {}
        }
    }
    out
}

/// Folders whose **name** contains `query` (case-insensitive).
pub fn find_matching_folders(nodes: &[VaultNode], query: &str) -> Vec<(String, String)> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    collect_matching_folders(nodes, &q, &mut out);
    out
}

fn collect_matching_folders(nodes: &[VaultNode], q: &str, out: &mut Vec<(String, String)>) {
    for node in nodes {
        if let VaultNode::Folder {
            name,
            path,
            children,
        } = node
        {
            if name.to_lowercase().contains(q) {
                out.push((path.clone(), name.clone()));
            }
            collect_matching_folders(children, q, out);
        }
    }
}

/// Expand `path` and every parent folder so the node is visible in the tree.
pub fn expand_folder_path(expanded: &mut std::collections::HashSet<String>, path: &str) {
    let mut acc = String::new();
    for part in path.split('/') {
        if part.is_empty() {
            continue;
        }
        if !acc.is_empty() {
            acc.push('/');
        }
        acc.push_str(part);
        expanded.insert(acc.clone());
    }
}

pub fn remap_path(path: &str, from: &str, to: &str) -> String {
    if path == from {
        return to.to_string();
    }
    if let Some(rest) = path.strip_prefix(&format!("{from}/")) {
        return format!("{to}/{rest}");
    }
    path.to_string()
}

pub fn strip_leading_heading(content: &str) -> String {
    let normalized = content.trim_start_matches('\u{FEFF}');
    let mut lines = normalized.split('\n');
    let Some(first) = lines.next() else {
        return String::new();
    };
    let first = first.trim_end_matches('\r');
    if first.trim_start().starts_with('#') {
        let rest: Vec<&str> = lines.collect();
        let joined = rest.join("\n");
        return joined.trim_start_matches('\n').to_string();
    }
    normalized.to_string()
}

pub fn body_for_preview(content: &str, title: &str) -> String {
    let mut body = strip_leading_heading(content);
    let t = title.trim().to_lowercase();
    if t.is_empty() {
        return body;
    }
    let lines: Vec<&str> = body.split('\n').collect();
    let first = lines
        .first()
        .map(|l| l.trim_end_matches('\r').trim())
        .unwrap_or("");
    if let Some(heading) = first
        .strip_prefix('#')
        .map(|s| s.trim_start_matches('#').trim())
        && (heading.eq_ignore_ascii_case(&t) || heading.to_lowercase() == t)
    {
        body = lines[1..].join("\n").trim_start_matches('\n').to_string();
    }
    body
}

pub fn content_with_title(content: &str, title: &str) -> String {
    let t = title.trim();
    if t.is_empty() {
        return content.to_string();
    }
    let mut lines = content.lines();
    if lines.next().is_some_and(|l| l.starts_with("# ")) {
        let rest: Vec<&str> = lines.collect();
        let rest = rest.join("\n");
        if rest.is_empty() {
            format!("# {t}")
        } else {
            format!("# {t}\n{rest}")
        }
    } else {
        format!("# {t}\n\n{content}")
    }
}

pub fn can_drop_note_or_folder(
    kind_note: bool,
    note_slug: Option<&str>,
    folder_path: Option<&str>,
    dest_folder: Option<&str>,
) -> bool {
    match dest_folder {
        None => {
            if kind_note {
                note_slug.is_some_and(|s| note_parent_folder(s).is_some())
            } else {
                folder_path.is_some_and(|p| p.contains('/'))
            }
        }
        Some(dest) => {
            if kind_note {
                note_slug.is_some_and(|s| note_parent_folder(s).as_deref() != Some(dest))
            } else {
                let from = match folder_path {
                    Some(p) => p,
                    None => return false,
                };
                let name = from.rsplit('/').next().unwrap_or(from);
                let new_path = format!("{dest}/{name}");
                if new_path == from {
                    return false;
                }
                if dest == from {
                    return false;
                }
                if dest.starts_with(&format!("{from}/")) {
                    return false;
                }
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_folder() {
        assert_eq!(note_parent_folder("a/b"), Some("a".into()));
        assert_eq!(note_parent_folder("solo"), None);
    }

    #[test]
    fn remap() {
        assert_eq!(remap_path("a/b", "a", "x"), "x/b");
        assert_eq!(remap_path("a", "a", "x"), "x");
    }

    #[test]
    fn matching_folders_by_name() {
        let tree = vec![VaultNode::Folder {
            name: "Ideas".into(),
            path: "ideas".into(),
            children: vec![VaultNode::Folder {
                name: "Archive".into(),
                path: "ideas/archive".into(),
                children: vec![],
            }],
        }];
        let hits = find_matching_folders(&tree, "arch");
        assert_eq!(hits, vec![("ideas/archive".into(), "Archive".into())]);
        let hits = find_matching_folders(&tree, "IDEAS");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, "ideas");
    }

    #[test]
    fn expand_parents() {
        let mut set = std::collections::HashSet::new();
        expand_folder_path(&mut set, "a/b/c");
        assert!(set.contains("a"));
        assert!(set.contains("a/b"));
        assert!(set.contains("a/b/c"));
    }

    fn note(title: &str, modified: &str) -> VaultNode {
        VaultNode::Note {
            slug: title.to_lowercase(),
            title: title.into(),
            created: "2026-01-01 00:00".into(),
            modified: modified.into(),
            size: 1,
            preview: String::new(),
            tags: vec![],
        }
    }

    #[test]
    fn sort_tree_name_and_modified() {
        let mut nodes = vec![
            note("Beta", "2026-01-01 10:00"),
            note("Alpha", "2026-02-01 10:00"),
        ];
        sort_vault_tree(&mut nodes, TreeSort::NameAsc);
        assert!(matches!(&nodes[0], VaultNode::Note { title, .. } if title == "Alpha"));

        sort_vault_tree(&mut nodes, TreeSort::ModifiedNew);
        assert!(matches!(&nodes[0], VaultNode::Note { title, .. } if title == "Alpha"));
    }
}
