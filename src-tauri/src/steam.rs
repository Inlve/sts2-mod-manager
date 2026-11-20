//! Steam 安装布局探测。
//!
//! 旧版 config.rs 在自身内部硬编码 5 条 Steam 根路径，且只用来找游戏目录。
//! 引入创意工坊支持后，游戏目录和工坊内容目录需要共用同一套"库发现"逻辑，
//! 所以抽到这里，config.rs 与 mods.rs 都从这里取。

use std::fs;
use std::path::{Path, PathBuf};

/// Slay the Spire 2 的 Steam AppID。
pub const APP_ID: &str = "2868840";

/// 游戏在 `steamapps/common` 下的目录名。
const GAME_DIR_NAME: &str = "Slay the Spire 2";

/// 常见 Steam 安装位置。这些只是"入口"，真正的库列表从 libraryfolders.vdf 解析。
fn candidate_roots() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    for p in [
        r"C:\Program Files (x86)\Steam",
        r"C:\Program Files\Steam",
        r"C:\Steam",
        r"D:\Steam",
        r"D:\Program Files (x86)\Steam",
        r"D:\SteamLibrary",
        r"E:\SteamLibrary",
        r"E:\Steam",
        r"F:\SteamLibrary",
        r"F:\Steam",
        r"G:\SteamLibrary",
    ] {
        v.push(PathBuf::from(p));
    }
    // 允许用户用环境变量覆盖，便于非常规安装。
    for key in ["STEAM_PATH", "STEAM_ROOT", "STEAM_HOME"] {
        if let Ok(p) = std::env::var(key) {
            if !p.trim().is_empty() {
                v.insert(0, PathBuf::from(p));
            }
        }
    }
    v
}

/// 从 libraryfolders.vdf 里抽出所有 `"path" "..."` 的值。
///
/// VDF 是 Valve 的自定义格式，这里不做完整解析——只需要 path 字段，
/// 逐行找 `"path"` 后取紧随其后的第一个带引号字符串即可。
fn parse_library_paths(vdf_content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in vdf_content.lines() {
        let Some(pos) = line.find("\"path\"") else {
            continue;
        };
        let rest = &line[pos + 6..];
        // 取第一个完整引号串
        let Some(start) = rest.find('"') else { continue };
        let after = &rest[start + 1..];
        let Some(end) = after.find('"') else { continue };
        let value = after[..end].replace("\\\\", "\\");
        if !value.trim().is_empty() {
            out.push(value);
        }
    }
    out
}

/// 去重后追加一个候选库目录。
fn add_unique(libs: &mut Vec<PathBuf>, p: PathBuf) {
    if p.exists() && !libs.iter().any(|x| x == &p) {
        libs.push(p);
    }
}

/// 返回所有可用的 Steam 库根目录（去重、只保留真实存在的）。
pub fn library_folders() -> Vec<PathBuf> {
    let mut libs: Vec<PathBuf> = Vec::new();

    for root in candidate_roots() {
        if !root.exists() {
            continue;
        }
        // 根目录自身也算一个库
        add_unique(&mut libs, root.clone());

        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(content) = fs::read_to_string(&vdf) {
            for p in parse_library_paths(&content) {
                add_unique(&mut libs, PathBuf::from(p));
            }
        }
    }

    libs
}

/// 找到 Slay the Spire 2 的安装目录。优先返回含可执行文件的那个。
pub fn find_game_dir() -> Option<PathBuf> {
    let mut fallback: Option<PathBuf> = None;
    for lib in library_folders() {
        let dir = lib.join("steamapps").join("common").join(GAME_DIR_NAME);
        if !dir.exists() {
            continue;
        }
        if dir.join("SlayTheSpire2.exe").exists() {
            return Some(dir);
        }
        if fallback.is_none() {
            fallback = Some(dir);
        }
    }
    fallback
}

/// 所有存在的创意工坊内容目录：`<lib>/steamapps/workshop/content/2868840`。
///
/// 同一份游戏一般只有一个，但订阅内容可能散落在多个库里，所以返回 Vec。
pub fn workshop_content_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for lib in library_folders() {
        let d = lib
            .join("steamapps")
            .join("workshop")
            .join("content")
            .join(APP_ID);
        if d.is_dir() && !dirs.iter().any(|x| x == &d) {
            dirs.push(d);
        }
    }
    dirs
}

/// 读取游戏版本。来源是安装目录下的 `release_info.json`，比解析日志可靠得多。
pub fn game_version(game_dir: &Path) -> Option<String> {
    let p = game_dir.join("release_info.json");
    let content = fs::read_to_string(p).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    v.get("version")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
}

/// 创意工坊详情页地址。
pub fn workshop_url(workshop_id: &str) -> String {
    format!(
        "https://steamcommunity.com/sharedfiles/filedetails/?id={}",
        workshop_id
    )
}

// ── 游戏用户数据目录 ──
//
// 这两层原来在 saves.rs 里各有一份实现，modsettings.rs 又要用，
// 于是统一放到这里，避免三处各写一遍。

/// `%APPDATA%\SlayTheSpire2`（其它平台为对应 config 目录）。
pub fn user_data_dir() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("SlayTheSpire2"))
}

/// `%APPDATA%\SlayTheSpire2\steam\<steamid64>`。
///
/// 目录名是纯数字的 SteamID64；多账号时取**修改时间最新**的那个，
/// 而不是像旧代码那样盲取 `users[0]`——那会在多账号机器上串档。
pub fn steam_user_dir() -> Option<PathBuf> {
    let steam_dir = user_data_dir()?.join("steam");
    if !steam_dir.exists() {
        return None;
    }

    let mut candidates: Vec<(PathBuf, u64)> = Vec::new();
    for entry in fs::read_dir(&steam_dir).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.chars().all(|c| c.is_ascii_digit()) || name.is_empty() {
            continue;
        }
        let mtime = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        candidates.push((entry.path(), mtime));
    }

    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    candidates.into_iter().next().map(|(p, _)| p)
}
