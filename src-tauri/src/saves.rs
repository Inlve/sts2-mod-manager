use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize)]
pub struct CharacterStat {
    pub id: String,
    pub name: String,
    pub wins: u64,
    pub losses: u64,
    #[serde(rename = "maxAscension")]
    pub max_ascension: u64,
    pub playtime: u64,
    #[serde(rename = "bestStreak")]
    pub best_streak: u64,
}

#[derive(Serialize)]
pub struct ProgressSummary {
    #[serde(rename = "totalPlaytime")]
    pub total_playtime: u64,
    #[serde(rename = "floorsClimbed")]
    pub floors_climbed: u64,
    #[serde(rename = "currentScore")]
    pub current_score: u64,
    #[serde(rename = "totalUnlocks")]
    pub total_unlocks: u64,
    #[serde(rename = "discoveredCards")]
    pub discovered_cards: usize,
    #[serde(rename = "discoveredRelics")]
    pub discovered_relics: usize,
    pub epochs: usize,
    pub characters: Vec<CharacterStat>,
    #[serde(rename = "uniqueId")]
    pub unique_id: String,
}

#[derive(Serialize)]
pub struct SaveSlot {
    pub slot: String,
    pub modded: bool,
    pub path: String,
    #[serde(rename = "hasProgress")]
    pub has_progress: bool,
    #[serde(rename = "hasPrefs")]
    pub has_prefs: bool,
    pub empty: bool,
    #[serde(rename = "lastModified")]
    pub last_modified: Option<String>,
    pub size: u64,
    pub summary: Option<ProgressSummary>,
}

#[derive(Serialize)]
pub struct BackupEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub time: String,
}

#[derive(Serialize)]
pub struct SavesResult {
    pub slots: Vec<SaveSlot>,
    pub backups: Vec<BackupEntry>,
}

#[derive(Serialize)]
pub struct SimpleResult {
    pub success: bool,
    pub error: Option<String>,
}

fn get_appdata() -> Option<PathBuf> {
    dirs::config_dir()
}

fn get_steam_user_dir() -> Option<PathBuf> {
    // 统一走 steam 模块（多账号时取最近活跃的那个）
    crate::steam::steam_user_dir()
}

fn get_save_backup_dir() -> PathBuf {
    let appdata = get_appdata().unwrap_or_else(|| PathBuf::from("."));
    let dir = appdata.join("STS2ModManager").join("save_backups");
    let _ = fs::create_dir_all(&dir);
    dir
}

const CHARACTER_NAMES: &[(&str, &str)] = &[
    ("CHARACTER.IRONCLAD", "铁甲战士"),
    ("CHARACTER.SILENT", "沉默猎手"),
    ("CHARACTER.REGENT", "摄政王"),
    ("CHARACTER.NECROBINDER", "缚灵师"),
    ("CHARACTER.DEFECT", "缺陷体"),
    ("CHARACTER.WATCHER", "观察者"),
];

fn char_name(id: &str) -> String {
    for (k, v) in CHARACTER_NAMES {
        if *k == id {
            return v.to_string();
        }
    }
    id.split('.').last().unwrap_or(id).to_string()
}

fn parse_progress(path: &Path) -> Option<ProgressSummary> {
    if !path.exists() {
        return None;
    }
    let mut content = fs::read_to_string(path).ok()?;
    if content.starts_with('\u{feff}') {
        content = content[3..].to_string();
    }
    let data: serde_json::Value = serde_json::from_str(&content).ok()?;

    let characters: Vec<CharacterStat> = data
        .get("character_stats")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    let id = c.get("id")?.as_str()?.to_string();
                    let wins = c.get("total_wins").and_then(|v| v.as_u64()).unwrap_or(0);
                    let losses = c.get("total_losses").and_then(|v| v.as_u64()).unwrap_or(0);
                    if wins == 0 && losses == 0 {
                        return None;
                    }
                    Some(CharacterStat {
                        name: char_name(&id),
                        id,
                        wins,
                        losses,
                        max_ascension: c.get("max_ascension").and_then(|v| v.as_u64()).unwrap_or(0),
                        playtime: c.get("playtime").and_then(|v| v.as_u64()).unwrap_or(0),
                        best_streak: c.get("best_win_streak").and_then(|v| v.as_u64()).unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Some(ProgressSummary {
        total_playtime: data.get("total_playtime").and_then(|v| v.as_u64()).unwrap_or(0),
        floors_climbed: data.get("floors_climbed").and_then(|v| v.as_u64()).unwrap_or(0),
        current_score: data.get("current_score").and_then(|v| v.as_u64()).unwrap_or(0),
        total_unlocks: data.get("total_unlocks").and_then(|v| v.as_u64()).unwrap_or(0),
        discovered_cards: data.get("discovered_cards").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
        discovered_relics: data.get("discovered_relics").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
        epochs: data.get("epochs").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
        characters,
        unique_id: data.get("unique_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    })
}

fn walk_size_and_mtime(dir: &Path) -> (u64, u64) {
    let mut total_size = 0u64;
    let mut last_modified = 0u64;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let (s, m) = walk_size_and_mtime(&p);
                total_size += s;
                if m > last_modified {
                    last_modified = m;
                }
            } else if let Ok(meta) = p.metadata() {
                total_size += meta.len();
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                if mtime > last_modified {
                    last_modified = mtime;
                }
            }
        }
    }
    (total_size, last_modified)
}

fn scan_save_slot(user_dir: &Path, slot: &str, modded: bool) -> Option<SaveSlot> {
    let prefix = if modded {
        user_dir.join("modded").join(slot)
    } else {
        user_dir.join(slot)
    };
    if !prefix.exists() {
        return None;
    }

    let saves_dir = prefix.join("saves");
    let progress_path = saves_dir.join("progress.save");
    let has_progress = progress_path.exists();
    let has_prefs = saves_dir.join("prefs.save").exists();

    let (mut total_size, last_modified) = if saves_dir.exists() {
        walk_size_and_mtime(&saves_dir)
    } else {
        (0, 0)
    };

    let replays_dir = prefix.join("replays");
    if replays_dir.exists() {
        let (rs, _) = walk_size_and_mtime(&replays_dir);
        total_size += rs;
    }

    let summary = parse_progress(&progress_path);

    let last_mod_str = if last_modified > 0 {
        // Simple ISO string from millis
        let secs = (last_modified / 1000) as i64;
        let naive = chrono_from_timestamp(secs);
        Some(naive)
    } else {
        None
    };

    Some(SaveSlot {
        slot: slot.to_string(),
        modded,
        path: prefix.to_string_lossy().to_string(),
        has_progress,
        has_prefs,
        empty: !has_progress && !has_prefs,
        last_modified: last_mod_str,
        size: total_size,
        summary,
    })
}

/// 把 Unix 秒数格式化成 `YYYY-MM-DDTHH:MM:SS`。
///
/// 说明：旧实现返回的是 `format!("{:?}", SystemTime)`，即
/// `SystemTime { tv_sec: ... }` 调试字符串，前端 `new Date()` 解析失败后
/// 会显示 Invalid Date——这是从 Electron 版迁移到 Rust 时引入的回归。
/// 这里用 Howard Hinnant 的 civil-from-days 算法自己算日期，
/// 避免为了一个时间格式化再引入 chrono 依赖。
fn chrono_from_timestamp(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    // civil_from_days
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        year, m, d, h, mi, s
    )
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    chrono_from_timestamp(secs)
}

/// 用于文件名的时间戳：`YYYY-MM-DDTHH-MM-SS`（冒号在 Windows 文件名里非法）。
fn timestamp_string() -> String {
    now_iso().replace(':', "-")
}

#[tauri::command]
pub fn saves_scan() -> SavesResult {
    let user_dir = match get_steam_user_dir() {
        Some(d) => d,
        None => return SavesResult { slots: vec![], backups: vec![] },
    };

    let mut slots = Vec::new();
    for s in &["profile1", "profile2", "profile3"] {
        if let Some(slot) = scan_save_slot(&user_dir, s, false) {
            slots.push(slot);
        }
        if let Some(slot) = scan_save_slot(&user_dir, s, true) {
            slots.push(slot);
        }
    }

    let backup_dir = get_save_backup_dir();
    let mut backups = Vec::new();
    if backup_dir.exists() {
        if let Ok(entries) = fs::read_dir(&backup_dir) {
            let mut bk_files: Vec<_> = entries
                .flatten()
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map(|s| s.ends_with(".zip"))
                        .unwrap_or(false)
                })
                .collect();
            bk_files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
            for entry in bk_files {
                if let Ok(meta) = entry.metadata() {
                    let mtime = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| chrono_from_timestamp(d.as_secs() as i64))
                        .unwrap_or_default();
                    backups.push(BackupEntry {
                        name: entry.file_name().to_string_lossy().to_string(),
                        path: entry.path().to_string_lossy().to_string(),
                        size: meta.len(),
                        time: mtime,
                    });
                }
            }
        }
    }

    SavesResult { slots, backups }
}

fn add_dir_to_zip(
    zip_writer: &mut zip::ZipWriter<fs::File>,
    base: &Path,
    current: &Path,
    options: zip::write::SimpleFileOptions,
) -> Result<(), String> {
    for entry in fs::read_dir(current).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        let rel = path
            .strip_prefix(base)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if kind.is_dir() {
            zip_writer
                .add_directory(format!("{}/", rel), options)
                .map_err(|e| e.to_string())?;
            add_dir_to_zip(zip_writer, base, &path, options)?;
        } else if kind.is_file() {
            zip_writer
                .start_file(&rel, options)
                .map_err(|e| e.to_string())?;
            let mut f = fs::File::open(&path).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, zip_writer).map_err(|e| e.to_string())?;
        } else {
            return Err(format!("无法备份特殊文件 {}", path.display()));
        }
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct SaveExportOpts {
    pub slot: String,
    pub modded: bool,
}

#[tauri::command]
pub async fn saves_export(
    app: tauri::AppHandle,
    opts: SaveExportOpts,
) -> Result<SimpleResult, String> {
    let user_dir = match get_steam_user_dir() {
        Some(d) => d,
        None => return Ok(SimpleResult { success: false, error: Some("未找到游戏存档目录".into()) }),
    };

    let prefix = if opts.modded {
        user_dir.join("modded").join(&opts.slot)
    } else {
        user_dir.join(&opts.slot)
    };
    if !prefix.exists() {
        return Ok(SimpleResult { success: false, error: Some("该存档槽位为空".into()) });
    }

    let tag = if opts.modded {
        format!("{}_modded", opts.slot)
    } else {
        opts.slot.clone()
    };
    let ts = timestamp_string();
    let default_name = format!("STS2_Save_{}_{}.zip", tag, ts);

    let dialog = app.dialog();
    let save_path = dialog
        .file()
        .set_title("导出存档")
        .set_file_name(&default_name)
        .add_filter("ZIP Archive", &["zip"])
        .blocking_save_file();

    let dest = match save_path {
        Some(p) => p.to_string(),
        None => return Ok(SimpleResult { success: false, error: None }),
    };

    let file = fs::File::create(&dest).map_err(|e| e.to_string())?;
    let mut zip_writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Meta
    let meta_json = serde_json::json!({
        "slot": opts.slot,
        "modded": opts.modded,
        "exportTime": now_iso(),
    });
    zip_writer
        .start_file("_meta.json", options)
        .map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut zip_writer, meta_json.to_string().as_bytes())
        .map_err(|e| e.to_string())?;

    // Add save folder with slot name as prefix
    add_dir_to_zip(&mut zip_writer, prefix.parent().unwrap_or(&prefix), &prefix, options)?;
    zip_writer.finish().map_err(|e| e.to_string())?;

    Ok(SimpleResult { success: true, error: None })
}

#[tauri::command]
pub async fn saves_import(
    app: tauri::AppHandle,
    opts: SaveExportOpts,
) -> Result<SimpleResult, String> {
    let user_dir = match get_steam_user_dir() {
        Some(d) => d,
        None => return Ok(SimpleResult { success: false, error: Some("未找到游戏存档目录".into()) }),
    };

    let dialog = app.dialog();
    let file = dialog
        .file()
        .set_title(&format!("导入存档到 {}", opts.slot))
        .add_filter("ZIP Archive", &["zip"])
        .blocking_pick_file();

    let zip_path = match file {
        Some(p) => p.to_string(),
        None => return Ok(SimpleResult { success: false, error: None }),
    };

    let target_dir = if opts.modded {
        user_dir.join("modded").join(&opts.slot)
    } else {
        user_dir.join(&opts.slot)
    };

    // Backup current slot
    if target_dir.exists() {
        let backup_dir = get_save_backup_dir();
        let tag = if opts.modded {
            format!("{}_modded", opts.slot)
        } else {
            opts.slot.clone()
        };
        let ts = timestamp_string();
        let backup_path = backup_dir.join(format!("auto_backup_{}_{}.zip", tag, ts));

        let bk_file = fs::File::create(&backup_path).map_err(|e| e.to_string())?;
        let mut bk_zip = zip::ZipWriter::new(bk_file);
        let bk_options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        add_dir_to_zip(
            &mut bk_zip,
            target_dir.parent().unwrap_or(&target_dir),
            &target_dir,
            bk_options,
        )?;
        bk_zip.finish().map_err(|e| e.to_string())?;
    }

    // Extract zip
    let file = fs::File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    // Find source slot prefix
    let mut folders: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            let name = entry.name().to_string();
            if let Some(first) = name.split('/').next() {
                if !folders.contains(&first.to_string()) {
                    folders.push(first.to_string());
                }
            }
        }
    }
    let source_slot = folders
        .iter()
        .find(|f| f.starts_with("profile"))
        .cloned()
        .unwrap_or_else(|| folders.first().cloned().unwrap_or_default());

    // Re-open archive for extraction
    let file = fs::File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        if name == "_meta.json" {
            continue;
        }
        // Remap source slot to target
        let rel_path = if name.starts_with(&format!("{}/", source_slot)) {
            name[source_slot.len() + 1..].to_string()
        } else {
            name
        };
        let dest = target_dir.join(&rel_path);
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        fs::write(&dest, &buf).map_err(|e| e.to_string())?;
    }

    Ok(SimpleResult { success: true, error: None })
}

#[tauri::command]
pub fn saves_delete_backup(backup_path: String) -> SimpleResult {
    let p = Path::new(&backup_path);
    if p.exists() {
        if let Err(e) = fs::remove_file(p) {
            return SimpleResult {
                success: false,
                error: Some(e.to_string()),
            };
        }
    }
    SimpleResult { success: true, error: None }
}

// ── 普通存档 ↔ MOD 存档 转换 ──
//
// 游戏本身把两套进度完全隔离：普通档在 `steam/<id>/profileN`，
// MOD 档在 `steam/<id>/modded/profileN`，互相不可见。
// 玩家想"把 MOD 档的进度搬到普通档"只能自己复制文件夹，
// 这里把它做成带备份的一键操作。

#[derive(Deserialize)]
pub struct SaveConvertOpts {
    /// profile1 / profile2 / profile3
    pub slot: String,
    /// "to_modded"（普通 → MOD）| "to_normal"（MOD → 普通）
    pub direction: String,
    /// 是否连回放一起搬
    #[serde(rename = "includeReplays", default = "default_true")]
    pub include_replays: bool,
    /// 覆盖前是否先自动备份目标槽位
    #[serde(default = "default_true")]
    pub backup: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize)]
pub struct ConvertResult {
    pub success: bool,
    pub error: Option<String>,
    pub slot: String,
    pub direction: String,
    #[serde(rename = "copiedFiles")]
    pub copied_files: usize,
    #[serde(rename = "copiedBytes")]
    pub copied_bytes: u64,
    #[serde(rename = "backupPath")]
    pub backup_path: Option<String>,
}

fn copy_tree_counted(src: &Path, dst: &Path) -> Result<(usize, u64), String> {
    let copy = || -> std::io::Result<(usize, u64)> {
        if !fs::symlink_metadata(src)?.is_dir() {
            return Err(std::io::Error::other("需要普通目录"));
        }
        fs::create_dir(dst)?;
        let mut files = 0;
        let mut bytes = 0;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let from = entry.path();
            let to = dst.join(entry.file_name());
            let kind = entry.file_type()?;
            if kind.is_dir() {
                let (f, b) = copy_tree_counted(&from, &to).map_err(std::io::Error::other)?;
                files += f;
                bytes += b;
            } else if kind.is_file() {
                let expected = entry.metadata()?.len();
                let copied = fs::copy(&from, &to)?;
                if copied != expected || fs::metadata(&to)?.len() != expected {
                    return Err(std::io::Error::other(format!(
                        "文件大小校验失败 {}",
                        from.display()
                    )));
                }
                files += 1;
                bytes += copied;
            } else {
                return Err(std::io::Error::other(format!(
                    "不支持的文件 {}",
                    from.display()
                )));
            }
        }
        Ok((files, bytes))
    };
    copy().map_err(|e| format!("复制 {} 失败：{e}", src.display()))
}

fn backup_slot(dst: &Path, dir: &Path, tag: &str) -> Result<PathBuf, String> {
    let temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(temp.reopen().map_err(|e| e.to_string())?);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    add_dir_to_zip(&mut zip, dst.parent().unwrap(), dst, options)?;
    zip.finish()
        .map_err(|e| e.to_string())?
        .sync_all()
        .map_err(|e| e.to_string())?;
    let unique = temp.path().file_name().unwrap().to_string_lossy();
    let path = dir.join(format!(
        "convert_{tag}_before_{}_{unique}.zip",
        timestamp_string()
    ));
    temp.persist_noclobber(&path).map_err(|e| e.to_string())?;
    Ok(path)
}

fn install_staged(work: tempfile::TempDir, dst: &Path, had_target: bool) -> Result<(), String> {
    let old = work.path().join("original");
    if had_target {
        fs::rename(dst, &old).map_err(|e| format!("无法移开目标槽位：{e}"))?;
    }
    if let Err(e) = fs::rename(work.path().join("staged"), dst) {
        if had_target {
            if let Err(restore) = fs::rename(&old, dst) {
                let _ = work.keep(); // 自动清理会删除唯一的原件
                return Err(format!(
                    "替换失败：{e}。还原失败：{restore}。原槽位保存在 {}",
                    old.display()
                ));
            }
        }
        return Err(format!("替换目标槽位失败：{e}"));
    }
    Ok(())
}

fn convert_slot(
    src: &Path,
    dst: &Path,
    backup_dir: &Path,
    opts: &SaveConvertOpts,
    backup_path: &mut Option<String>,
) -> Result<(usize, u64), String> {
    let parent = dst.parent().unwrap();
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let work = tempfile::Builder::new()
        .prefix(".sts2-convert-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let staged = work.path().join("staged");
    fs::create_dir(&staged).map_err(|e| e.to_string())?;
    let saves = staged.join("saves");
    let (mut files, mut bytes) = copy_tree_counted(&src.join("saves"), &saves)?;
    // 与扫描界面的空槽位判定一致，并拒绝仅含空文件的源。
    let has_save = ["progress.save", "prefs.save"].iter().any(|name| {
        fs::metadata(saves.join(name)).is_ok_and(|meta| meta.is_file() && meta.len() > 0)
    });
    if !has_save {
        return Err("源槽位里没有可转换的存档数据".into());
    }
    let replays = src.join("replays");
    if opts.include_replays && replays.try_exists().map_err(|e| e.to_string())? {
        let (f, b) = copy_tree_counted(&replays, &staged.join("replays"))?;
        files += f;
        bytes += b;
    }
    let had_target = match fs::symlink_metadata(dst) {
        Ok(meta) if meta.is_dir() => true,
        Ok(_) => return Err("目标槽位不是目录".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(e.to_string()),
    };
    if opts.backup && had_target {
        let kind = if opts.direction == "to_modded" {
            "modded"
        } else {
            "normal"
        };
        let path = backup_slot(dst, backup_dir, &format!("{}_{kind}", opts.slot))
            .map_err(|e| format!("备份失败：{e}"))?;
        *backup_path = Some(path.display().to_string());
    }
    install_staged(work, dst, had_target)?;
    Ok((files, bytes))
}

#[tauri::command]
pub fn saves_convert(opts: SaveConvertOpts) -> ConvertResult {
    static CONVERSION: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let mut backup_path = None;
    let outcome = (|| {
        let _guard = CONVERSION.try_lock().map_err(|_| "已有存档转换正在执行")?;
        if crate::game::is_game_running() {
            return Err("游戏正在运行，请先退出游戏再转换存档".into());
        }
        let user = get_steam_user_dir().ok_or("未找到游戏存档目录")?;
        if !["profile1", "profile2", "profile3"].contains(&opts.slot.as_str()) {
            return Err(format!("未知的存档槽位：{}", opts.slot));
        }
        let normal = user.join(&opts.slot);
        let modded = user.join("modded").join(&opts.slot);
        let (src, dst) = match opts.direction.as_str() {
            "to_modded" => (normal, modded),
            "to_normal" => (modded, normal),
            other => return Err(format!("未知的转换方向：{other}")),
        };
        convert_slot(&src, &dst, &get_save_backup_dir(), &opts, &mut backup_path)
    })();
    let success = outcome.is_ok();
    let error = outcome.as_ref().err().map(|e| match &backup_path {
        Some(path) => format!("{e}。覆盖前备份：{path}"),
        None => e.clone(),
    });
    let (copied_files, copied_bytes) = outcome.unwrap_or_default();
    ConvertResult {
        slot: opts.slot,
        direction: opts.direction,
        success,
        error,
        copied_files,
        copied_bytes,
        backup_path,
    }
}

#[cfg(test)]
mod conversion_tests {
    use super::*;

    struct Fixture {
        root: tempfile::TempDir,
        src: PathBuf,
        dst: PathBuf,
        opts: SaveConvertOpts,
        backup: Option<String>,
    }

    impl Fixture {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let src = root.path().join("profile1");
            let dst = root.path().join("modded/profile1");
            fs::create_dir_all(src.join("saves")).unwrap();
            fs::create_dir_all(dst.join("saves")).unwrap();
            fs::write(dst.join("saves/old.save"), b"old").unwrap();
            Self {
                root,
                src,
                dst,
                opts: SaveConvertOpts {
                    slot: "profile1".into(),
                    direction: "to_modded".into(),
                    include_replays: true,
                    backup: true,
                },
                backup: None,
            }
        }

        fn convert(&mut self) -> Result<(usize, u64), String> {
            convert_slot(
                &self.src,
                &self.dst,
                self.root.path(),
                &self.opts,
                &mut self.backup,
            )
        }

        fn assert_original(&self) {
            assert_eq!(fs::read(self.dst.join("saves/old.save")).unwrap(), b"old");
            assert!(!self.dst.join("saves/progress.save").exists());
        }
    }

    #[test]
    fn empty_source_with_replays_preserves_target() {
        let mut f = Fixture::new();
        fs::create_dir(f.src.join("replays")).unwrap();
        fs::write(f.src.join("replays/run"), b"replay").unwrap();
        assert!(f.convert().is_err());
        f.assert_original();
    }

    #[test]
    fn replay_copy_failure_after_save_copy_preserves_target_without_zip() {
        let mut f = Fixture::new();
        f.opts.backup = false;
        fs::write(f.src.join("saves/progress.save"), b"new").unwrap();
        fs::write(f.src.join("replays"), b"not a directory").unwrap();
        assert!(f.convert().is_err());
        f.assert_original();
    }

    #[test]
    fn swap_failure_restores_original() {
        let f = Fixture::new();
        let work = tempfile::tempdir_in(f.dst.parent().unwrap()).unwrap();
        // 缺少 staged，第二次 rename 必定失败。
        assert!(install_staged(work, &f.dst, true).is_err());
        f.assert_original();
    }

    #[test]
    fn successful_swap_replaces_whole_slot_and_backs_up_old_bytes() {
        let mut f = Fixture::new();
        fs::write(f.src.join("saves/progress.save"), b"new").unwrap();
        fs::create_dir(f.src.join("replays")).unwrap();
        fs::write(f.src.join("replays/run"), b"replay").unwrap();
        assert_eq!(f.convert().unwrap(), (2, 9));
        assert_eq!(fs::read(f.dst.join("replays/run")).unwrap(), b"replay");
        assert!(!f.dst.join("saves/old.save").exists());
        let mut zip =
            zip::ZipArchive::new(fs::File::open(f.backup.as_ref().unwrap()).unwrap()).unwrap();
        let mut old = Vec::new();
        zip.by_name("profile1/saves/old.save")
            .unwrap()
            .read_to_end(&mut old)
            .unwrap();
        assert_eq!(old, b"old");
    }

    #[cfg(windows)]
    #[test]
    fn locked_target_returns_completed_backup() {
        use std::os::windows::fs::OpenOptionsExt;
        let mut f = Fixture::new();
        fs::write(f.src.join("saves/progress.save"), b"new").unwrap();
        // FILE_FLAG_BACKUP_SEMANTICS allows a directory handle; deny delete sharing.
        let _lock = fs::OpenOptions::new()
            .read(true)
            .custom_flags(0x02000000)
            .share_mode(3)
            .open(&f.dst)
            .unwrap();
        assert!(f.convert().unwrap_err().contains("无法移开"));
        f.assert_original();
        assert!(Path::new(f.backup.as_ref().unwrap()).is_file());
    }
}
