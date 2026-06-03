<div align="center">

# Modus

**本地优先的 macOS 桌面应用，用来管理 AI 编程工具的规则、技能、MCP 和配置。**

<p>
  <img alt="macOS" src="https://img.shields.io/badge/macOS-desktop-111111">
  <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2-24C8DB">
  <img alt="本地优先" src="https://img.shields.io/badge/%E6%9C%AC%E5%9C%B0%E4%BC%98%E5%85%88-yes-2E7D32">
  <img alt="License MIT" src="https://img.shields.io/badge/license-MIT-blue">
</p>
[English](README.md) · [文档](docs/README.md) · [更新日志](docs/changelog.md)

</div>

![Modus 仪表盘](docs/assets/dashboard_cn.png)

## 为什么需要 Modus？

AI 编程工具通常会把规则、技能、MCP 条目和配置文件放在不同的本机路径里。手动维护这些文件很容易出错，尤其是多个工具共享相似概念、但存储结构并不相同时。

Modus 提供一个本地工作台，让你集中查看这些资产，先预览文件变更，再确认写入。

## 功能

- **工具总览**：查看已管理的 AI 编程工具，以及可见的规则、技能、MCP 和配置资产。
- **规则管理**：维护一份全局规则，并在写入前预览会影响哪些工具文件。
- **工具原生规则**：查看和编辑各工具自己的规则文件。
- **技能管理**：浏览共享目录和工具目录里的技能，并对单个技能执行复制、安装、卸载、编辑或删除。
- **MCP 编辑**：查看和编辑支持的 MCP Server 配置片段，保存前校验并备份。
- **配置查看**：查看本机配置文件路径和健康状态。
- **设置**：管理语言、主题、启用工具、自定义路径、日志入口和手动更新检查。

## 截图

| 规则 | 技能 |
| --- | --- |
| ![规则](docs/assets/rules_cn.png) | ![技能](docs/assets/skills_cn.png) |

| MCP | 配置 |
| --- | --- |
| ![MCP](docs/assets/mcp_cn.png) | ![配置](docs/assets/config_cn.png) |

## Modus 管理什么？

| 页面 | 说明 |
| --- | --- |
| 仪表盘 | 汇总已管理工具和可见的本地资产。 |
| 规则 | 管理全局规则和工具自己的规则文件。 |
| 技能 | 管理共享目录和工具目录里的本地技能来源。 |
| MCP | 查看和编辑支持的 MCP Server 配置条目。 |
| 配置 | 查看工具配置文件状态和路径。 |
| 设置 | 控制 Modus 偏好、启用工具和自定义路径。 |

## Modus 不做什么？

Modus 不是模型代理、API 路由器、账号管理器、订阅管理器、远程技能商店或云同步服务。它不会上传本机配置文件，不管理模型凭据，也不替其它工具转发请求。

## 安装

从 [GitHub Releases](https://github.com/leon4z/Modus/releases/latest) 下载最新构建。如果暂时没有可用的 release 资产，请先从源码运行。

当前 macOS release 资产暂未使用 Apple Developer ID 签名，也暂未经过 Apple 公证。首次启动时，macOS 可能会要求你在“隐私与安全性”中手动允许打开。

## 开发

前提条件：

- Node.js 18 或更新版本
- Rust toolchain
- Tauri 2 CLI，通过项目脚本调用

运行开发沙盒：

```bash
npm install
npm run tauri:dev
```

开发沙盒会把 Modus 应用数据写入 `~/.modus-dev/`，并使用沙盒工具目录。发布前如果需要验证真实本机工具状态，使用预发布入口：

```bash
npm run tauri:pre-release
```

常用检查：

```bash
npm test
npm run build
npm run verify
```

构建命令：

```bash
npm run tauri build
npm run tauri:build:pre-release -- --config '{"version":"1.0.1-test.1"}'
```

## 文档

- [公开文档](docs/README.md)
- [仪表盘](docs/dashboard.md)
- [规则](docs/rules.md)
- [技能](docs/skills.md)
- [配置](docs/config.md)
- [MCP](docs/mcp.md)
- [设置](docs/settings.md)
- [更新日志](docs/changelog.md)

## License

MIT
