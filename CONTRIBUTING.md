# Contributing

## 提交规范

本仓库统一使用 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/)。
PR 标题会被 CI 校验（`Conventional PR title`），合并方式建议 **Squash and merge**，
这样最终进入 `main` 的提交信息就是 PR 标题。

```
<type>(<scope>)!: <subject>

<body>

<footer>
```

### type

| type | 用途 |
|---|---|
| `feat` | 新功能 |
| `fix` | 修 bug |
| `docs` | 只改文档 |
| `refactor` | 重构（不改行为） |
| `perf` | 性能优化 |
| `test` | 测试相关 |
| `build` | 构建系统 / 依赖 |
| `ci` | CI 配置 |
| `chore` | 杂项（版本号、脚手架等） |
| `revert` | 回滚 |

### scope（可选）

按功能域划分，常用：`mods`、`saves`、`logs`、`ui`、`steam`、`build`、`ci`、`repo`。

### 破坏性变更

在 type 后加 `!`，并在 footer 里写 `BREAKING CHANGE: <说明>`。

```
refactor(build)!: 移除 Electron，改为纯 Tauri

BREAKING CHANGE: 不再提供 Electron 便携版，安装包改为 NSIS。
```

### 示例

```
feat(mods): 支持 Steam 创意工坊与本地 MOD 双来源管理
fix(saves): 修正存档最后修改时间显示为 Invalid Date
perf(logs): 日志列表改用 @tanstack/react-virtual 虚拟化
ci(release): 由 CHANGELOG 自动生成 Release 说明
```

## 分支与 PR

1. 从 `main` 切分支：`feat/xxx`、`fix/xxx`
2. 提交按上面的规范拆分，尽量一个提交只做一件事
3. 开 PR，标题写成规范的 Conventional Commit
4. CI 必须通过（前端构建 + `cargo check` + 标题校验）

## 本地开发

```bash
npm ci
npm run tauri:dev    # 带热更新的开发模式
npm run build        # 只构建前端（产物在 dist/）
npm run test:ui      # 日志视图 UI 冒烟测试（需先运行 npm run dev）
```

打包需要 [Rust](https://rustup.rs/) 与
[VS Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（勾选 C++ 桌面开发）。

## 版本与发布

版本号出现在三个文件里，用脚本一次改完，避免漏改导致安装包版本与代码版本不一致：

```bash
npm run version:bump -- 1.2.0
```

发布流程（维护者）：

1. `npm run version:bump -- <版本>` 同步三处版本号
2. 在 `CHANGELOG.md` 里把 `[Unreleased]` 的内容整理成 `[<版本>] - YYYY-MM-DD`，
   并更新底部的 compare 链接
3. 提交：`chore(release): <版本>`
4. 打 tag 并推送：

   ```bash
   git tag v<版本>
   git push origin main --follow-tags
   ```

5. tag 推送后，`Release` 工作流会自动：
   - 构建 NSIS 安装包与免安装 exe
   - 从 `CHANGELOG.md` 提取该版本段落作为 Release 正文
   - 生成 `checksums.txt` 并把产物挂到 Release

> 发布说明只维护 `CHANGELOG.md` 一份，不要再新建 `release_notes_vX.Y.Z.md`。

> 升级安装前请先关闭正在运行的程序：安装器覆盖 `sts2-mod-manager.exe` 时若文件被占用，
> 会报 `Error opening file for writing`。
