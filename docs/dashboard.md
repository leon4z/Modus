# Dashboard

- Status: active
- Source-of-Truth: true
- Type: product

Dashboard is the home view for Modus. It gives the user a quick answer to one question: which local AI coding tools are being managed, and what can Modus do with them right now?

The page is designed for orientation rather than editing. It collects the most important local counts and states so the user can decide where to go next.

## Product Functions

- Shows the local tools that are currently enabled for Modus management.
- Displays each tool with its product icon and display name.
- Summarizes how many Rules, Skills, configuration files, and MCP entries Modus can see for each tool.
- Shows abnormal primary configuration states when a tool's expected local configuration path is missing or unreadable.
- Lets the user refresh the overview after installing a tool or changing local files.
- Routes directly to Rules, Skills, Config, MCP, or Settings for the selected tool.

## Typical Workflow

1. Open Dashboard after launching Modus.
2. Review which tools are detected and enabled.
3. Check the Rules, Skills, Config, and MCP counts for each tool.
4. Use a module tile to jump into the area that needs work.
5. If a configuration warning appears, open the tool in Settings and correct the path or management state.

## What The Counts Mean

Dashboard counts are local Modus observations. A count means Modus found supported local sources for that tool and module.

- Rules count means Modus found global or tool-native rule sources.
- Skills count means Modus found local Skill sources.
- Config count means Modus found ordinary configuration files.
- MCP count means Modus found parseable MCP configuration entries.

A zero count does not necessarily mean the external tool has no such feature. It means Modus does not currently have a supported local source for that feature in the managed scope.

## Scope

Dashboard does not edit tool files directly. It does not inject rules, install Skills, save configuration files, save MCP entries, or change which tools are managed.

Those actions stay in the owning modules:

- Rules owns rule editing and injection.
- Skills owns Skill install, copy, uninstall, edit, and delete.
- Config owns ordinary configuration viewing and saving.
- MCP owns MCP configuration viewing and saving.
- Settings owns tool management, paths, preferences, logs, and updates.
