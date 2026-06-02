# Modus

Modus 是一个 macOS 桌面应用，用来集中管理本机多个 AI 编程工具的规则、Skills、配置文件和 MCP 配置。

## 当前社区版范围

Modus 社区版聚焦本地核心工作流：

- 查看已检测到的 AI 编程工具，以及规则、Skills、配置和 MCP 的概览状态。
- 维护一份全局规则，并在写入前预览即将影响的工具文件。
- 查看和编辑各工具自己的规则文件。
- 浏览本机 Skills，查看共享目录和工具目录里的文件树与内容，并执行单个 Skill 的复制、移除和删除操作。
- 查看各工具的配置文件属性和 MCP Server 配置。
- 管理基础设置，包括语言、主题、工具启用状态、工具路径、运行日志入口和应用更新检查。

所有会写入本机文件的核心操作都应先展示预览，由用户确认后再执行。

## 使用方式

首次启动后，Modus 会扫描本机已安装的 AI 编程工具。你可以在设置中启用或停用要管理的工具，也可以调整工具路径。

主要页面：

| 页面 | 说明 |
|------|------|
| Dashboard | 查看已管理工具和本地资产概览 |
| Rules | 管理全局规则和各工具自己的规则文件 |
| Skills | 浏览本机 Skills，并处理共享目录和工具目录中的单个 Skill 操作 |
| Config | 查看工具配置文件 |
| MCP | 查看 MCP Server 配置 |
| Settings | 管理语言、主题、工具、日志入口和应用更新 |

## 开发

```bash
npm install
npm run tauri:dev
```

日常开发默认使用开发沙盒，应用数据写入 `~/.modus-dev/`，工具数据来自沙盒工具目录，并使用独立于正式版的原生应用标识。需要在发布前验证真实本机工具时，使用预发布版本入口：

```bash
npm run tauri:pre-release
```

常用检查：

```bash
npm test
npm run build
npm run verify
```

发布构建：

```bash
npm run tauri build
```

预发布安装包构建：

```bash
npm run tauri:build:pre-release -- --config '{"version":"0.1.1-test.1"}'
```

## 前提条件

- Node.js 18 或更新版本
- Rust toolchain
- Tauri 2 CLI，可通过项目脚本调用

## 文档

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
