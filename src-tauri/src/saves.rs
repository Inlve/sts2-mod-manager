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
    if let Ok(entries) = fs::read_dir(current) {
        for entry in entries.flatten() {
            let path = entry.path();
            let rel = path
                .strip_prefix(base)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                let _ = zip_writer.add_directory(&format!("{}/", rel), options);
                add_dir_to_zip(zip_writer, base, &path, options)?;
            } else {
                zip_writer.start_file(&rel, options).map_err(|e| e.to_string())?;
                let mut f = fs::File::open(&path).map_err(|e| e.to_string())?;
                let mut buf = Vec::new();
                f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                std::io::Write::write_all(zip_writer, &buf).map_err(|e| e.to_string())?;
            }
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

/// 递归复制目录，返回 (文件数, 字节数)。
fn copy_tree_counted(src: &Path, dst: &Path) -> Result<(usize, u64), String> {
    let mut files = 0usize;
    let mut bytes = 0u64;
    if !src.exists() {
        return Ok((0, 0));
    }
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            let (f, b) = copy_tree_counted(&from, &to)?;
            files += f;
            bytes += b;
        } else {
            if let Some(parent) = to.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let size = fs::copy(&from, &to).map_err(|e| e.to_string())?;
            files += 1;
            bytes += size;
        }
    }
    Ok((files, bytes))
}

#[tauri::command]
pub fn saves_convert(opts: SaveConvertOpts) -> ConvertResult {
    // 先把进错误结果要用到的字段复制出来，避免闭包借用 opts 后
    // 又在末尾移出 opts.slot / opts.direction（借用检查会不答应）
    let slot_echo = opts.slot.clone();
    let direction_echo = opts.direction.clone();
    let fail = |msg: String| ConvertResult {
        success: false,
        error: Some(msg),
        slot: slot_echo.clone(),
        direction: direction_echo.clone(),
        copied_files: 0,
        copied_bytes: 0,
        backup_path: None,
    };

    // 存档正被游戏读写时搬家会损坏数据
    if crate::game::is_game_running() {
        return fail("游戏正在运行，请先退出游戏再转换存档".into());
    }

    let user_dir = match get_steam_user_dir() {
        Some(d) => d,
        None => return fail("未找到游戏存档目录".into()),
    };

    if !["profile1", "profile2", "profile3"].contains(&opts.slot.as_str()) {
        return fail(format!("未知的存档槽位: {}", opts.slot));
    }

    let (src_root, dst_root) = match opts.direction.as_str() {
        "to_modded" => (user_dir.clone(), user_dir.join("modded")),
        "to_normal" => (user_dir.join("modded"), user_dir.clone()),
        other => return fail(format!("未知的转换方向: {}", other)),
    };

    let src = src_root.join(&opts.slot);
    let dst = dst_root.join(&opts.slot);

    if !src.exists() {
        return fail(format!(
            "源存档 {} 不存在（{}）",
            opts.slot,
            if opts.direction == "to_modded" {
                "普通存档里没有这个槽位"
            } else {
                "MOD 存档里没有这个槽位"
            }
        ));
    }

    let tag = if opts.direction == "to_modded" {
        format!("{}_normal", opts.slot)
    } else {
        format!("{}_modded", opts.slot)
    };

    // ── 备份目标槽位 ──
    let mut backup_path: Option<String> = None;
    if opts.backup && dst.exists() {
        let dir = get_save_backup_dir();
        let path = dir.join(format!("convert_{}_before_{}.zip", tag, timestamp_string()));
        let file = match fs::File::create(&path) {
            Ok(f) => f,
            Err(e) => return fail(format!("创建备份失败: {}", e)),
        };
        let mut zip_writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let listing: SimpleResult = {
            match add_dir_to_zip(
                &mut zip_writer,
                dst.parent().unwrap_or(&dst),
                &dst,
                options,
            ) {
                Ok(_) => SimpleResult { success: true, error: None },
                Err(e) => SimpleResult { success: false, error: Some(e) },
            }
        };
        if let Some(e) = listing.error {
            return fail(format!("备份失败: {}", e));
        }
        if let Err(e) = zip_writer.finish() {
            return fail(format!("备份写入失败: {}", e));
        }
        backup_path = Some(path.to_string_lossy().to_string());
    }

    // ── 替换式复制：先清掉目标槽位，避免残留上一次的当前局文件 ──
    if dst.exists() {
        if let Err(e) = fs::remove_dir_all(&dst) {
            return fail(format!("清理目标槽位失败: {}", e));
        }
    }
    if let Err(e) = fs::create_dir_all(&dst) {
        return fail(format!("创建目标槽位失败: {}", e));
    }

    let mut copied_files = 0usize;
    let mut copied_bytes = 0u64;

    match copy_tree_counted(&src.join("saves"), &dst.join("saves")) {
        Ok((f, b)) => {
            copied_files += f;
            copied_bytes += b;
        }
        Err(e) => return fail(format!("复制存档失败: {}", e)),
    }

    if opts.include_replays {
        match copy_tree_counted(&src.join("replays"), &dst.join("replays")) {
            Ok((f, b)) => {
                copied_files += f;
                copied_bytes += b;
            }
            Err(e) => return fail(format!("复制回放失败: {}", e)),
        }
    }

    if copied_files == 0 {
        return fail("源槽位里没有可转换的存档文件".into());
    }

    ConvertResult {
        success: true,
        error: None,
        slot: opts.slot,
        direction: opts.direction,
        copied_files,
        copied_bytes,
        backup_path,
    }
}
