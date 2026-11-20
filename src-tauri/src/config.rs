use crate::AppState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(rename = "gamePath")]
    pub game_path: Option<String>,
}

#[derive(Serialize)]
pub struct InitResult {
    #[serde(rename = "gamePath")]
    pub game_path: Option<String>,
    #[serde(rename = "modsDir")]
    pub mods_dir: Option<String>,
    /// 从游戏目录的 release_info.json 读到的版本号（如 v0.113.1）
    #[serde(rename = "gameVersion")]
    pub game_version: Option<String>,
    /// 检测到的创意工坊内容目录数量，0 表示没有订阅或没装 Steam 版
    #[serde(rename = "workshopDirs")]
    pub workshop_dirs: usize,
}

fn config_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("STS2ModManager")
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn load_config() -> Config {
    let p = config_path();
    if p.exists() {
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(cfg) = serde_json::from_str(&content) {
                return cfg;
            }
        }
    }
    Config::default()
}

pub fn save_config(cfg: &Config) {
    let dir = config_dir();
    let _ = fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = fs::write(config_path(), json);
    }
}

fn detect_game_path() -> Option<String> {
    // 库发现逻辑统一在 steam 模块里，这里只负责"没有用户配置时自动找一次"
    crate::steam::find_game_dir().map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn app_init(state: tauri::State<'_, AppState>) -> InitResult {
    let cfg = load_config();
    let detected = match cfg.game_path {
        Some(ref gp) if Path::new(gp).exists() => Some(gp.clone()),
        _ => detect_game_path(),
    };

    if let Some(ref gp) = detected {
        let mut state_gp = state.game_path.lock().unwrap();
        *state_gp = Some(gp.clone());
        // 自动探测到的路径要落盘，否则每次启动都重新找
        if cfg.game_path.as_deref() != Some(gp.as_str()) {
            let mut new_cfg = cfg;
            new_cfg.game_path = Some(gp.clone());
            save_config(&new_cfg);
        }
    }

    let game_dir = detected.as_ref().map(PathBuf::from);
    let mods_dir = game_dir
        .as_ref()
        .map(|p| p.join("mods").to_string_lossy().to_string());
    let game_version = game_dir
        .as_ref()
        .and_then(|p| crate::steam::game_version(p));
    let workshop_dirs = crate::steam::workshop_content_dirs().len();

    InitResult {
        game_path: detected,
        mods_dir,
        game_version,
        workshop_dirs,
    }
}

#[tauri::command]
pub async fn app_select_game_path(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Option<InitResult>, String> {
    let dialog = app.dialog();
    let folder = dialog
        .file()
        .set_title("选择 Slay the Spire 2 游戏目录")
        .blocking_pick_folder();

    if let Some(folder_path) = folder {
        let gp = folder_path.to_string();
        let mut state_gp = state.game_path.lock().unwrap();
        *state_gp = Some(gp.clone());

        let mut cfg = load_config();
        cfg.game_path = Some(gp.clone());
        save_config(&cfg);

        let game_dir = PathBuf::from(&gp);
        let mods_dir = game_dir.join("mods").to_string_lossy().to_string();
        let game_version = crate::steam::game_version(&game_dir);
        let workshop_dirs = crate::steam::workshop_content_dirs().len();
        Ok(Some(InitResult {
            game_path: Some(gp),
            mods_dir: Some(mods_dir),
            game_version,
            workshop_dirs,
        }))
    } else {
        Ok(None)
    }
}
