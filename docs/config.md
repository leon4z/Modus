# Config

- Status: active
- Source-of-Truth: true
- Type: product

Config helps the user view and edit ordinary configuration files for supported AI coding tools.

The module is for ordinary tool configuration, such as JSON, TOML, or YAML settings files that Modus can safely identify. It is not a general file browser.

## Product Functions

- Shows configuration files that Modus can identify as ordinary tool configuration sources.
- Displays file existence, format, editability, and basic availability state.
- Opens readable configuration files for inspection.
- Provides content search while a configuration file is open.
- Allows editing only when the source is writable and exists.
- Saves writable configuration files with format validation.
- Creates a backup before writing when the save path supports it.
- Refreshes the file state after save so the user can see the updated result.

## Typical Workflow

1. Open Config and choose a managed tool.
2. Select a configuration file from the list.
3. Review the content and availability state.
4. Enter edit mode only when the file is writable.
5. Save after checking the edited content.

## What Counts As Ordinary Configuration

Ordinary configuration is a tool settings file that is not owned by another Modus module. Examples include supported JSON, TOML, or YAML settings files declared for a tool.

Config intentionally excludes files that belong to specialized workflows:

- Rules files.
- Skill folders and Skill files.
- MCP configuration fragments.
- Credentials and secret files.
- Logs, caches, generated files, and runtime state.

## File Safety

Config does not create missing configuration files implicitly. If a file is missing, unreadable, unsupported, or read-only, the page should show that state instead of offering a normal save flow.

When saving is available, the edited content is validated according to the declared file format before write.

## Scope

Config manages explicit ordinary configuration files only. It does not manage every file under a tool's configuration directory and does not replace Settings path management.
