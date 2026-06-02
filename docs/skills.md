# Skills

- Status: active
- Source-of-Truth: true
- Type: product

Skills helps the user inspect and manage local Skill folders used by supported AI coding tools.

The module focuses on local files the user already has: shared Skills and tool-specific Skill directories. It is not a remote catalog or package manager.

The shared Skill directory is `~/.agents/skills/`. This is a community-used user-level Agent Skills directory for Skills that can be reused across compatible local AI coding tools.

## Product Functions

- Shows Skills from the shared Skill directory, `~/.agents/skills/`.
- Shows Skills from supported tool-specific Skill directories.
- Groups multiple local sources for the same logical Skill when they exist.
- Opens a Skill detail view with file tree and content preview.
- Searches and navigates inside visible Skill content.
- Supports editing eligible Skill files.
- Installs a shared Skill into supported tools when the target source allows it.
- Copies a Skill between supported local sources.
- Uninstalls linked Skill sources when the tool source is a link.
- Deletes eligible real local Skill folders after confirmation.
- Shows abnormal local source states so the user can decide whether to keep, remove, or repair them.

## Typical Workflow

1. Open Skills and choose a managed tool or the shared source view.
2. Select a Skill to inspect its files and content.
3. Install or copy the Skill to another supported local source when needed.
4. Edit eligible files when local Skill content should change.
5. Use uninstall or delete only after reviewing the confirmation preview.

## Source Types

Skills distinguishes where a Skill came from because the safe action depends on the source.

- Shared source: a Skill stored in `~/.agents/skills/`.
- Tool source: a real Skill folder inside a supported tool's Skill directory.
- Linked source: a tool source that points back to shared content.
- Abnormal source: a missing, broken, duplicated, or inconsistent local source that needs review.

These source labels help the user avoid confusing "remove this tool link" with "delete the real shared Skill folder."

## Shared Directory Behavior

Skills in `~/.agents/skills/` are treated as user-level shared local content. A compatible tool may read that directory directly, or Modus may install a shared Skill into a tool-specific Skill directory when that tool needs a local link or copy.

Modus keeps these cases separate:

- Direct shared availability: the tool can use the Skill from `~/.agents/skills/`.
- Shared install: Modus links a shared Skill into a supported tool directory.
- Tool copy: Modus creates a real copy inside a supported tool directory.

This distinction is why a shared Skill can appear as available for one tool, installable for another tool, and copied for a third tool.

## File Safety

Skill operations can affect folders, links, and multiple local sources. Modus should show a confirmation preview for file-changing operations such as install, copy, uninstall, and delete.

The preview explains the action, target paths, and affected sources before the operation is applied.

## Scope

Skills manages local files that already belong to the user's shared or tool-specific Skill sources. It does not provide a remote store, global version governance, batch migration, batch update workflow, or automatic cleanup of every abnormal local state.
