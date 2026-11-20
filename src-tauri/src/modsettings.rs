//! 游戏自身的 Mod 开关状态。
//!
//! Slay the Spire 2 把"哪些 Mod 被启用"记在自己的设置文件里：
//! `<用户数据>\steam\<steamid64>\settings.save` → `mod_settings.mod_list[]`，
//! 每项形如 `{"id": "BaseLib", "is_enabled": true, "source": "steam_workshop"}`。
//!
//! 这是唯一能对**创意工坊 Mod**生效的启停手段：工坊内容是 Steam 托管的，
//! 移动/改名会被 Steam 校验后重新下载，不能像本地 Mod 那样靠搬目录来禁用。
//! 本地 Mod 也读这里，这样在游戏内被关掉的本地 Mod 在工具里也能显示为已禁用，
//! 避免"界面上开着、游戏里其实没加载"的错觉。

use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

/// 从 settings.save 读出来的 Mod 开关快照。
#[derive(Default)]
pub struct ModSettings {
    /// mod id -> 是否启用（只包含文件里明确登记过的条目）
    pub enabled: HashMap<String, bool>,
    /// 登记过的 id 集合，用于判断"能否安全改写"
    pub registered: HashSet<String>,
}

fn settings_path() -> Option<PathBuf> {
    Some(crate::steam::steam_user_dir()?.join("settings.save"))
}

fn backup_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("STS2ModManager")
        .join("settings_backups")
}

fn timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn read_raw() -> Option<(PathBuf, String)> {
    let path = settings_path()?;
    if !path.exists() {
        return None;
    }
    let mut content = fs::read_to_string(&path).ok()?;
    if content.starts_with('\u{feff}') {
        content = content[3..].to_string();
    }
    Some((path, content))
}

/// 读取当前 Mod 开关状态。文件不存在或损坏时返回 `None`，
/// 调用方应把 `None` 当作"没有可用的权威状态"，退回默认（全部启用）。
pub fn read() -> Option<ModSettings> {
    let (_, content) = read_raw()?;
    let v: Value = serde_json::from_str(&content).ok()?;

    let mod_settings = v.get("mod_settings")?;
    // 全局总开关（mod_settings.mods_enabled）不在快照里暴露：
    // 它在 set_enabled 里单独检查，避免误以为"单个 Mod 启用"就真的生效。

    let mut enabled = HashMap::new();
    let mut registered = HashSet::new();
    if let Some(list) = mod_settings.get("mod_list").and_then(|x| x.as_array()) {
        for item in list {
            let Some(id) = item.get("id").and_then(|x| x.as_str()) else {
                continue;
            };
            let on = item
                .get("is_enabled")
                .and_then(|x| x.as_bool())
                .unwrap_or(true);
            enabled.insert(id.to_string(), on);
            registered.insert(id.to_string());
        }
    }

    Some(ModSettings { enabled, registered })
}

/// 判断某个 mod id 是否在设置文件里登记过。
pub fn is_registered(id: &str) -> bool {
    read()
        .map(|s| s.registered.contains(id))
        .unwrap_or(false)
}

/// 改写单个 Mod 的启用状态。
///
/// - 只动 `mod_settings.mod_list[i].is_enabled`，其余字段原样保留；
/// - 改之前把原文件备份到 `%APPDATA%\STS2ModManager\settings_backups\`；
/// - 游戏正在运行时拒绝写入（否则会被游戏退出时覆盖，造成状态闪烁）；
/// - id 未登记时返回 `Err`，由调用方决定是否退回"搬目录"方案。
pub fn set_enabled(id: &str, enabled: bool) -> Result<(), String> {
    if crate::game::is_game_running() {
        return Err("游戏正在运行，请先退出游戏再修改 Mod 状态".into());
    }

    let (path, content) = read_raw().ok_or("未找到游戏设置文件 settings.save")?;

    let mut v: Value = serde_json::from_str(&content)
        .map_err(|e| format!("settings.save 解析失败: {}", e))?;

    // 游戏里还有个"启用 Mod"总开关；关着时单独启用某个 Mod 不会生效，
    // 这里提前告知，免得用户以为已经打开了。
    if enabled {
        let mods_enabled = v
            .get("mod_settings")
            .and_then(|m| m.get("mods_enabled"))
            .and_then(|x| x.as_bool())
            .unwrap_or(true);
        if !mods_enabled {
            return Err("游戏里的「启用 Mod」总开关是关闭的，请先在游戏设置中打开".into());
        }
    }

    let mod_settings = v
        .get_mut("mod_settings")
        .ok_or("settings.save 里没有 mod_settings 段")?;

    let list = mod_settings
        .get_mut("mod_list")
        .and_then(|x| x.as_array_mut())
        .ok_or("settings.save 里没有 mod_settings.mod_list 数组")?;

    let mut found = false;
    for item in list.iter_mut() {
        if item.get("id").and_then(|x| x.as_str()) == Some(id) {
            item["is_enabled"] = Value::Bool(enabled);
            found = true;
            break;
        }
    }

    if !found {
        return Err(format!(
            "设置文件里还没有登记 Mod \"{}\"（先启动一次游戏让它登记，或改用文件移动方式）",
            id
        ));
    }

    // 备份，失败就中止——宁可不改，也不要留下无法回滚的状态
    let dir = backup_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("创建备份目录失败: {}", e))?;
    let backup = dir.join(format!("settings_{}.save", timestamp()));
    fs::copy(&path, &backup).map_err(|e| format!("备份 settings.save 失败: {}", e))?;

    // 游戏写的是 CRLF 缩进 JSON，这里保持一致，减少差异
    let text = serde_json::to_string_pretty(&v)
        .map_err(|e| e.to_string())?
        .replace('\n', "\r\n");
    fs::write(&path, text).map_err(|e| format!("写入 settings.save 失败: {}", e))?;

    Ok(())
}
