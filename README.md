**English** | [简体中文](README.zh-CN.md)

<div align="center">

# SlimBrave - Revived

  <img src="https://i.postimg.cc/QCyWVFGN/SlimBrave.png" alt="SlimBrave Lion Logo" width="200"/>

A lightweight utility designed to give you ultimate control over your Brave (and Chrome) browser. Lock down telemetry, enforce strict privacy standards, and strip away built-in browser bloatware—all from a single, clean interface.

Supported on Windows and macOS — native Rust / egui.
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
> This tool is currently not built for Linux.
> On Linux, it is recommended to use Brave Origin. Brave Origin is free to use on Linux and debloats Brave out of the box.

<details>
<summary> Requirements </summary>

## Native (Rust) version

- Rust 1.75+ (cargo)
- macOS 11+ / Windows 10+
- Brave or Chrome installed (auto-detected; falls back to the default channel)

> The original Python / PowerShell implementations live in `reference/` and are
> kept for historical reference only. The Rust version is the active one.

</details>


## ✨ Features

### Core policy management

- [x] **48 curated policies + 15 site permissions** — grouped into Telemetry / Privacy & Security / Brave Features / Performance & Bloat, name-matched to the original Python version
- [x] **Data-driven catalog** — add/remove policies via JSON (`assets/catalog.json` + user overrides), no code changes
- [x] **Quick presets** — High Privacy / High Security
- [x] **Pull Settings from Brave** — reads back currently applied policies (managed prefs + legacy defaults domain)
- [x] **Policy Manager window** — enable/disable any policy on the fly
- [x] **Import / Export settings** — JSON, compatible with the Python version's format
- [x] **State persistence** — last applied settings restored on launch

### Platform support

- [x] **macOS** — `.mobileconfig` configuration profile (officially supported forced-policy mechanism)
- [x] **Windows** — registry policies (HKCU, no elevation needed)
- [x] **Dual browser** — Brave (Release/Beta/Dev/Nightly) and Chrome (Stable/Beta/Dev/Canary)
- [x] **Close Brave before apply** — graceful quit → TERM → KILL fallback (macOS) / taskkill (Windows)
- [x] **Light Reset** — remove profile + managed plists + legacy domain keys

### Data & intelligence

- [x] **Official policy templates** — fetch up-to-date metadata from Brave ADMX/ADML and Chromium definitions
- [x] **Three-layer merge** — user > remote > builtin, field-level overrides
- [x] **Theme** — dark / light / follow-system, JSON token overrides
- [x] **i18n** — English & 中文 (Fluent), language switcher in-app
- [x] **Packaging** — `scripts/package_macos.sh` produces `.app` / `.dmg`

### Not planned / limited

- [ ] **Hard Reset** (quarantine user data) — intentionally not implemented: destructive, out of scope for a policy manager
- [ ] **Runtime cache clearing** — part of Hard Reset, not implemented
- [ ] **Policy template fetch on Windows** — Rust fetch module is macOS/Linux-only; Windows can use `tools/fetch_policies.py`
- [ ] **Linux support** — not built; see Brave Origin for Linux

## 🚀 How to Use

## Rust (native) version

Requirements: Rust 1.75+ (cargo).

```sh
cargo run          # dev
cargo run --release
cargo test         # 43 tests
```

### macOS quick start

1. Pick the browser and channel in the top bar.
2. Tick policies (or use High Privacy / High Security presets).
3. Click **Apply Settings** — a `.mobileconfig` configuration profile is generated and opened.
4. Install it in **System Settings → Privacy & Security → Profiles** (one-time per channel).
5. Click **Pull Settings from Brave** — the top-right badge turns green when the profile is active.

> The profile is managed by macOS: update it by applying again, remove it with **Reset All Settings** or in System Settings.

### Packaging

```sh
./scripts/package_macos.sh           # SlimBrave.app in dist/
./scripts/package_macos.sh --dmg     # plus a compressed .dmg
```

The bundle is ad-hoc signed for local use; distribution builds should sign with a Developer ID.

### Architecture (DDD-style layering)

```
src/
├── main.rs                    entry point
├── application/
│   └── app.rs                 application layer: SlimBraveApp state & use cases
├── domain/                    domain layer (no dependencies upward)
│   ├── mod.rs                 PlatformKind, Browser enums
│   ├── catalog.rs             JSON-driven policy catalog + three-layer merge
│   ├── payload.rs             build/sanitize/apply policy payloads
│   └── state.rs               UI state, snapshots, presets
├── infrastructure/
│   ├── platform.rs            macOS plist / Windows registry read-write
│   ├── profile.rs             macOS .mobileconfig generation & install (macOS)
│   ├── fetch.rs               pull official policy templates (non-Windows)
│   └── i18n.rs                Fluent-based localization (en/zh)
└── presentation/
    ├── ui.rs                  egui rendering (panels, policy manager)
    └── theme.rs               design-token theming (dark/light, JSON overrides)
assets/
├── catalog.json               builtin policy catalog (48 features, 15 permissions, presets)
└── i18n/*.ftl                 Fluent message files
reference/                     original Python / PowerShell implementations
tools/                         fetch_policies.py (Python fallback for Windows)
```

### Data layers (highest priority first)

| Layer | Location | Purpose |
|---|---|---|
| user | `~/.config/slimbrave/catalog.json` | field-level overrides, custom policies, `remove` list |
| remote | `~/Library/Caches/slimbrave/catalog.remote-{browser}.json` (macOS) / `%LOCALAPPDATA%\slimbrave\...` (Windows) | official template metadata (fetched) |
| builtin | `assets/catalog.json` | offline fallback |

- `~/.config/slimbrave/theme.json` — theme token overrides (`bg`, `panel`, `button_success`, ...)
- `~/.config/slimbrave/config.json` — persisted theme preference
- `~/.config/slimbrave/SlimBraveState.json` — last applied settings

Adding a policy = one line in the user catalog (name/tooltip/type come from official templates).
Removing one = add its key to the `remove` list, or toggle it off in the Policy Manager window.

### Policy sources

- Brave: official `policy_templates.zip` (ADMX/ADML) + Chromium definitions
- Chrome: Chromium `policy_definitions` (browser switcher in the top bar)

### Why SlimBrave Matters

In an era of increasingly bloated browsers, SlimBrave puts **you** back in control:

🚀 **Faster browsing** by removing unnecessary features.

🛡️ **Enhanced privacy and security** through granular controls.

⚙️ **Transparent customization** without hidden settings.

---

<p align="center">
  <b>⭐ Star the repo • ☕ Support development • 🚀 Explore more projects</b>
</p>

## ⭐ Show Your Support

If this repo has helped you, please consider giving it a **star** on GitHub!  
It really helps show support, motivates future updates, and encourages continued development. 🚀

Every ⭐ makes a difference and means a lot. Thanks for helping this project grow! 🙌

## ☕ Support Development

If you'd like to support my work even more, you can **buy me a coffee** here:  
[☕ buymeacoffee.com/SinZZzz](https://buymeacoffee.com/SinZZzz)

Your support helps keep development active and appreciated. 💙

## 🔍 Check Out My Other Repos

You might also like these projects:

[🔎 RLSBB-Search-Plus](https://github.com/xXSalamanderXx/RLSBB-Search-Plus)

[🎬 HDEncode-Search-Plus](https://github.com/xXSalamanderXx/HDEncode-Search-Plus)

[🦎 salamander-trackers](https://github.com/xXSalamanderXx/salamander-trackers)

[📷️ Caesium Image Compressor - Linux](https://github.com/xXSalamanderXx/caesium-image-compressor-linux)

---

## 🙌 Credit

Acknowledgment and thanks goes to the original creator:

[ltx0101/SlimBrave](https://github.com/ltx0101/SlimBrave)

---

## Disclaimer

This project is provided as-is, with no guarantees or warranties of any kind.

You are responsible for how you use the contents of this repository and for making sure your usage complies with any applicable laws, rules, or policies.

The author and contributors are not liable for any claims, damages, or other issues arising from the use of this project.

## License 📄

Licensed under the **GPL-3.0** license.  
See the full license here: [GPL-3.0 License](https://github.com/xXSalamanderXx/SlimBrave/blob/main/LICENSE)
