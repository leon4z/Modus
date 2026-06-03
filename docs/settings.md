# Settings

- Status: active
- Source-of-Truth: true
- Type: product

Settings controls Modus preferences and local tool management options.

The module is where the user changes how Modus behaves. It keeps app preferences separate from feature actions such as rule injection, Skill installation, configuration saving, or MCP editing.

## Product Functions

- Changes language preference.
- Changes theme preference.
- Enables or disables tools for Modus management.
- Shows detected tools and their local management state.
- Edits supported tool paths and local source overrides.
- Resets custom tool path or source settings back to defaults when supported.
- Opens local application log views.
- Opens module performance diagnostic views when diagnostics are enabled.
- Exports local logs for troubleshooting.
- Opens the public project GitHub page from About.
- Checks for application updates.
- Shows install or restart actions when an application update is available.

## Typical Workflow

1. Open Settings after first launch to choose which tools Modus should manage.
2. Adjust language and theme preferences.
3. Review tool rows when a tool is missing, disabled, or using a custom path.
4. Open logs only when troubleshooting is needed.
5. Use the update section to check for, install, or complete an available app update.

## Tool Management

Settings controls Modus' management scope. Disabling a tool removes it from normal Dashboard, Rules, Skills, Config, and MCP surfaces, but it does not delete the external tool's files.

Custom paths and source overrides are local Modus settings. Saving them changes how Modus resolves the tool; it does not automatically create, edit, delete, inject, install, or clean up files in the external tool.

## Logs And Diagnostics

Settings can open local logs from inside the app so the user does not need to browse hidden folders manually. Logs are for troubleshooting and should avoid exposing raw secrets or unnecessary local content.

Module performance diagnostics are local and opt-in. They are meant to help diagnose slow module loads or expensive refreshes.

## Updates

Settings includes the app update surface. When an update is detected, Modus can show an available update state and guide the user through install or restart actions.

## Scope

Settings changes Modus-owned preferences and management scope. It does not delete external tool files, rewrite tool-owned sources, or perform module actions that belong to Rules, Skills, Config, or MCP.
