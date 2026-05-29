use crate::{message::ToolCall, tui::ui::tools_ui};
use jcode_tui_style::theme::dim_color;
use ratatui::prelude::*;
use crate::tui::markdown;

pub(super) fn diff_add_color() -> Color {
    Color::Rgb(100, 200, 100)
}

pub(super) fn diff_del_color() -> Color {
    Color::Rgb(200, 100, 100)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DiffLineKind {
    Add,
    Del,
}

#[derive(Clone, Debug)]
pub(super) struct ParsedDiffLine {
    pub kind: DiffLineKind,
    pub prefix: String,
    pub content: String,
}

pub(super) fn diff_change_counts(content: &str) -> (usize, usize) {
    let lines = collect_diff_lines(content);
    let additions = lines
        .iter()
        .filter(|line| line.kind == DiffLineKind::Add)
        .count();
    let deletions = lines
        .iter()
        .filter(|line| line.kind == DiffLineKind::Del)
        .count();
    (additions, deletions)
}

pub(super) fn diff_change_counts_for_tool(tool: &ToolCall, content: &str) -> (usize, usize) {
    let (additions, deletions) = diff_change_counts(content);
    if additions > 0 || deletions > 0 {
        return (additions, deletions);
    }

    match tools_ui::canonical_tool_name(&tool.name) {
        "edit" => {
            diff_counts_from_input_pair(&tool.input, "old_string", "new_string").unwrap_or((0, 0))
        }
        "write" => {
            let content = tool
                .input
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            diff_counts_from_strings("", content)
        }
        "multiedit" => diff_counts_from_multiedit(&tool.input).unwrap_or((0, 0)),
        "patch" => diff_counts_from_unified_patch_input(&tool.input).unwrap_or((0, 0)),
        "apply_patch" => diff_counts_from_apply_patch_input(&tool.input).unwrap_or((0, 0)),
        _ => (additions, deletions),
    }
}

fn diff_counts_from_input_pair(
    input: &serde_json::Value,
    old_key: &str,
    new_key: &str,
) -> Option<(usize, usize)> {
    let old = input.get(old_key)?.as_str()?;
    let new = input.get(new_key)?.as_str()?;
    Some(diff_counts_from_strings(old, new))
}

fn diff_counts_from_multiedit(input: &serde_json::Value) -> Option<(usize, usize)> {
    let edits = input.get("edits")?.as_array()?;
    let mut additions = 0usize;
    let mut deletions = 0usize;

    for edit in edits {
        let old = edit
            .get("old_string")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let new = edit
            .get("new_string")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if old.is_empty() && new.is_empty() {
            continue;
        }
        let (add, del) = diff_counts_from_strings(old, new);
        additions += add;
        deletions += del;
    }

    Some((additions, deletions))
}

fn diff_counts_from_unified_patch_input(input: &serde_json::Value) -> Option<(usize, usize)> {
    let patch_text = input.get("patch_text")?.as_str()?;
    let mut additions = 0usize;
    let mut deletions = 0usize;

    for line in patch_text.lines() {
        if line.starts_with("+++")
            || line.starts_with("---")
            || line.starts_with("@@")
            || line.starts_with("diff --git")
            || line.starts_with("index ")
            || line.starts_with("\\ No newline")
        {
            continue;
        }
        if line.starts_with('+') {
            additions += 1;
        } else if line.starts_with('-') {
            deletions += 1;
        }
    }

    Some((additions, deletions))
}

fn diff_counts_from_apply_patch_input(input: &serde_json::Value) -> Option<(usize, usize)> {
    let patch_text = input.get("patch_text")?.as_str()?;
    let mut additions = 0usize;
    let mut deletions = 0usize;

    for line in patch_text.lines() {
        if line.starts_with("***") || line.starts_with("@@") {
            continue;
        }

        if line.starts_with('+') {
            additions += 1;
        } else if line.starts_with('-') {
            deletions += 1;
        }
    }

    Some((additions, deletions))
}

fn diff_counts_from_strings(old: &str, new: &str) -> (usize, usize) {
    use similar::ChangeTag;

    let diff = similar::TextDiff::from_lines(old, new);
    let mut additions = 0usize;
    let mut deletions = 0usize;
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => additions += 1,
            ChangeTag::Delete => deletions += 1,
            ChangeTag::Equal => {}
        }
    }
    (additions, deletions)
}

pub(super) fn generate_diff_lines_from_tool_input(tool: &ToolCall) -> Vec<ParsedDiffLine> {
    match tools_ui::canonical_tool_name(&tool.name) {
        "edit" => {
            let old = tool
                .input
                .get("old_string")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let new = tool
                .input
                .get("new_string")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            generate_diff_lines_from_strings(old, new)
        }
        "multiedit" => {
            let Some(edits) = tool.input.get("edits").and_then(|v| v.as_array()) else {
                return Vec::new();
            };
            let mut all_lines = Vec::new();
            for edit in edits {
                let old = edit
                    .get("old_string")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let new = edit
                    .get("new_string")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                all_lines.extend(generate_diff_lines_from_strings(old, new));
            }
            all_lines
        }
        "write" => {
            let content = tool
                .input
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            generate_diff_lines_from_strings("", content)
        }
        "patch" => {
            let patch_text = tool
                .input
                .get("patch_text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            collect_diff_lines(patch_text)
        }
        "apply_patch" => {
            let patch_text = tool
                .input
                .get("patch_text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            collect_diff_lines(patch_text)
        }
        _ => Vec::new(),
    }
}

fn generate_diff_lines_from_strings(old: &str, new: &str) -> Vec<ParsedDiffLine> {
    use similar::ChangeTag;

    let diff = similar::TextDiff::from_lines(old, new);
    let mut lines = Vec::new();

    for change in diff.iter_all_changes() {
        let content = change.value().trim();
        if content.is_empty() {
            continue;
        }

        match change.tag() {
            ChangeTag::Delete => {
                lines.push(ParsedDiffLine {
                    kind: DiffLineKind::Del,
                    prefix: format!("{}- ", change.old_index().unwrap_or(0) + 1),
                    content: content.to_string(),
                });
            }
            ChangeTag::Insert => {
                lines.push(ParsedDiffLine {
                    kind: DiffLineKind::Add,
                    prefix: format!("{}+ ", change.new_index().unwrap_or(0) + 1),
                    content: content.to_string(),
                });
            }
            ChangeTag::Equal => {}
        }
    }

    lines
}

pub(super) fn collect_diff_lines(content: &str) -> Vec<ParsedDiffLine> {
    content.lines().filter_map(parse_diff_line).collect()
}

fn parse_diff_line(raw_line: &str) -> Option<ParsedDiffLine> {
    let trimmed = raw_line.trim();
    if trimmed.is_empty() || trimmed == "..." {
        return None;
    }
    if trimmed.starts_with("diff --git ")
        || trimmed.starts_with("index ")
        || trimmed.starts_with("--- ")
        || trimmed.starts_with("+++ ")
        || trimmed.starts_with("@@ ")
        || trimmed.starts_with("\\ No newline")
    {
        return None;
    }

    if let Some(pos) = trimmed.find("- ") {
        let (prefix, content) = trimmed.split_at(pos + 2);
        if !prefix.is_empty() && prefix[..pos].chars().all(|c| c.is_ascii_digit()) {
            return Some(ParsedDiffLine {
                kind: DiffLineKind::Del,
                prefix: prefix.to_string(),
                content: trim_diff_content(content),
            });
        }
    }
    if let Some(pos) = trimmed.find("+ ") {
        let (prefix, content) = trimmed.split_at(pos + 2);
        if !prefix.is_empty() && prefix[..pos].chars().all(|c| c.is_ascii_digit()) {
            return Some(ParsedDiffLine {
                kind: DiffLineKind::Add,
                prefix: prefix.to_string(),
                content: trim_diff_content(content),
            });
        }
    }

    if let Some(rest) = raw_line.strip_prefix('+') {
        return Some(ParsedDiffLine {
            kind: DiffLineKind::Add,
            prefix: "+".to_string(),
            content: trim_diff_content(rest),
        });
    }
    if let Some(rest) = raw_line.strip_prefix('-') {
        return Some(ParsedDiffLine {
            kind: DiffLineKind::Del,
            prefix: "-".to_string(),
            content: trim_diff_content(rest),
        });
    }

    None
}

fn trim_diff_content(content: &str) -> String {
    content.trim_start_matches([' ', '\t']).to_string()
}

pub(super) fn tint_span_with_diff_color(span: Span<'static>, diff_color: Color) -> Span<'static> {
    let (dr, dg, db) = match diff_color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Indexed(n) => super::color_support::indexed_to_rgb(n),
        _ => return span,
    };

    let fg = span.style.fg.unwrap_or(Color::White);
    let (sr, sg, sb) = match fg {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Indexed(n) => super::color_support::indexed_to_rgb(n),
        Color::White => (255, 255, 255),
        Color::Black => (0, 0, 0),
        _ => return span,
    };

    let blend = |s: u8, d: u8| -> u8 { ((s as u16 * 70 + d as u16 * 30) / 100) as u8 };

    let tinted = Color::Rgb(blend(sr, dr), blend(sg, dg), blend(sb, db));
    Span::styled(span.content, span.style.fg(tinted))
}

/// Shared pure collection helper (promoted from ui_messages per Ola 2 seam promotion).
///
/// Deduplicates the "try collect_diff_lines from content first (the parsed
/// +/- lines already present in the tool output), else fall back to
/// generate_diff_lines_from_tool_input (synthesizes from old/new or patch_text
/// in the ToolCall input)" pattern.
///
/// Now lives in ui_diff.rs (pub(super)) so ui_messages, ui_pinned, and ui_file_diff
/// all delegate to the single implementation (zero dupe, identical behavior).
pub(super) const MAX_INLINE_DIFF_LINES: usize = 12;

pub(super) fn edit_change_lines_for_tool(tc: &ToolCall, content: &str) -> Vec<ParsedDiffLine> {
    let from_content = collect_diff_lines(content);
    if !from_content.is_empty() {
        from_content
    } else {
        generate_diff_lines_from_tool_input(tc)
    }
}

/// Renders the inline edit diff block (┌─ header, bordered change lines with
/// syntax highlight tinted by add/del colors, truncation "… more changes …"
/// for non-full-inline when exceeding MAX_INLINE_DIFF_LINES, and └─ footer)
/// for edit/multiedit/patch/apply_patch tools under inline diff modes.
///
/// Promoted to ui_diff.rs (pub(super)) during Ola 2 TUI seam promotion for
/// reusability alongside the change_lines collector (used by messages render
/// + potentially other diff views). Delegates to ui_diff collectors + markdown
/// highlight + tint. Behavior 100% identical to prior private impl.
/// No App state, no mutation.
pub(super) fn render_edit_diff_block(
    tc: &ToolCall,
    content: &str,
    width: u16,
    full_inline: bool,
) -> Vec<Line<'static>> {
    let file_path_for_ext = tc
        .input
        .get("file_path")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| {
            tc.input
                .get("patch_text")
                .and_then(|v| v.as_str())
                .and_then(|patch_text| match tools_ui::canonical_tool_name(&tc.name) {
                    "apply_patch" => tools_ui::extract_apply_patch_primary_file(patch_text),
                    "patch" => tools_ui::extract_unified_patch_primary_file(patch_text),
                    _ => None,
                })
        });
    let file_ext = file_path_for_ext
        .as_deref()
        .and_then(|p| std::path::Path::new(p).extension())
        .and_then(|e| e.to_str());

    let change_lines = edit_change_lines_for_tool(tc, content);

    const MAX_DIFF_LINES: usize = MAX_INLINE_DIFF_LINES;
    let total_changes = change_lines.len();
    let additions = change_lines
        .iter()
        .filter(|line| line.kind == DiffLineKind::Add)
        .count();
    let deletions = change_lines
        .iter()
        .filter(|line| line.kind == DiffLineKind::Del)
        .count();

    let (display_lines, truncated, half_point): (Vec<&ParsedDiffLine>, bool, usize) =
        if full_inline || total_changes <= MAX_DIFF_LINES {
            (change_lines.iter().collect(), false, usize::MAX)
        } else {
            let half = MAX_DIFF_LINES / 2;
            let mut result: Vec<&ParsedDiffLine> = change_lines.iter().take(half).collect();
            result.extend(change_lines.iter().skip(total_changes - half));
            (result, true, half)
        };

    let pad_str = "";

    let mut out: Vec<Line<'static>> = Vec::new();

    out.push(
        Line::from(Span::styled(
            format!("{}┌─ diff", pad_str),
            Style::default().fg(dim_color()),
        ))
        .alignment(ratatui::layout::Alignment::Left),
    );

    let mut shown_truncation = false;

    for (i, line) in display_lines.iter().enumerate() {
        if truncated && !shown_truncation && i >= half_point {
            let skipped = total_changes - MAX_DIFF_LINES;
            out.push(
                Line::from(Span::styled(
                    format!("{}│ ... {} more changes ...", pad_str, skipped),
                    Style::default().fg(dim_color()),
                ))
                .alignment(ratatui::layout::Alignment::Left),
            );
            shown_truncation = true;
        }

        let base_color = if line.kind == DiffLineKind::Add {
            diff_add_color()
        } else {
            diff_del_color()
        };

        let border_prefix = format!("{}│ ", pad_str);
        let prefix_visual_width = unicode_width::UnicodeWidthStr::width(border_prefix.as_str())
            + unicode_width::UnicodeWidthStr::width(line.prefix.as_str());
        let max_content_width = (width as usize).saturating_sub(prefix_visual_width + 1);

        let mut spans: Vec<Span<'static>> = vec![
            Span::styled(border_prefix, Style::default().fg(dim_color())),
            Span::styled(line.prefix.clone(), Style::default().fg(base_color)),
        ];

        if !line.content.is_empty() {
            let content = &line.content;
            let content_vis_width = unicode_width::UnicodeWidthStr::width(content.as_str());
            if !full_inline && max_content_width > 1 && content_vis_width > max_content_width {
                let mut end = 0;
                let mut vis_w = 0;
                let limit = max_content_width.saturating_sub(1);
                for (i, ch) in content.char_indices() {
                    let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                    if vis_w + cw > limit {
                        break;
                    }
                    vis_w += cw;
                    end = i + ch.len_utf8();
                }
                let truncated = &content[..end];
                let highlighted = markdown::highlight_line(truncated, file_ext);
                for span in highlighted {
                    spans.push(tint_span_with_diff_color(span, base_color));
                }
                spans.push(Span::styled("…", Style::default().fg(dim_color())));
            } else {
                let highlighted = markdown::highlight_line(content.as_str(), file_ext);
                for span in highlighted {
                    spans.push(tint_span_with_diff_color(span, base_color));
                }
            }
        }

        out.push(Line::from(spans).alignment(ratatui::layout::Alignment::Left));
    }

    let footer = if total_changes > 0 && truncated {
        format!("{}└─ (+{} -{} total)", pad_str, additions, deletions)
    } else {
        format!("{}└─", pad_str)
    };
    out.push(
        Line::from(Span::styled(footer, Style::default().fg(dim_color())))
            .alignment(ratatui::layout::Alignment::Left),
    );

    out
}

#[cfg(test)]
mod tests {
    use super::{
        DiffLineKind, diff_change_counts_for_tool, diff_counts_from_apply_patch_input,
        generate_diff_lines_from_strings,
    };
    use crate::message::ToolCall;
    use serde_json::json;

    #[test]
    fn apply_patch_counts_ignore_context_lines_with_plus_or_minus_prefixes() {
        let input = json!({
            "patch_text": "*** Begin Patch\n*** Update File: demo.txt\n@@\n  +context line\n  -context line\n+added line\n-deleted line\n*** End Patch\n"
        });

        assert_eq!(diff_counts_from_apply_patch_input(&input), Some((1, 1)));
    }

    #[test]
    fn write_tool_falls_back_to_content_diff_counts() {
        let tool = ToolCall {
            id: "tool_1".to_string(),
            name: "write".to_string(),
            input: json!({
                "file_path": "demo.txt",
                "content": "first line\nsecond line\n"
            }),
            intent: None,
        };

        assert_eq!(diff_change_counts_for_tool(&tool, ""), (2, 0));
    }

    #[test]
    fn multiedit_pascal_case_falls_back_to_input_diff_counts() {
        let tool = ToolCall {
            id: "tool_2".to_string(),
            name: "MultiEdit".to_string(),
            input: json!({
                "file_path": "demo.txt",
                "edits": [
                    {"old_string": "two\n", "new_string": "TWO\n"},
                    {"old_string": "three\n", "new_string": "THREE\n"}
                ]
            }),
            intent: None,
        };

        assert_eq!(diff_change_counts_for_tool(&tool, ""), (2, 2));
    }

    #[test]
    fn generated_diff_lines_use_old_and_new_line_numbers() {
        let lines =
            generate_diff_lines_from_strings("one\ntwo\nthree\n", "one\nthree\nfour\nfive\n");

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].kind, DiffLineKind::Del);
        assert_eq!(lines[0].prefix, "2- ");
        assert_eq!(lines[1].kind, DiffLineKind::Add);
        assert_eq!(lines[1].prefix, "3+ ");
        assert_eq!(lines[2].kind, DiffLineKind::Add);
        assert_eq!(lines[2].prefix, "4+ ");
    }
}
