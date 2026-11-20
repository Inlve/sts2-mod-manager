use crate::AppState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use tauri_plugin_dialog::DialogExt;

const DISABLED_DIR: &str = "mods_disabled";
const LEGACY_DISABLED_DIR: &str = "_disabled";

/// Mod 来源。
pub const SOURCE_WORKSHOP: &str = "workshop";
pub const SOURCE_LOCAL: &str = "local";

/// Mod 在磁盘上的归属位置。
pub const LOC_ENABLED: &str = "enabled";
pub const LOC_DISABLED: &str = "disabled";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModDependency {
    pub id: String,
    #[serde(rename = "minVersion")]
    pub min_version: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModInfo {
    pub id: Option<String>,
    pub name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    /// 依赖的 mod id 列表（保持扁平，前端既有逻辑不用改）。
    pub dependencies: Option<Vec<String>>,
    /// 依赖的完整信息（含 min_version），新版 manifest 才有。
    #[serde(rename = "dependencyVersions")]
    pub dependency_versions: Option<Vec<ModDependency>>,
    pub affects_gameplay: Option<bool>,
    pub has_dll: Option<bool>,
    pub has_pck: Option<bool>,
    pub enabled: bool,
    #[serde(rename = "instanceKey")]
    pub instance_key: String,
    #[serde(rename = "folderName")]
    pub folder_name: String,
    #[serde(rename = "isFolder")]
    pub is_folder: bool,
    pub path: String,
    pub files: Vec<String>,
    pub size: u64,
    // ── 多来源管理新增 ──
    /// "workshop" | "local"
    pub source: String,
    #[serde(rename = "workshopId")]
    pub workshop_id: Option<String>,
    #[serde(rename = "workshopUrl")]
    pub workshop_url: Option<String>,
    /// Mod 目录相对其容器（工坊条目目录 / mods 目录）的路径
    #[serde(rename = "modDir")]
    pub mod_dir: Option<String>,
    #[serde(rename = "minGameVersion")]
    pub min_game_version: Option<String>,
    /// "enabled"（在 mods/ 或工坊目录里）| "disabled"（被停放在 mods_disabled/）
    pub location: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ToggleModInfo {
    #[serde(rename = "isFolder")]
    pub is_folder: bool,
    #[serde(rename = "folderName")]
    pub folder_name: String,
    pub files: Option<Vec<String>>,
    pub enabled: bool,
    // 新增：启停需要知道是哪个 mod、哪来的
    pub id: Option<String>,
    pub source: Option<String>,
    pub location: Option<String>,
}

#[derive(Serialize)]
pub struct ModResult {
    pub success: bool,
    pub error: Option<String>,
    pub mods: Option<Vec<ModInfo>>,
    pub installed: Option<Vec<String>>,
}

fn get_mods_dir(game_path: &str) -> PathBuf {
    Path::new(game_path).join("mods")
}

fn get_disabled_dir(game_path: &str) -> PathBuf {
    let dir = Path::new(game_path).join(DISABLED_DIR);
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    dir
}

fn get_legacy_disabled_dir(game_path: &str) -> PathBuf {
    get_mods_dir(game_path).join(LEGACY_DISABLED_DIR)
}

fn migrate_legacy_disabled(game_path: &str) {
    let legacy = get_legacy_disabled_dir(game_path);
    let disabled = get_disabled_dir(game_path);
    if !legacy.exists() {
        return;
    }
    if let Ok(entries) = fs::read_dir(&legacy) {
        for entry in entries.flatten() {
            let dst = disabled.join(entry.file_name());
            if !dst.exists() {
                let _ = fs::rename(entry.path(), dst);
            }
        }
    }
    if legacy.exists() {
        if let Ok(entries) = fs::read_dir(&legacy) {
            if entries.count() == 0 {
                let _ = fs::remove_dir(&legacy);
            }
        }
    }
}

fn read_json_file(path: &Path) -> Option<serde_json::Value> {
    let mut content = fs::read_to_string(path).ok()?;
    // Strip BOM
    if content.starts_with('\u{feff}') {
        content = content[3..].to_string();
    }
    serde_json::from_str(&content).ok()
}

fn dir_size(path: &Path) -> u64 {
    let mut size = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                size += dir_size(&p);
            } else if let Ok(meta) = p.metadata() {
                size += meta.len();
            }
        }
    }
    size
}

/// 把 zip 内的相对路径安全地拼到目标目录下。
///
/// 拒绝绝对路径、盘符、`..` 以及任何非普通路径段——这正是之前缺失的
/// Zip Slip 防护（恶意压缩包里的 `Mod/../../evil` 会跳出 mods 目录）。
fn safe_join(base: &Path, rel: &str) -> Option<PathBuf> {
    let rel = rel.replace('\\', "/");
    let mut out = base.to_path_buf();
    for part in rel.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return None;
        }
        let p = Path::new(part);
        // 只接受单个普通组件，挡掉 `C:` / 绝对路径 / 分隔符残余
        match p.components().next() {
            Some(Component::Normal(seg)) if p.components().count() == 1 => out.push(seg),
            _ => return None,
        }
    }
    Some(out)
}

// ── Manifest 解析 ──

struct Manifest {
    id: String,
    name: String,
    author: Option<String>,
    version: Option<String>,
    description: Option<String>,
    dependencies: Option<Vec<String>>,
    dependency_versions: Option<Vec<ModDependency>>,
    affects_gameplay: Option<bool>,
    has_dll: Option<bool>,
    has_pck: Option<bool>,
    min_game_version: Option<String>,
}

/// 解析 manifest。
///
/// 关键兼容点：`dependencies` 有**两种历史格式**——
/// 旧版 `["BaseLib"]`，新版 `[{"id":"BaseLib","min_version":"3.4.5"}]`。
/// 旧代码只认字符串，导致新版 Mod 的依赖被静默丢弃、缺失依赖检测完全失效。
fn parse_manifest(path: &Path) -> Option<Manifest> {
    let data = read_json_file(path)?;
    let id = data.get("id")?.as_str()?.to_string();
    let name = data.get("name")?.as_str()?.to_string();

    let mut deps: Vec<String> = Vec::new();
    let mut dep_versions: Vec<ModDependency> = Vec::new();
    if let Some(arr) = data.get("dependencies").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(s) = item.as_str() {
                deps.push(s.to_string());
                dep_versions.push(ModDependency {
                    id: s.to_string(),
                    min_version: None,
                });
            } else if let Some(obj_id) = item.get("id").and_then(|v| v.as_str()) {
                let min = item
                    .get("min_version")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                deps.push(obj_id.to_string());
                dep_versions.push(ModDependency {
                    id: obj_id.to_string(),
                    min_version: min,
                });
            }
        }
    }

    Some(Manifest {
        id,
        name,
        author: data.get("author").and_then(|v| v.as_str()).map(String::from),
        version: data.get("version").and_then(|v| v.as_str()).map(String::from),
        description: data
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from),
        dependencies: if deps.is_empty() { None } else { Some(deps) },
        dependency_versions: if dep_versions.is_empty() {
            None
        } else {
            Some(dep_versions)
        },
        affects_gameplay: data.get("affects_gameplay").and_then(|v| v.as_bool()),
        has_dll: data.get("has_dll").and_then(|v| v.as_bool()),
        has_pck: data.get("has_pck").and_then(|v| v.as_bool()),
        min_game_version: data
            .get("min_game_version")
            .and_then(|v| v.as_str())
            .map(String::from),
    })
}

/// 列出目录下直接子项的文件名。
fn entries_of(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .map(|it| {
            it.flatten()
                .filter_map(|e| e.file_name().to_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// 旧代码用文件存在性推断 has_dll/has_pck；新版 manifest 会自己声明，
/// 声明优先，缺失时回退到文件探测。
fn detect_has(files: &[String], ext: &str) -> bool {
    files.iter().any(|f| f.to_lowercase().ends_with(ext))
}

// ── 本地 Mod（mods/ 与 mods_disabled/） ──

fn build_local_mod(
    full_path: &Path,
    item_name: &str,
    parked: bool,
    settings: Option<&crate::modsettings::ModSettings>,
) -> Option<ModInfo> {
    let meta = fs::metadata(full_path).ok()?;

    // 统一的"是否启用"：被停放一定算禁用；否则看游戏设置，未登记视为启用。
    let settings_enabled = |id: &str| {
        settings
            .and_then(|s| s.enabled.get(id).copied())
            .unwrap_or(true)
    };

    if meta.is_dir() {
        let entries = entries_of(full_path);
        for jf in entries.iter().filter(|f| f.ends_with(".json")) {
            let json_path = full_path.join(jf);
            if let Some(m) = parse_manifest(&json_path) {
                let enabled = !parked && settings_enabled(&m.id);
                return Some(ModInfo {
                    id: Some(m.id.clone()),
                    name: Some(m.name.clone()),
                    author: m.author.clone(),
                    version: m.version.clone(),
                    description: m.description.clone(),
                    dependencies: m.dependencies.clone(),
                    dependency_versions: m.dependency_versions.clone(),
                    affects_gameplay: m.affects_gameplay,
                    has_dll: Some(m.has_dll.unwrap_or_else(|| detect_has(&entries, ".dll"))),
                    has_pck: Some(m.has_pck.unwrap_or_else(|| detect_has(&entries, ".pck"))),
                    enabled,
                    instance_key: full_path.to_string_lossy().to_string(),
                    folder_name: item_name.to_string(),
                    is_folder: true,
                    path: full_path.to_string_lossy().to_string(),
                    files: entries.clone(),
                    size: dir_size(full_path),
                    source: SOURCE_LOCAL.to_string(),
                    workshop_id: None,
                    workshop_url: None,
                    mod_dir: None,
                    min_game_version: m.min_game_version.clone(),
                    location: if parked { LOC_DISABLED } else { LOC_ENABLED }.to_string(),
                });
            }
        }
    } else if item_name.ends_with(".json") && !item_name.starts_with('.') {
        let m = parse_manifest(full_path)?;
        let base_name = item_name.trim_end_matches(".json");
        let parent = full_path.parent()?;
        let mut related_files = Vec::new();
        let mut total_size = meta.len();

        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.starts_with(&format!("{}.", base_name)) && fname != item_name {
                    related_files.push(fname);
                    if let Ok(mm) = entry.metadata() {
                        total_size += mm.len();
                    }
                }
            }
        }

        let mut files = vec![item_name.to_string()];
        files.extend(related_files.clone());

        let enabled = !parked && settings_enabled(&m.id);
        return Some(ModInfo {
            id: Some(m.id.clone()),
            name: Some(m.name.clone()),
            author: m.author.clone(),
            version: m.version.clone(),
            description: m.description.clone(),
            dependencies: m.dependencies.clone(),
            dependency_versions: m.dependency_versions.clone(),
            affects_gameplay: m.affects_gameplay,
            has_dll: Some(m.has_dll.unwrap_or_else(|| detect_has(&related_files, ".dll"))),
            has_pck: Some(m.has_pck.unwrap_or_else(|| detect_has(&related_files, ".pck"))),
            enabled,
            instance_key: full_path.to_string_lossy().to_string(),
            folder_name: base_name.to_string(),
            is_folder: false,
            path: parent.to_string_lossy().to_string(),
            files,
            size: total_size,
            source: SOURCE_LOCAL.to_string(),
            workshop_id: None,
            workshop_url: None,
            mod_dir: None,
            min_game_version: m.min_game_version.clone(),
            location: if parked { LOC_DISABLED } else { LOC_ENABLED }.to_string(),
        });
    }
    None
}

// ── 创意工坊 Mod ──

/// 在工坊条目目录里递归找 manifest。
///
/// 工坊的目录层级不固定：
/// - 通常 `content/2868840/<工坊ID>/<Mod名>/<Mod名>.json`
/// - 也有直接放根上的 `content/2868840/<工坊ID>/<Mod名>.json`
/// - 一个工坊条目可以包含**多个** Mod（如 BaseLib 条目里还有 BaseLibCompatibilityHelper）
/// 所以必须递归找，而不能假设固定一层。
fn find_manifests(root: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 3 {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if name.starts_with('.') {
                continue;
            }
            find_manifests(&path, depth + 1, out);
        } else if name.to_lowercase().ends_with(".json") && !name.starts_with('.') {
            // 只有能被 parse_manifest 认出的（含 id+name）才算 manifest，
            // 顺带把配置文件类的 json 排除掉
            if parse_manifest(&path).is_some() {
                out.push(path);
            }
        }
    }
}

fn scan_workshop(
    workshop_root: &Path,
    settings: Option<&crate::modsettings::ModSettings>,
    out: &mut Vec<ModInfo>,
) {
    let Ok(items) = fs::read_dir(workshop_root) else {
        return;
    };
    for item in items.flatten() {
        let item_path = item.path();
        if !item_path.is_dir() {
            continue;
        }
        let workshop_id = item.file_name().to_string_lossy().to_string();
        if !workshop_id.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let mut manifests = Vec::new();
        find_manifests(&item_path, 0, &mut manifests);

        for manifest_path in manifests {
            let Some(m) = parse_manifest(&manifest_path) else {
                continue;
            };
            let Some(mod_root) = manifest_path.parent() else {
                continue;
            };

            let entries = entries_of(mod_root);
            let folder_name = mod_root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| workshop_id.clone());
            let mod_dir = mod_root
                .strip_prefix(&item_path)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
                .filter(|s| !s.is_empty());

            let enabled = settings
                .and_then(|s| s.enabled.get(&m.id).copied())
                .unwrap_or(true);

            out.push(ModInfo {
                id: Some(m.id.clone()),
                name: Some(m.name.clone()),
                author: m.author.clone(),
                version: m.version.clone(),
                description: m.description.clone(),
                dependencies: m.dependencies.clone(),
                dependency_versions: m.dependency_versions.clone(),
                affects_gameplay: m.affects_gameplay,
                has_dll: Some(m.has_dll.unwrap_or_else(|| detect_has(&entries, ".dll"))),
                has_pck: Some(m.has_pck.unwrap_or_else(|| detect_has(&entries, ".pck"))),
                enabled,
                // 同一目录可能承载多个 Mod，用 #id 保证唯一
                instance_key: format!("{}#{}", mod_root.to_string_lossy(), m.id),
                folder_name,
                is_folder: true,
                path: mod_root.to_string_lossy().to_string(),
                files: entries,
                size: dir_size(mod_root),
                source: SOURCE_WORKSHOP.to_string(),
                workshop_url: Some(crate::steam::workshop_url(&workshop_id)),
                workshop_id: Some(workshop_id.clone()),
                mod_dir,
                min_game_version: m.min_game_version.clone(),
                location: LOC_ENABLED.to_string(),
            });
        }
    }
}

pub fn scan_mods_internal(game_path: &str) -> Vec<ModInfo> {
    let mods_dir = get_mods_dir(game_path);
    let settings = crate::modsettings::read();
    let mut mods = Vec::new();

    migrate_legacy_disabled(game_path);

    // ── 本地：启用中 ──
    if mods_dir.exists() {
        if let Ok(entries) = fs::read_dir(&mods_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name == LEGACY_DISABLED_DIR {
                    continue;
                }
                if let Some(m) = build_local_mod(&entry.path(), &name, false, settings.as_ref()) {
                    mods.push(m);
                }
            }
        }
    }

    // ── 本地：已停放 ──
    let disabled_dir = get_disabled_dir(game_path);
    if disabled_dir.exists() {
        if let Ok(entries) = fs::read_dir(&disabled_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(m) = build_local_mod(&entry.path(), &name, true, settings.as_ref()) {
                    mods.push(m);
                }
            }
        }
    }

    // ── 创意工坊 ──
    for dir in crate::steam::workshop_content_dirs() {
        scan_workshop(&dir, settings.as_ref(), &mut mods);
    }

    // 排序：已启用优先，其次按来源（本地 → 创意工坊），再按名称
    mods.sort_by(|a, b| {
        if a.enabled != b.enabled {
            return if a.enabled {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        if a.source != b.source {
            return if a.source == SOURCE_LOCAL {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        let a_name = a.name.as_deref().unwrap_or("");
        let b_name = b.name.as_deref().unwrap_or("");
        a_name.to_lowercase().cmp(&b_name.to_lowercase())
    });

    mods
}

fn find_folder_mod_location(game_path: &str, folder_name: &str) -> Option<PathBuf> {
    let roots = vec![
        get_mods_dir(game_path),
        get_disabled_dir(game_path),
        get_legacy_disabled_dir(game_path),
    ];
    for root in roots {
        let candidate = root.join(folder_name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn find_flat_mod_base_dir(game_path: &str, files: &[String]) -> Option<PathBuf> {
    let roots = vec![
        get_mods_dir(game_path),
        get_disabled_dir(game_path),
        get_legacy_disabled_dir(game_path),
    ];
    for root in roots {
        if files.iter().any(|f| root.join(f).exists()) {
            return Some(root);
        }
    }
    None
}

#[tauri::command]
pub fn mods_scan(state: tauri::State<'_, AppState>) -> Vec<ModInfo> {
    let gp = state.game_path.lock().unwrap();
    match &*gp {
        Some(p) => scan_mods_internal(p),
        None => vec![],
    }
}

/// 把本地 Mod 在 `mods/` 与 `mods_disabled/` 之间搬运。
fn park_local_mod(game_path: &str, info: &ToggleModInfo, to_disabled: bool) -> Result<(), String> {
    let mods_dir = get_mods_dir(game_path);
    let disabled_dir = get_disabled_dir(game_path);
    let dst_root = if to_disabled { disabled_dir } else { mods_dir };

    if info.is_folder {
        let src = find_folder_mod_location(game_path, &info.folder_name)
            .ok_or_else(|| format!("找不到 MOD 文件夹: {}", info.folder_name))?;
        let dst = dst_root.join(&info.folder_name);
        if src != dst {
            fs::rename(&src, &dst).map_err(|e| format!("移动失败: {}", e))?;
        }
    } else {
        let files = info.files.clone().unwrap_or_default();
        let src_dir = find_flat_mod_base_dir(game_path, &files)
            .ok_or_else(|| "找不到 MOD 文件".to_string())?;
        for file in &files {
            let src = src_dir.join(file);
            let dst = dst_root.join(file);
            if src.exists() {
                fs::rename(&src, &dst).map_err(|e| format!("移动失败: {}", e))?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn mods_toggle(state: tauri::State<'_, AppState>, mod_info: ToggleModInfo) -> ModResult {
    let gp = state.game_path.lock().unwrap();
    let game_path = match &*gp {
        Some(p) => p.clone(),
        None => {
            return ModResult {
                success: false,
                error: Some("Game path not set".into()),
                mods: None,
                installed: None,
            }
        }
    };
    drop(gp);

    let enable = !mod_info.enabled;
    let source = mod_info
        .source
        .clone()
        .unwrap_or_else(|| SOURCE_LOCAL.to_string());
    let location = mod_info
        .location
        .clone()
        .unwrap_or_else(|| LOC_ENABLED.to_string());
    let id = mod_info.id.clone().unwrap_or_default();

    let err = |msg: String| ModResult {
        success: false,
        error: Some(msg),
        mods: None,
        installed: None,
    };

    let ok = |game_path: &str| ModResult {
        success: true,
        error: None,
        mods: Some(scan_mods_internal(game_path)),
        installed: None,
    };

    // ── 创意工坊：只能改游戏设置，绝不搬 Steam 的文件 ──
    if source == SOURCE_WORKSHOP {
        if id.is_empty() {
            return err("创意工坊 MOD 缺少 id，无法切换".into());
        }
        return match crate::modsettings::set_enabled(&id, enable) {
            Ok(_) => ok(&game_path),
            Err(e) => err(e),
        };
    }

    // ── 本地：停放中的先搬回来，其余优先走游戏设置，无法登记时退回搬目录 ──
    if location == LOC_DISABLED {
        if let Err(e) = park_local_mod(&game_path, &mod_info, false) {
            return err(e);
        }
        // 搬回来后顺带把设置里的状态也打开（没登记就算了，默认就是启用）
        if !id.is_empty() {
            let _ = crate::modsettings::set_enabled(&id, true);
        }
        return ok(&game_path);
    }

    if enable {
        // 文件还在 mods/，说明当初是通过游戏设置关掉的
        if !id.is_empty() && crate::modsettings::is_registered(&id) {
            if let Err(e) = crate::modsettings::set_enabled(&id, true) {
                return err(e);
            }
            return ok(&game_path);
        }
        return ok(&game_path);
    }

    // 禁用本地 Mod：能写设置就写设置，否则退回停放
    if !id.is_empty() && crate::modsettings::is_registered(&id) {
        if let Err(e) = crate::modsettings::set_enabled(&id, false) {
            return err(e);
        }
        return ok(&game_path);
    }

    match park_local_mod(&game_path, &mod_info, true) {
        Ok(_) => ok(&game_path),
        Err(e) => err(e),
    }
}

#[tauri::command]
pub fn mods_uninstall(state: tauri::State<'_, AppState>, mod_info: ToggleModInfo) -> ModResult {
    let gp = state.game_path.lock().unwrap();
    let game_path = match &*gp {
        Some(p) => p.clone(),
        None => {
            return ModResult {
                success: false,
                error: Some("Game path not set".into()),
                mods: None,
                installed: None,
            }
        }
    };
    drop(gp);

    // 创意工坊内容由 Steam 托管，删除会被 Steam 校验后重新下载，
    // 这里明确拒绝，并提示去取消订阅。
    if mod_info.source.as_deref() == Some(SOURCE_WORKSHOP) {
        return ModResult {
            success: false,
            error: Some(
                "创意工坊 MOD 由 Steam 管理，无法在这里卸载。\n请在 Steam 中取消订阅，或点击「创意工坊」按钮打开页面。"
                    .into(),
            ),
            mods: None,
            installed: None,
        };
    }

    if mod_info.is_folder {
        if let Some(mod_path) = find_folder_mod_location(&game_path, &mod_info.folder_name) {
            if let Err(e) = fs::remove_dir_all(&mod_path) {
                return ModResult {
                    success: false,
                    error: Some(format!("删除失败: {}", e)),
                    mods: None,
                    installed: None,
                };
            }
        }
    } else {
        let files = mod_info.files.unwrap_or_default();
        if let Some(base_dir) = find_flat_mod_base_dir(&game_path, &files) {
            for file in &files {
                let fp = base_dir.join(file);
                if fp.exists() {
                    let _ = fs::remove_file(&fp);
                }
            }
        }
    }

    ModResult {
        success: true,
        error: None,
        mods: Some(scan_mods_internal(&game_path)),
        installed: None,
    }
}

fn smart_extract_zip(zip_path: &str, mods_dir: &Path) -> Result<(), String> {
    let ext = Path::new(zip_path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if ext != "zip" {
        return Err(format!(
            "不支持的格式: .{}\n\n目前仅支持 .zip 格式的压缩包。\n如果是 .rar / .7z 请先解压后拖入文件夹，或转换为 .zip 格式。",
            ext
        ));
    }

    let file = fs::File::open(zip_path).map_err(|e| format!(
        "无法读取压缩包: {}\n\n该文件可能已损坏或不是有效的 ZIP 格式。\n\nMOD 压缩包应为 .zip 格式，内含:\n  • ModName.json (MOD 描述文件)\n  • ModName.dll (代码类 MOD)\n  • ModName.pck (资源类 MOD)",
        e
    ))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!(
        "无法读取压缩包: {}\n\n该文件可能已损坏或不是有效的 ZIP 格式。",
        e
    ))?;

    if archive.len() == 0 {
        return Err(format!(
            "压缩包为空: {}",
            Path::new(zip_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        ));
    }

    // ── Smart search: find MOD manifest JSON files inside the ZIP ──
    let mut mod_roots: Vec<(String, String)> = Vec::new(); // (mod_dir, folder_name)
    let mut flat_mod = false;
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        if !name.ends_with(".json") {
            continue;
        }
        let mut buf = Vec::new();
        let mut reader = entry;
        std::io::Read::read_to_end(&mut reader, &mut buf).map_err(|e| e.to_string())?;
        if let Ok(text) = String::from_utf8(buf) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                if val.get("id").is_some() && val.get("name").is_some() {
                    let parts: Vec<&str> = name.split('/').collect();
                    if parts.len() >= 2 {
                        let mod_dir = parts[..parts.len() - 1].join("/");
                        let folder_name = parts[parts.len() - 2].to_string();
                        mod_roots.push((mod_dir, folder_name));
                    } else {
                        flat_mod = true;
                    }
                }
            }
        }
    }

    if !mod_roots.is_empty() || flat_mod {
        // Re-open for extraction
        let file2 = fs::File::open(zip_path).map_err(|e| e.to_string())?;
        let mut archive2 = zip::ZipArchive::new(file2).map_err(|e| e.to_string())?;

        for (mod_dir, folder_name) in &mod_roots {
            // folder_name 也来自压缩包，同样要过一遍校验
            if folder_name.is_empty() || folder_name == "." || folder_name == ".." {
                continue;
            }
            let dest_dir = match safe_join(mods_dir, folder_name) {
                Some(p) => p,
                None => continue,
            };
            let _ = fs::create_dir_all(&dest_dir);
            let prefix = format!("{}/", mod_dir);
            for i in 0..archive2.len() {
                let mut entry = archive2.by_index(i).map_err(|e| e.to_string())?;
                let ep = entry.name().replace('\\', "/");
                if !ep.starts_with(&prefix) {
                    continue;
                }
                let rel = &ep[prefix.len()..];
                if rel.is_empty() {
                    continue;
                }
                // ★ Zip Slip 防护：拒绝任何试图跳出目标目录的路径
                let Some(out_path) = safe_join(&dest_dir, rel) else {
                    continue;
                };
                if entry.is_dir() {
                    let _ = fs::create_dir_all(&out_path);
                } else {
                    if let Some(parent) = out_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let mut outfile = fs::File::create(&out_path).map_err(|e| e.to_string())?;
                    std::io::copy(&mut entry, &mut outfile).map_err(|e| e.to_string())?;
                }
            }
        }

        if flat_mod && mod_roots.is_empty() {
            for i in 0..archive2.len() {
                let mut entry = archive2.by_index(i).map_err(|e| e.to_string())?;
                let ep = entry.name().replace('\\', "/");
                if ep.contains('/') || entry.is_dir() {
                    continue;
                }
                let Some(out_path) = safe_join(mods_dir, &ep) else {
                    continue;
                };
                let mut outfile = fs::File::create(&out_path).map_err(|e| e.to_string())?;
                std::io::copy(&mut entry, &mut outfile).map_err(|e| e.to_string())?;
            }
        }
        return Ok(());
    }

    // ── Fallback: no manifest found, use legacy extraction ──
    let mut top_dirs = std::collections::HashSet::new();
    let mut has_root_file = false;
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.mangled_name();
        let parts: Vec<_> = name.components().collect();
        if parts.len() == 1 && !entry.is_dir() {
            has_root_file = true;
            break;
        }
        if let Some(first) = parts.first() {
            top_dirs.insert(first.as_os_str().to_string_lossy().to_string());
        }
    }

    let dest = if !has_root_file && top_dirs.len() == 1 {
        mods_dir.to_path_buf()
    } else {
        let base_name = Path::new(zip_path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown_mod".to_string());
        let sub_dir = mods_dir.join(&base_name);
        let _ = fs::create_dir_all(&sub_dir);
        sub_dir
    };

    let file2 = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive2 = zip::ZipArchive::new(file2).map_err(|e| e.to_string())?;
    for i in 0..archive2.len() {
        let mut entry = archive2.by_index(i).map_err(|e| e.to_string())?;
        let out_path = dest.join(entry.mangled_name());
        if entry.is_dir() {
            let _ = fs::create_dir_all(&out_path);
        } else {
            if let Some(parent) = out_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let mut outfile = fs::File::create(&out_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut outfile).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn install_folder(folder_path: &str, mods_dir: &Path) -> Result<(), String> {
    let src = Path::new(folder_path);
    if !src.is_dir() {
        return Err(format!("不是有效的文件夹: {}", folder_path));
    }
    let folder_name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown_mod".to_string());
    let dest = mods_dir.join(&folder_name);
    copy_dir_recursive(src, &dest)
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    let _ = fs::create_dir_all(dest);
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn mods_install(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<ModResult, String> {
    let gp = state.game_path.lock().unwrap().clone();
    let game_path = match gp {
        Some(p) => p,
        None => {
            return Ok(ModResult {
                success: false,
                error: Some("Game path not set".into()),
                mods: None,
                installed: None,
            })
        }
    };

    let mods_dir = get_mods_dir(&game_path);
    let dialog = app.dialog();
    let files = dialog
        .file()
        .set_title("Select MOD Archive")
        .add_filter("Archives", &["zip"])
        .blocking_pick_files();

    let file_paths = match files {
        Some(paths) => paths,
        None => {
            return Ok(ModResult {
                success: false,
                error: Some("Cancelled".into()),
                mods: None,
                installed: None,
            })
        }
    };

    let mut installed = Vec::new();
    for fp in &file_paths {
        let path_str = fp.to_string();
        let p = Path::new(&path_str);
        let result = if p.is_dir() {
            install_folder(&path_str, &mods_dir)
        } else {
            smart_extract_zip(&path_str, &mods_dir)
        };
        if let Err(e) = result {
            return Ok(ModResult {
                success: false,
                error: Some(e),
                mods: None,
                installed: None,
            });
        }
        installed.push(
            p.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
        );
    }

    Ok(ModResult {
        success: true,
        error: None,
        mods: Some(scan_mods_internal(&game_path)),
        installed: Some(installed),
    })
}

#[tauri::command]
pub fn mods_install_drop(
    state: tauri::State<'_, AppState>,
    file_paths: Vec<String>,
) -> ModResult {
    let gp = state.game_path.lock().unwrap().clone();
    let game_path = match gp {
        Some(p) => p,
        None => {
            return ModResult {
                success: false,
                error: Some("Game path not set".into()),
                mods: None,
                installed: None,
            }
        }
    };

    let mods_dir = get_mods_dir(&game_path);
    let mut installed = Vec::new();

    for fp in &file_paths {
        let p = Path::new(fp.as_str());
        let result = if p.is_dir() {
            install_folder(fp, &mods_dir)
        } else {
            smart_extract_zip(fp, &mods_dir)
        };
        if let Err(e) = result {
            return ModResult {
                success: false,
                error: Some(e),
                mods: None,
                installed: None,
            };
        }
        installed.push(
            p.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
        );
    }

    ModResult {
        success: true,
        error: None,
        mods: Some(scan_mods_internal(&game_path)),
        installed: Some(installed),
    }
}

fn create_zip_from_dir(source: &Path, zip_path: &str) -> Result<(), String> {
    let file = fs::File::create(zip_path).map_err(|e| e.to_string())?;
    let mut zip_writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

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
                    .to_string();
                if path.is_dir() {
                    let _ = zip_writer.add_directory(&format!("{}/", rel), options);
                    add_dir_to_zip(zip_writer, base, &path, options)?;
                } else {
                    zip_writer
                        .start_file(&rel, options)
                        .map_err(|e| e.to_string())?;
                    let mut f = fs::File::open(&path).map_err(|e| e.to_string())?;
                    let mut buf = Vec::new();
                    f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                    std::io::Write::write_all(zip_writer, &buf).map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }

    add_dir_to_zip(&mut zip_writer, source, source, options)?;
    zip_writer.finish().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn mods_backup(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<ModResult, String> {
    let gp = state.game_path.lock().unwrap().clone();
    let game_path = match gp {
        Some(p) => p,
        None => {
            return Ok(ModResult {
                success: false,
                error: Some("Game path not set".into()),
                mods: None,
                installed: None,
            })
        }
    };

    let mods_dir = get_mods_dir(&game_path);
    let dialog = app.dialog();
    let default_name = format!("sts2_mods_backup_{}.zip", unix_millis());

    let save_path = dialog
        .file()
        .set_title("Save MOD Backup")
        .set_file_name(&default_name)
        .add_filter("ZIP Archive", &["zip"])
        .blocking_save_file();

    match save_path {
        Some(path) => {
            let path_str = path.to_string();
            if let Err(e) = create_zip_from_dir(&mods_dir, &path_str) {
                return Ok(ModResult {
                    success: false,
                    error: Some(e),
                    mods: None,
                    installed: None,
                });
            }
            Ok(ModResult {
                success: true,
                error: None,
                mods: None,
                installed: None,
            })
        }
        None => Ok(ModResult {
            success: false,
            error: None,
            mods: None,
            installed: None,
        }),
    }
}

#[tauri::command]
pub async fn mods_restore(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<ModResult, String> {
    let gp = state.game_path.lock().unwrap().clone();
    let game_path = match gp {
        Some(p) => p,
        None => {
            return Ok(ModResult {
                success: false,
                error: Some("Game path not set".into()),
                mods: None,
                installed: None,
            })
        }
    };

    let mods_dir = get_mods_dir(&game_path);
    let dialog = app.dialog();
    let file = dialog
        .file()
        .set_title("Select MOD Backup")
        .add_filter("ZIP Archive", &["zip"])
        .blocking_pick_file();

    match file {
        Some(path) => {
            let path_str = path.to_string();
            if let Err(e) = smart_extract_zip(&path_str, &mods_dir) {
                return Ok(ModResult {
                    success: false,
                    error: Some(e),
                    mods: None,
                    installed: None,
                });
            }
            Ok(ModResult {
                success: true,
                error: None,
                mods: Some(scan_mods_internal(&game_path)),
                installed: None,
            })
        }
        None => Ok(ModResult {
            success: false,
            error: None,
            mods: None,
            installed: None,
        }),
    }
}

fn unix_millis() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    now.to_string()
}
