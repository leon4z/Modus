# MCP

- Status: active
- Source-of-Truth: true
- Type: product

MCP helps the user inspect and edit local MCP configuration entries for supported AI coding tools.

The module is focused on configuration text. It shows what Modus can read from local MCP configuration files and lets the user edit supported entries without taking over server lifecycle management.

## Product Functions

- Shows user-level or global MCP configuration sources for supported tools.
- Lists MCP server entries that can be parsed from those sources.
- Displays source path, format, and availability state.
- Opens a single MCP entry's configuration fragment.
- Supports search inside the opened fragment.
- Saves writable MCP fragments with validation and backup while preserving unrelated configuration.
- Refreshes MCP sources and entries after local changes.
- Shows unsupported, unreadable, malformed, or empty states without inventing a server status.

## Typical Workflow

1. Open MCP and choose a managed tool.
2. Review the MCP entries Modus found for that tool.
3. Open one entry to inspect the exact configuration fragment.
4. Edit the fragment when the source is writable.
5. Save after validation, then restart or reload the external tool manually if that tool requires it.

## What Modus Shows

MCP entries are parsed from local configuration files. Modus may show names, formats, transport fields, and other non-secret summary information.

The detailed editor shows the selected configuration fragment so the user can inspect or change the text. Values that already exist in the user's local configuration can be visible in the editor because the user opened that file-backed fragment intentionally.

## File Safety

When a save is supported, Modus edits the selected MCP fragment and preserves unrelated configuration in the same file. The save path uses validation and backup before write.

If the source is read-only, missing, malformed, unsupported, or not confirmed, Modus should show the state and avoid offering a normal edit/save flow.

## Scope

MCP manages configuration text only. It does not start or stop MCP servers, test connections, install or uninstall servers, manage marketplace state, configure cloud services, or promise that a saved configuration has already taken effect in the external tool.
