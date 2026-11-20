<div align="center">

# 🃏 STS2 Mod Manager

**给杀戮尖塔 2 玩家的 MOD 管理工具**

不用再手动复制文件夹了。装、删、开、关，拖进来就完事。

[![Release](https://img.shields.io/github/v/release/Inlve/sts2-mod-manager?style=flat-square&color=7c3aed)](https://github.com/Inlve/sts2-mod-manager/releases)
[![Stars](https://img.shields.io/github/stars/Inlve/sts2-mod-manager?style=flat-square&color=f59e0b)](https://github.com/Inlve/sts2-mod-manager)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

<br>

<img src="docs/preview-mods.png" width="90%" alt="MOD 管理界面" />

<br><br>

<img src="docs/preview-saves.png" width="90%" alt="存档管理界面" />

<br><br>

<img src="docs/preview-logs.png" width="90%" alt="游戏日志界面" />

</div>

<br>

---

<br>

## ✨ 亮点

| | 功能 | 说明 |
|---|---|---|
| 📦 | **MOD 管理** | 安装 / 卸载 / 启用 / 禁用，支持拖拽 `.zip` 或文件夹直接安装 |
| 🎮 | **双来源管理** | 自动识别 **Steam 创意工坊**订阅的 MOD 与**本地 / Nexus 手动安装**的 MOD，可分开筛选、统一启停 |
| 🧩 | **重复 ID 检测** | 同一 MOD 同时来自工坊和本地时会标出来，避免游戏重复加载 |
| �️ | **风险信号** | 自动标出缺失依赖，区分框架前置 / 玩法改动 / 资源类 MOD，一眼看清能不能安全启用 |
| 🔗 | **依赖跳转** | 详情面板中点击依赖项直接跳转到对应 MOD，并显示所需最低版本 |
| 🔀 | **智能排序** | 按名称 / 依赖问题优先 / 影响玩法 / 分类 / 大小排序 |
| 🖼️ | **双视图模式** | 卡片网格视图（默认）和双栏列表视图随时切换 |
| 🧭 | **首次引导** | 3 步卡片式引导流程：选目录 → 装 MOD → 启动验证，新手友好 |
| ⚠️ | **操作确认** | 卸载、批量卸载、应用配置、还原备份、存档转换等风险操作弹出确认，说明后果再执行 |
| �💾 | **存档管理** | 普通存档 & MOD 存档分开展示，一键导出备份、导入还原 |
| 🔁 | **存档转换** | 普通存档 ↔ MOD 存档 一键互转（可含回放），覆盖前自动备份原存档 |
| 📋 | **游戏日志** | 实时查看最新日志，出问题秒定位 |
| 🔍 | **崩溃分析** | 游戏退出后自动扫日志，精确到哪个 MOD 炸了 |
| 🌐 | **MOD 翻译** | 英文描述看不懂？一键翻译成中文 |
| ⚙️ | **配置档案** | 保存 MOD 启用方案，联机 / 单机随时切换 |
| 🚀 | **启动游戏** | Steam 正版、非 Steam 版都支持，自动识别 |

<br>

## 📥 下载

👉 [**点这里下载最新版**](https://github.com/Inlve/sts2-mod-manager/releases)

> 安装包约 4MB，内置 WebView2 引导。Win10 / Win11 直接运行。

<br>

## 🚀 上手

1. 双击打开，管理器会自动找到你的游戏
2. 找不到？左下角点一下手动选目录，只需选一次
3. 装 MOD — 点「安装 MOD」选文件，或者**直接拖进窗口**
4. 开玩

<br>

## 🛠 本地开发

```bash
npm install          # 装依赖
npm run tauri:dev    # 开发模式（Rsbuild dev server + Tauri 热重载）
npm run tauri:build  # 打包 NSIS 安装包 + 免安装 exe
npm run build        # 只构建前端（产物在 dist/）
```

> 打包需要 [Rust](https://rustup.rs/) + [VS Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（勾选 C++ 桌面开发）

<br>

## 📐 技术栈

```
前端    React 19 + TailwindCSS 3 + Lucide Icons
构建    Rsbuild 2（Rspack + SWC，已移除 Babel/Webpack）
后端    Rust (Tauri v2)
打包    NSIS 安装包 + 免安装 exe
```

<br>

## 📁 项目结构

```
src/                前端源码 (React)
  index.jsx         入口：先挂载 window.api，再渲染 React
  index.html        页面模板（Rsbuild 注入资源）
  tauri-api.js      window.api 桥接层，UI 唯一的原生能力入口
  components/       UI 组件
src-tauri/          后端源码 (Rust)
  src/              mods / saves / steam / modsettings / game / logs …
rsbuild.config.mjs  前端构建配置
dist/               前端构建产物（gitignore）
docs/               截图素材
```

<br>

## 📄 关于本项目

本项目是 [ImogeneOctaviap794/sts2-mod-manager](https://github.com/ImogeneOctaviap794/sts2-mod-manager) 的延续分支。
原仓库自 2026-04 起停止更新，本仓库在其基础上继续维护并独立发布版本，
问题反馈与新功能请提交到[本仓库的 Issues](https://github.com/Inlve/sts2-mod-manager/issues)。

原仓库未提供 LICENSE 文件；本仓库的改动与新增内容按 [MIT](LICENSE) 发布。

<br>

## 🤝 参与贡献

提交遵循 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/)，
流程与发布步骤见 [CONTRIBUTING.md](CONTRIBUTING.md)。

版本变更记录统一维护在 [CHANGELOG.md](CHANGELOG.md)。

<br>

---

<div align="center">
<sub>MIT License · 给个 ⭐ 就是最大的支持</sub>
</div>
