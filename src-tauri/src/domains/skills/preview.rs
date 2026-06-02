// Purpose: Build stable Skill operation preview payloads without mutating state.

use super::types::{
    OperationPreview, RejectReason, SkillChangeConfirmationItem, StructuredBlocked,
    StructuredChange,
};
use std::path::Path;

pub(crate) const SUBJECT_SHARED: &str = "__shared__";
pub(crate) const ACTION_INSTALL: &str = "install";
pub(crate) const ACTION_UNINSTALL: &str = "uninstall";
pub(crate) const ACTION_COPY: &str = "copy";
pub(crate) const ACTION_DELETE_FROM_TOOL: &str = "delete_from_tool";
pub(crate) const ACTION_DELETE_SKILL: &str = "delete_skill";
pub(crate) const ACTION_RENAME_SOURCE: &str = "rename_source";

pub(crate) const CHANGE_CREATE: &str = "create";
pub(crate) const CHANGE_DELETE: &str = "delete";
pub(crate) const CHANGE_OVERWRITE: &str = "overwrite";
pub(crate) const CHANGE_PRESERVE: &str = "preserve";

pub(crate) const DOMAIN_TOOL_DIRECTORY: &str = "tool_directory";
pub(crate) const DOMAIN_SHARED_SOURCE: &str = "shared_source";

pub(crate) const AFFECTED_TOOL: &str = "tool";
pub(crate) const AFFECTED_SHARED_SOURCE: &str = "shared_source";

pub(crate) const RISK_OVERWRITE: &str = "overwrite";
pub(crate) const RISK_DELETE: &str = "delete";
pub(crate) const RISK_CONTENT_DIFFERENCE: &str = "content_difference";

pub(crate) const REASON_NOT_INSTALLED: &str = "not_installed";
pub(crate) const REASON_MISSING_SOURCE: &str = "missing_source";
pub(crate) const REASON_SHARED_SOURCE_MISSING: &str = "shared_source_missing";
pub(crate) const REASON_SAME_SOURCE_TARGET: &str = "same_source_target";
pub(crate) const REASON_GENERIC_WRITE_NOT_ALLOWED: &str = "generic_write_not_allowed";
pub(crate) const REASON_POLICY_MISMATCH: &str = "policy_mismatch";
pub(crate) const REASON_BROKEN_SKILL_DIR: &str = "broken_skill_dir";
pub(crate) const REASON_AMBIGUOUS_SOURCE: &str = "ambiguous_source";
pub(crate) const REASON_UNKNOWN: &str = "unknown";

#[derive(Clone, Copy)]
pub(crate) enum PreviewChangeKind {
    Create,
    Delete,
    Preserve,
}

impl PreviewChangeKind {
    fn as_str(self) -> &'static str {
        match self {
            PreviewChangeKind::Create => CHANGE_CREATE,
            PreviewChangeKind::Delete => CHANGE_DELETE,
            PreviewChangeKind::Preserve => CHANGE_PRESERVE,
        }
    }

    fn push_legacy(self, preview: &mut OperationPreview, path: String) {
        match self {
            PreviewChangeKind::Create => preview.creates.push(path),
            PreviewChangeKind::Delete => preview.deletes.push(path),
            PreviewChangeKind::Preserve => preview.preserves.push(path),
        }
    }
}

pub(crate) fn push_preview_change_with_entry_kind(
    preview: &mut OperationPreview,
    kind: PreviewChangeKind,
    action: &str,
    skill_name: &str,
    subject: impl Into<String>,
    path: impl AsRef<Path>,
    entry_kind: Option<&str>,
) {
    let path = path.as_ref().to_string_lossy().to_string();
    let subject = subject.into();
    kind.push_legacy(preview, path.clone());
    preview.changes.push(StructuredChange {
        action: action.to_string(),
        change_kind: kind.as_str().to_string(),
        skill_name: skill_name.to_string(),
        subject: subject.clone(),
        path: path.clone(),
        entry_kind: entry_kind.map(|kind| kind.to_string()),
    });
    preview
        .confirmation_items
        .push(confirmation_item_from_parts(
            kind.as_str(),
            action,
            skill_name,
            &subject,
            &path,
            entry_kind,
        ));
}

pub(crate) fn push_preview_blocked(
    preview: &mut OperationPreview,
    action: &str,
    skill_name: &str,
    subject: impl Into<String>,
    reason: RejectReason,
) {
    if preview.message.is_none() {
        preview.message = Some(reason.message.clone());
    }
    preview.blocked.push(StructuredBlocked {
        action: action.to_string(),
        skill_name: skill_name.to_string(),
        subject: subject.into(),
        reason,
    });
}

fn path_starts_with(path: &str, root: &Path) -> bool {
    let path = Path::new(path);
    path.starts_with(root)
}

fn location_domain_for(subject: &str, path: &str, _action: &str) -> &'static str {
    let generic_dir = crate::platform::env::generic_skills_dir();
    if path_starts_with(path, &generic_dir) || subject == SUBJECT_SHARED || subject == "generic" {
        return DOMAIN_SHARED_SOURCE;
    }
    DOMAIN_TOOL_DIRECTORY
}

fn affected_object_kind_for(_subject: &str, location_domain: &str) -> &'static str {
    if location_domain == DOMAIN_SHARED_SOURCE {
        return AFFECTED_SHARED_SOURCE;
    }
    AFFECTED_TOOL
}

fn affected_object_for(subject: &str, location_domain: &str) -> String {
    if location_domain == DOMAIN_SHARED_SOURCE {
        return SUBJECT_SHARED.to_string();
    }
    subject.to_string()
}

fn risks_for(operation: &str) -> Vec<String> {
    match operation {
        CHANGE_OVERWRITE => vec![
            RISK_OVERWRITE.to_string(),
            RISK_CONTENT_DIFFERENCE.to_string(),
        ],
        CHANGE_DELETE => vec![RISK_DELETE.to_string()],
        _ => Vec::new(),
    }
}

pub(crate) fn confirmation_item_from_parts(
    operation: &str,
    action: &str,
    skill_name: &str,
    subject: &str,
    path: &str,
    entry_shape: Option<&str>,
) -> SkillChangeConfirmationItem {
    let location_domain = location_domain_for(subject, path, action);
    SkillChangeConfirmationItem {
        operation: operation.to_string(),
        location_domain: location_domain.to_string(),
        affected_object_kind: affected_object_kind_for(subject, location_domain).to_string(),
        affected_object: affected_object_for(subject, location_domain),
        real_path: path.to_string(),
        entry_shape: entry_shape.map(str::to_string),
        risk: risks_for(operation),
        action: action.to_string(),
        skill_name: skill_name.to_string(),
        source_context: None,
        target_context: None,
    }
}

pub(crate) fn reject_reason(code: &str, message: &str, raw: Option<String>) -> RejectReason {
    RejectReason {
        code: code.to_string(),
        message: message.to_string(),
        raw,
    }
}

pub(crate) fn classify_reject_reason(raw: &str) -> Option<RejectReason> {
    let reason = if raw == "source_path is required for Community Skill install" {
        reject_reason(
            REASON_MISSING_SOURCE,
            "复制操作缺少来源目录",
            Some(raw.to_string()),
        )
    } else if raw == "source_path must be an existing skill directory containing SKILL.md" {
        reject_reason(
            REASON_BROKEN_SKILL_DIR,
            "来源路径不是合法的 Skill 目录",
            Some(raw.to_string()),
        )
    } else if raw == "source_path and target resolve to the same directory" {
        reject_reason(
            REASON_SAME_SOURCE_TARGET,
            "来源与目标是同一个目录，无需复制",
            Some(raw.to_string()),
        )
    } else if raw == "shared_source_tool_delete_not_allowed" {
        reject_reason(
            REASON_POLICY_MISMATCH,
            "当前工具正在直接使用共享目录，请在共享入口或全局删除里处理",
            Some(raw.to_string()),
        )
    } else if raw == "目标工具未安装该 skill" || raw == "Target skill is not installed" {
        reject_reason(
            REASON_NOT_INSTALLED,
            "目标工具未安装该 Skill，无法执行当前操作",
            Some(raw.to_string()),
        )
    } else if raw == "目标工具未检测到可删除副本" {
        reject_reason(
            REASON_NOT_INSTALLED,
            "目标工具没有可删除的本地文件",
            Some(raw.to_string()),
        )
    } else if raw == "duplicate_skill_source_path_required" {
        reject_reason(
            REASON_AMBIGUOUS_SOURCE,
            "存在多个同名来源，请先选择具体路径",
            Some(raw.to_string()),
        )
    } else if raw.starts_with("Unsupported Community install mode: ") {
        let mode = raw
            .trim_start_matches("Unsupported Community install mode: ")
            .trim();
        reject_reason(
            REASON_UNKNOWN,
            &format!("不支持的安装模式：{}", mode),
            Some(raw.to_string()),
        )
    } else if raw == "Symlink is only supported on Unix" {
        reject_reason(
            REASON_UNKNOWN,
            "当前平台不支持符号链接",
            Some(raw.to_string()),
        )
    } else if raw == "Target path exists but is not a directory" {
        reject_reason(
            REASON_UNKNOWN,
            "目标路径被占用但不是目录",
            Some(raw.to_string()),
        )
    } else {
        return None;
    };
    Some(reason)
}

pub(crate) fn blocked_preview(
    action: &str,
    skill_name: &str,
    subject: &str,
    reason: RejectReason,
) -> OperationPreview {
    let mut preview = OperationPreview::default();
    push_preview_blocked(
        &mut preview,
        action,
        skill_name,
        subject.to_string(),
        reason,
    );
    preview
}

pub(crate) fn preview_reject_or_error(
    dry_run: bool,
    action: &str,
    skill_name: &str,
    subject: &str,
    err: String,
) -> Result<OperationPreview, String> {
    if let Some(reason) = classify_reject_reason(&err) {
        if dry_run {
            return Ok(blocked_preview(action, skill_name, subject, reason));
        }
        return Err(reason.message);
    }
    Err(err)
}
