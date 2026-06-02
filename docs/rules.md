# Rules

- Status: active
- Source-of-Truth: true
- Type: product

Rules helps the user manage local instruction content for supported AI coding tools.

The module separates two ideas that are easy to mix up:

- Modus global rules: one app-owned rule set that can be written into supported tool files.
- Tool rules: the rule files that already belong to a specific external tool.

This lets the user keep one shared instruction baseline while still inspecting and editing tool-native rule files when they exist.

## Product Functions

- Maintains a Modus-owned global rule.
- Shows which tools can receive the global rule.
- Previews global rule injection before changing tool-owned files.
- Writes only the managed rule section in supported target files.
- Shows each supported tool's own rule files when they are available.
- Lets the user view, search, edit, copy, create, rename, or delete eligible tool rule files.
- Compares rule content when the user needs to inspect differences.
- Tracks whether managed rule sections are synchronized, pending, drifted, or unavailable.
- Allows leaving rule management for supported targets while preserving or removing the managed section according to the selected action.

## Typical Workflow

1. Open Rules and review the global rule.
2. Edit the global rule content when the shared instruction should change.
3. Preview which enabled tools will be updated.
4. Confirm injection only after checking the target list and exceptions.
5. Open a tool's own rule files when tool-specific content needs to be inspected or edited.

## File Safety

Rules is file-backed. When Modus writes to external tool files, it should make the change explicit before the write happens.

- Global rule injection uses preview and confirmation.
- The managed section is separated from the rest of the target file.
- Existing content outside the managed section is not treated as Modus-owned content.
- Unsupported, unknown, unreadable, or unconfigured targets are not silently written.
- Tool-native file edits stay scoped to the selected file.

## Unsupported States

Some AI coding tools keep global instructions inside private app settings or do not expose a stable local file for Modus to manage. In those cases, Rules shows the source as unavailable or unsupported instead of guessing a path.

## Scope

Rules manages file-backed local rule sources only. It does not manage project rules, private app-internal rule settings, cloud rules, or rule models that require editing inside another application.
