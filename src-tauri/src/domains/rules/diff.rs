// Purpose: Own pure rule content diff mapping for the Rules domain.

use super::*;
use similar::{ChangeTag, TextDiff};

pub(crate) fn diff_rules_domain(
    left_content: String,
    left_label: String,
    right_content: String,
    right_label: String,
) -> DiffResult {
    let left_content = normalize_managed_marker_generation(&left_content);
    let right_content = normalize_managed_marker_generation(&right_content);
    let diff = TextDiff::from_lines(&left_content, &right_content);
    let changes = diff
        .iter_all_changes()
        .map(|change| {
            let tag = match change.tag() {
                ChangeTag::Delete => "delete",
                ChangeTag::Insert => "insert",
                ChangeTag::Equal => "equal",
            };
            DiffLine {
                tag: tag.to_string(),
                content: change.to_string(),
            }
        })
        .collect();

    DiffResult {
        left_label,
        right_label,
        changes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_identical_content() {
        let result = diff_rules_domain(
            "hello\nworld".to_string(),
            "left".to_string(),
            "hello\nworld".to_string(),
            "right".to_string(),
        );
        assert!(result.changes.iter().all(|c| c.tag == "equal"));
    }

    #[test]
    fn diff_completely_different() {
        let result = diff_rules_domain(
            "aaa".to_string(),
            "left".to_string(),
            "bbb".to_string(),
            "right".to_string(),
        );
        assert!(result.changes.iter().any(|c| c.tag == "delete"));
        assert!(result.changes.iter().any(|c| c.tag == "insert"));
    }

    #[test]
    fn diff_empty_left() {
        let result = diff_rules_domain(
            "".to_string(),
            "left".to_string(),
            "content".to_string(),
            "right".to_string(),
        );
        assert!(result.changes.iter().any(|c| c.tag == "insert"));
        assert!(!result.changes.iter().any(|c| c.tag == "delete"));
    }

    #[test]
    fn diff_empty_right() {
        let result = diff_rules_domain(
            "content".to_string(),
            "left".to_string(),
            "".to_string(),
            "right".to_string(),
        );
        assert!(result.changes.iter().any(|c| c.tag == "delete"));
        assert!(!result.changes.iter().any(|c| c.tag == "insert"));
    }

    #[test]
    fn diff_partial_change() {
        let result = diff_rules_domain(
            "line1\nline2\nline3".to_string(),
            "left".to_string(),
            "line1\nchanged\nline3".to_string(),
            "right".to_string(),
        );
        assert!(result.changes.iter().any(|c| c.tag == "equal"));
        assert!(result.changes.iter().any(|c| c.tag == "delete"));
        assert!(result.changes.iter().any(|c| c.tag == "insert"));
    }

    #[test]
    fn diff_preserves_labels() {
        let result = diff_rules_domain(
            "a".to_string(),
            "LEFT_LABEL".to_string(),
            "b".to_string(),
            "RIGHT_LABEL".to_string(),
        );
        assert_eq!(result.left_label, "LEFT_LABEL");
        assert_eq!(result.right_label, "RIGHT_LABEL");
    }

    #[test]
    fn diff_treats_legacy_and_current_managed_markers_as_equal() {
        let left = "<!-- ACC:DEFAULT:START -->\nmanaged\n<!-- ACC:DEFAULT:END -->";
        let right = "<!-- MODUS:DEFAULT:START -->\nmanaged\n<!-- MODUS:DEFAULT:END -->";
        let result = diff_rules_domain(
            left.to_string(),
            "left".to_string(),
            right.to_string(),
            "right".to_string(),
        );

        assert!(result.changes.iter().all(|change| change.tag == "equal"));
        assert!(result
            .changes
            .iter()
            .any(|change| change.content.contains("MODUS:DEFAULT:START")));
        assert!(!result
            .changes
            .iter()
            .any(|change| change.content.contains("ACC:DEFAULT:START")));
    }

    #[test]
    fn diff_still_reports_body_changes_inside_managed_markers() {
        let left = "<!-- ACC:DEFAULT:START -->\nold\n<!-- ACC:DEFAULT:END -->";
        let right = "<!-- MODUS:DEFAULT:START -->\nnew\n<!-- MODUS:DEFAULT:END -->";
        let result = diff_rules_domain(
            left.to_string(),
            "left".to_string(),
            right.to_string(),
            "right".to_string(),
        );

        assert!(result
            .changes
            .iter()
            .any(|change| change.tag == "delete" && change.content.contains("old")));
        assert!(result
            .changes
            .iter()
            .any(|change| change.tag == "insert" && change.content.contains("new")));
        assert!(!result
            .changes
            .iter()
            .any(|change| change.tag != "equal" && change.content.contains("DEFAULT")));
    }
}
