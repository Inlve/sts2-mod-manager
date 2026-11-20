# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **创意工坊 MOD 支持**：自动扫描 Steam 创意工坊目录（`steamapps/workshop/content/2868840`），与本地 / Nexus 手动安装的 MOD 统一管理，可按来源筛选
- **重复 ID 警告**：同一 MOD 同时来自工坊和本地时标出「ID 重复」，避免游戏重复加载
- **存档转换**：普通存档与 MOD 存档一键互转，支持同时转换回放、覆盖前自动备份
- **日志自动刷新**：默认 3 秒轮询，内容无变化时不重渲染，不打断翻看历史
- 版本号校验：详情面板显示 MOD 声明的最低游戏版本

### Changed

- **构建链重写**：移除 Electron 与 Babel/Webpack，改为纯 Tauri + Rsbuild（Rspack + SWC）
- **React 升级到 19**
- **日志视图改为虚拟列表**：无论多少行只渲染视口附近的行；默认聚焦最新日志，向上翻自动暂停跟随，可一键回到底部
- **MOD 列表排序**：始终优先展示已启用的 MOD，所选排序方式只作用于组内
- 游戏目录探测改用 Steam `libraryfolders.vdf` 解析，多账号时取最近活跃的账号
- 游戏版本改读 `release_info.json`，不再解析日志

### Fixed

- 存档「最后修改」显示 `Invalid Date`（原实现返回的是 `SystemTime` 的调试字符串）
- 存档导出文件名时间戳的近似日期计算在跨月/闰年会算错
- 解压 MOD 时的 Zip Slip 路径穿越风险
- 新版 manifest 的 `dependencies`（对象数组）被静默丢弃，导致缺失依赖检测失效
- 多 Steam 账号时存档串档
- 横向滚动条与纵向样式不一致（缺少 `height` 声明）
- `tauri-plugin-single-instance` 已声明但从未初始化，导致可多开

### Removed

- Electron 主进程、预加载脚本与 electron-builder 打包配置
- Webpack / Babel 构建配置与产物
- 单版本发布的 `release_notes_*.md`（改为本文件统一维护）

## [1.1.0] - 2026-04-08

### Added

- MOD 风险信号：缺失依赖的卡片顶部显示红色横幅，明确标出「缺失依赖，无法正常工作」
- MOD 自动分类：框架前置（被其他 MOD 依赖）、玩法改动（含 DLL 或影响玩法）、资源类，各有独立配色与图标
- 依赖跳转：详情面板中的依赖项可点击，直接跳转到对应 MOD
- 智能排序：按名称 / 依赖问题优先 / 影响玩法优先 / 按分类 / 按大小
- 双视图模式：卡片网格（默认）与双栏列表，列表视图支持多选与批量操作
- 首次使用引导：选目录 → 装 MOD → 启动验证 3 步引导
- 风险操作二次确认：卸载、批量卸载、应用配置档案、还原备份

### Changed

- 默认窗口尺寸改为 1400×900
- MOD 安装改为智能解压到子文件夹
- 配置档案功能整合进 MOD 管理页面

## [1.0.0] - 2026-04-06

### Added

- 首个版本：MOD 管理、崩溃分析、存档查看、日志查看

[Unreleased]: https://github.com/Inlve/sts2-mod-manager/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/Inlve/sts2-mod-manager/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/Inlve/sts2-mod-manager/releases/tag/v1.0.0
