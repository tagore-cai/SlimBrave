[English](README.md) | **简体中文**

<div align="center">

# SlimBrave - Revived

  <img src="https://i.postimg.cc/QCyWVFGN/SlimBrave.png" alt="SlimBrave Lion Logo" width="200"/>

一款轻量级工具，让你完全掌控 Brave（及 Chrome）浏览器。封禁遥测、强制执行严格隐私标准、剥离内置臃肿功能——全部通过一个简洁的界面完成。

支持 Windows 与 macOS —— 原生 Rust / egui 实现。
</div>

<br>

[![Release](https://img.shields.io/github/v/release/xXSalamanderXx/SlimBrave?style=for-the-badge)](https://github.com/xXSalamanderXx/SlimBrave/releases)
![](https://img.shields.io/badge/Rust-000000?style=for-the-badge&labelColor=ffffff&logoColor=000000&logo=rust)
![](https://img.shields.io/badge/macOS-000000?style=for-the-badge&labelColor=ffffff&logoColor=000000&logo=apple)
![](https://img.shields.io/badge/Windows-0078D6?style=for-the-badge&labelColor=ffffff&logoColor=0078D6&logo=windows)
[![License](https://img.shields.io/badge/License-GPL--3.0-blue?style=for-the-badge)](./LICENSE)

[![](https://img.shields.io/static/v1?label=Sponsor&message=%E2%9D%A4&logo=GitHub&color=%23fe8e86)](https://buymeacoffee.com/SinZZzz)

## SlimBrave Revived - Windows
[![Slimbrave-Windows.png](https://i.postimg.cc/5yNPg6pc/Slimbrave-Windows.png)](https://postimg.cc/Pp9zrfBK)

## SlimBrave Revived - macOS
[![Slimbrave-mac-OS.png](https://i.postimg.cc/cChH34Lq/Slimbrave-mac-OS.png)](https://postimg.cc/xNknDQmg)

> [!IMPORTANT]
> 本工具目前不面向 Linux 构建。
> Linux 上推荐使用 Brave Origin（免费，开箱即用即可精简 Brave）。

<details>
<summary> 环境要求 </summary>

## 原生（Rust）版本

- Rust 1.75+（cargo）
- macOS 11+ / Windows 10+
- 已安装 Brave 或 Chrome（自动检测；未检测到时回退到默认渠道）

> 原始 Python / PowerShell 实现位于 `reference/`，仅作历史参考。Rust 版本为当前活跃版本。

</details>

## ✨ 功能特性

### 核心策略管理

- [x] **48 项精选策略 + 15 项站点权限** —— 分为遥测 / 隐私与安全 / Brave 功能 / 性能与臃肿四组，英文名与 Python 原版一致
- [x] **数据驱动的目录** —— 通过 JSON（`assets/catalog.json` + 用户覆盖层）增删策略，无需改代码
- [x] **快捷预设** —— 高隐私 / 高安全
- [x] **从 Brave 拉取设置** —— 回读当前生效的策略（受管偏好 + 遗留 defaults 域）
- [x] **策略管理窗口** —— 随时启用/禁用任意策略
- [x] **导入 / 导出设置** —— JSON 格式，与 Python 版格式兼容
- [x] **状态持久化** —— 启动时恢复上次应用的设置

### 平台支持

- [x] **macOS** —— `.mobileconfig` 配置描述文件（官方支持的强制策略机制）
- [x] **Windows** —— 注册表策略（HKCU，无需提权）
- [x] **双浏览器** —— Brave（Release/Beta/Dev/Nightly）与 Chrome（Stable/Beta/Dev/Canary）
- [x] **应用前关闭 Brave** —— 优雅退出 → TERM → KILL 分级（macOS）/ taskkill（Windows）
- [x] **轻量重置** —— 移除描述文件 + 受管 plist + 遗留域名键

### 数据与智能

- [x] **官方策略模板** —— 从 Brave ADMX/ADML 与 Chromium 定义拉取最新元数据
- [x] **三层合并** —— 用户 > 远程 > 内置，字段级覆盖
- [x] **主题** —— 深色 / 浅色 / 跟随系统，JSON 令牌覆盖
- [x] **i18n** —— 中文 & English（Fluent），应用内切换语言
- [x] **打包** —— `scripts/package_macos.sh` 生成 `.app` / `.dmg`

### 未实现 / 受限

- [ ] **硬重置**（隔离用户数据）—— 有意不实现：破坏性高，超出策略管理器定位
- [ ] **运行时缓存清理** —— 属硬重置范畴，未实现
- [ ] **Windows 端策略模板拉取** —— Rust 拉取模块仅限 macOS/Linux；Windows 可用 `tools/fetch_policies.py`
- [ ] **Linux 支持** —— 未构建；Linux 可参考 Brave Origin

## 🚀 使用方法

## 原生（Rust）版本

环境要求：Rust 1.75+（cargo）。

```sh
cargo run          # 开发模式
cargo run --release
cargo test         # 43 项测试
```

### macOS 快速上手

1. 在顶栏选择浏览器与渠道。
2. 勾选策略（或使用高隐私 / 高安全预设）。
3. 点击**应用设置** —— 生成并打开 `.mobileconfig` 配置描述文件。
4. 在**系统设置 → 隐私与安全性 → 描述文件**中安装（每个渠道一次）。
5. 点击**从 Brave 拉取设置** —— 顶栏徽标变绿即表示描述文件已激活。

> 描述文件由 macOS 管理：再次应用即更新，**重置所有设置**或系统设置中可移除。

### 打包

```sh
./scripts/package_macos.sh           # 生成 dist/SlimBrave.app
./scripts/package_macos.sh --dmg     # 另生成压缩 .dmg
```

生成的 bundle 为 ad-hoc 签名（本机使用）；正式分发请使用 Developer ID 签名。

### 架构（DDD 分层）

```
src/
├── main.rs                    入口
├── application/
│   └── app.rs                 应用层：SlimBraveApp 状态与用例
├── domain/                    领域层（不依赖上层）
│   ├── mod.rs                 PlatformKind、Browser 枚举
│   ├── catalog.rs             JSON 驱动的策略目录 + 三层合并
│   ├── payload.rs             策略载荷构建/清洗/应用
│   └── state.rs               UI 状态、快照、预设
├── infrastructure/
│   ├── platform.rs            macOS plist / Windows 注册表读写
│   ├── profile.rs             macOS .mobileconfig 生成与安装（macOS）
│   ├── fetch.rs               拉取官方策略模板（非 Windows）
│   └── i18n.rs                Fluent 本地化（中/英）
└── presentation/
    ├── ui.rs                  egui 渲染（面板、策略管理）
    └── theme.rs               设计令牌主题（深/浅、JSON 覆盖）
assets/
├── catalog.json               内置策略目录（48 策略、15 权限、预设）
└── i18n/*.ftl                 Fluent 消息文件
reference/                     原始 Python / PowerShell 实现
tools/                         fetch_policies.py（Windows 备用）
```

### 数据层（优先级从高到低）

| 层 | 位置 | 用途 |
|---|---|---|
| 用户 | `~/.config/slimbrave/catalog.json` | 字段级覆盖、自定义策略、`remove` 列表 |
| 远程 | `~/Library/Caches/slimbrave/catalog.remote-{browser}.json`（macOS）/ `%LOCALAPPDATA%\slimbrave\...`（Windows） | 官方模板元数据（拉取所得） |
| 内置 | `assets/catalog.json` | 离线兜底 |

- `~/.config/slimbrave/theme.json` —— 主题令牌覆盖（`bg`、`panel`、`button_success` 等）
- `~/.config/slimbrave/config.json` —— 持久化的主题偏好
- `~/.config/slimbrave/SlimBraveState.json` —— 上次应用的设置

新增策略 = 在用户目录清单中加一行（名称/说明/类型自动取自官方模板）。
删除策略 = 加入 `remove` 列表，或在策略管理窗口中关闭。

### 策略数据源

- Brave：官方 `policy_templates.zip`（ADMX/ADML）+ Chromium 定义
- Chrome：Chromium `policy_definitions`（顶栏切换浏览器）

---

### 为什么 SlimBrave 重要

在浏览器日益臃肿的时代，SlimBrave 把控制权**交还给你**：

🚀 **更快的浏览** —— 移除不必要的功能。

🛡️ **更强的隐私与安全** —— 细粒度控制。

⚙️ **透明的定制** —— 没有隐藏设置。

---

<p align="center">
  <b>⭐ 给仓库点个 Star • ☕ 支持开发 • 🚀 探索更多项目</b>
</p>

## ⭐ 支持我们

如果这个仓库对你有帮助，请考虑给它一个 **Star**！
这对项目的发展和持续更新很有帮助。🚀

## ☕ 支持开发

如果你想支持我的工作，可以在这里**请我喝杯咖啡**：
[☕ buymeacoffee.com/SinZZzz](https://buymeacoffee.com/SinZZzz)

## 🔍 我的其他项目

[🔎 RLSBB-Search-Plus](https://github.com/xXSalamanderXx/RLSBB-Search-Plus)

[🎬 HDEncode-Search-Plus](https://github.com/xXSalamanderXx/HDEncode-Search-Plus)

[🦎 salamander-trackers](https://github.com/xXSalamanderXx/salamander-trackers)

[📷️ Caesium Image Compressor - Linux](https://github.com/xXSalamanderXx/caesium-image-compressor-linux)

---

## 🙌 致谢

感谢原作者的贡献：

[ltx0101/SlimBrave](https://github.com/ltx0101/SlimBrave)

---

## 免责声明

本项目按现状提供，不提供任何形式的保证。

您需自行负责使用方式，并确保使用符合适用的法律、规则或政策。

作者与贡献者对因使用本项目而产生的任何索赔、损害或其他问题概不负责。

## 许可证 📄

采用 **GPL-3.0** 许可证。
完整许可证见：[GPL-3.0 License](https://github.com/xXSalamanderXx/SlimBrave/blob/main/LICENSE)
