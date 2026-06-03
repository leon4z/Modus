<div align="center">

# Modus

**A local-first macOS desktop app for managing AI coding tool rules, Skills, MCP, and configs.**

<p>
  <img alt="macOS" src="https://img.shields.io/badge/macOS-desktop-111111">
  <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2-24C8DB">
  <img alt="Local first" src="https://img.shields.io/badge/local--first-yes-2E7D32">
  <img alt="License MIT" src="https://img.shields.io/badge/license-MIT-blue">
</p>

[简体中文](README.zh-CN.md) · [Docs](docs/README.md) · [Changelog](docs/changelog.md)

</div>

![Modus dashboard](docs/assets/dashboard_en.png)

## Why Modus?

AI coding tools often keep their rules, Skills, MCP entries, and configuration files in different local paths. A rule that works well in one agent often has to be copied and edited again for another tool. Whether a Skill is actually available to a specific tool can also require checking folders and tool-specific behavior by hand.

Modus gives those local assets one visible, reusable, and auditable workspace. You can manage rules, Skills, MCP, and config files in one interface, then review the exact file changes before anything is written.

## Highlights

- **Inject one global rule set across tools**: Maintain shared rules once and sync them into supported tools so different agents work under the same constraints without repeated manual edits.
- **See whether a Skill is usable by each tool**: Skill cards show per-tool availability, making it clear which tools can currently use a given Skill.
- **Support shared Skills and tool-local Skills**: Modus distinguishes tools that support user-level shared Skills from tools that only read tool-local Skill folders. It is compatible with common `skill.sh` community install patterns, including linking a shared Skill into a tool folder while keeping one maintainable source.
- **Reuse polished Skills across tools**: A Skill refined inside one tool can be copied into another tool without rebuilding the same folder by hand.
- **Unload Skills that waste context**: Too many Skills can consume context. Modus makes it easy to uninstall a Skill from one tool or delete a source after review.
- **Edit dotfile configs from one place**: Agent configuration often lives in local dotfile directories and usually requires a terminal or IDE. Modus brings supported config entries into one editing surface.
- **Audit file changes before writing**: Creates, edits, deletes, and symlinks are previewed before confirmation, reducing the risk of accidentally removing rules or Skills you have already refined.

## Screenshots

| Rules | Skills |
| --- | --- |
| ![Rules](docs/assets/rules_en.png) | ![Skills](docs/assets/skills_en.png) |

| MCP | Config |
| --- | --- |
| ![MCP](docs/assets/mcp_en.png) | ![Config](docs/assets/config_en.png) |

## What Modus Manages

| Surface | What Modus does |
| --- | --- |
| Dashboard | Summarizes managed tools and visible local assets. |
| Rules | Manages global rules and tool-owned rule files. |
| Skills | Manages shared and tool-specific local Skill sources. |
| MCP | Shows and edits supported MCP server configuration entries. |
| Config | Shows tool configuration file state and paths. |
| Settings | Controls Modus preferences, enabled tools, and custom paths. |

## What Modus Does Not Do

Modus is not a model proxy, API router, account manager, subscription manager, remote Skill store, or cloud sync service. It does not upload local configuration files, manage model credentials, or route requests for other tools.

## Install

Download the latest build from [GitHub Releases](https://github.com/leon4z/Modus/releases/latest). If no release asset is available yet, run Modus from source.

Current macOS release assets are not yet signed with an Apple Developer ID or notarized by Apple. macOS may ask you to approve the app manually in Privacy & Security before the first launch.

## Development

Requirements:

- Node.js 18 or newer
- Rust toolchain
- Tauri 2 CLI, used through the project scripts

Run the development sandbox:

```bash
npm install
npm run tauri:dev
```

The development sandbox writes Modus app data to `~/.modus-dev/` and uses sandbox tool directories. To test against real local tool state before release, use the pre-release entry:

```bash
npm run tauri:pre-release
```

Common checks:

```bash
npm test
npm run build
npm run verify
```

Build commands:

```bash
npm run tauri build
npm run tauri:build:pre-release -- --config '{"version":"1.0.1-test.1"}'
```

## Documentation

- [Public docs](docs/README.md)
- [Dashboard](docs/dashboard.md)
- [Rules](docs/rules.md)
- [Skills](docs/skills.md)
- [Config](docs/config.md)
- [MCP](docs/mcp.md)
- [Settings](docs/settings.md)
- [Changelog](docs/changelog.md)

## License

MIT
