/**
 * 生成 README 用的界面截图。
 *
 * 为什么需要 mock：前端通过 window.__TAURI__ 走 Tauri IPC，
 * 直接用浏览器打开时不存在，tauri-api.js 会立刻抛错、React 挂不上。
 * 这里在页面脚本执行前注入假实现，喂一份贴近真实使用的数据。
 *
 * 采集用 2x 缩放，再降采样到 1600px 宽，保证文字清晰同时控制文件体积。
 *
 * 用法：
 *   1) 另开终端跑 `npm run dev`（:3000）
 *   2) node scripts/capture-screenshots.mjs
 */
import { chromium } from 'playwright';

const URL = process.env.CAPTURE_URL || 'http://localhost:3000';
const OUT_DIR = process.env.CAPTURE_OUT || 'docs';
const VIEWPORT = { width: 1440, height: 900 };

const initScript = () => {
  const iso = (daysAgo) => new Date(Date.now() - daysAgo * 86400000).toISOString();

  const ws = (id, name, folder, extra = {}) => ({
    id,
    name,
    author: extra.author || 'Alchyr',
    version: extra.version || 'v1.0.0',
    description: extra.description || '',
    dependencies: extra.dependencies || [],
    dependencyVersions: (extra.dependencies || []).map((d) => ({ id: d, minVersion: '3.0.0' })),
    affects_gameplay: extra.affects_gameplay || false,
    has_dll: true,
    has_pck: true,
    enabled: extra.enabled !== false,
    instanceKey: `ws:${extra.workshopId || '1'}:${id}`,
    folderName: folder,
    isFolder: true,
    path: `E:\\SteamLibrary\\steamapps\\workshop\\content\\2868840\\${extra.workshopId || '1'}\\${folder}`,
    files: [`${folder}.json`, `${folder}.dll`, `${folder}.pck`],
    size: extra.size || 1024 * 1024 * 2,
    source: 'workshop',
    workshopId: extra.workshopId || '1',
    workshopUrl: `https://steamcommunity.com/sharedfiles/filedetails/?id=${extra.workshopId || '1'}`,
    modDir: folder,
    minGameVersion: extra.minGameVersion || '0.113.0',
    location: 'enabled',
  });

  const local = (id, name, extra = {}) => ({
    id,
    name,
    author: extra.author || 'Nexus',
    version: extra.version || 'v1.0.0',
    description: extra.description || '',
    dependencies: extra.dependencies || [],
    dependencyVersions: [],
    affects_gameplay: extra.affects_gameplay !== false,
    has_dll: true,
    has_pck: false,
    enabled: extra.enabled !== false,
    instanceKey: `local:${name}`,
    folderName: name,
    isFolder: true,
    path: `E:\\SteamLibrary\\steamapps\\common\\Slay the Spire 2\\mods\\${name}`,
    files: [`${name}.json`, `${name}.dll`],
    size: extra.size || 1024 * 512,
    source: 'local',
    workshopId: null,
    workshopUrl: null,
    modDir: null,
    minGameVersion: null,
    location: extra.enabled === false ? 'disabled' : 'enabled',
  });

  const mods = [
    ws('BaseLib', 'BaseLib', 'BaseLib', {
      author: 'Alchyr',
      version: 'v3.4.8',
      description: 'Modding utility for Slay the Spire 2. Every other mod depends on it.',
      dependencies: ['BaseLibCompatibilityHelper'],
      workshopId: '3737335127',
      size: 3200 * 1024,
      minGameVersion: '0.107.1',
    }),
    ws('BaseLibCompatibilityHelper', 'BaseLibCompatibilityHelper', 'BaseLib', {
      author: 'Alchyr',
      version: 'v3.4.8',
      description: 'Compatibility shim bundled with BaseLib.',
      workshopId: '3737335127',
      size: 96 * 1024,
    }),
    ws('QuickRestart', 'Quick Restart', 'QuickRestart', {
      author: 'erasels',
      version: 'v2.0.0',
      description: 'Provides a button that allows for quickly save&loading a room, undoing any actions taken since entering.',
      dependencies: ['BaseLib'],
      workshopId: '3737322022',
      size: 210 * 1024,
    }),
    ws('RegentFX', '万象辉星[RegentFX]', 'RegentFX', {
      author: 'Vitech, SSUika, Margaritana',
      version: '0.5.2',
      description: '为储君的部分卡牌按照卡面还原了特效。',
      workshopId: '3747497501',
      size: 12800 * 1024,
      minGameVersion: '0.103.0',
    }),
    ws('Mesugaki', 'Mesugaki_Regent', 'Mesugaki', {
      author: 'Seic_Oh, Dodobird',
      version: '0.2.0',
      description: 'Zako Zako ~♡  Regent skin & voice pack.',
      workshopId: '3748603697',
      size: 9800 * 1024,
    }),
    local('SpeedX', 'SpeedX', {
      author: 'piyixia',
      version: 'v3.3.2',
      description: '战斗加速、跳过横幅、自动结束回合等一大堆可调项。',
      affects_gameplay: true,
      size: 380 * 1024,
    }),
    local('FairLAN', 'FairLAN', {
      author: 'Inlve',
      version: 'v1.4.0',
      description: '本地联机辅助，修正联机时的状态不同步。',
      enabled: false,
      size: 260 * 1024,
    }),
    local('CrossPlatformMod', 'CrossPlatformMod', {
      author: 'community',
      version: 'v0.9.1',
      description: '跨平台联机支持。',
      enabled: false,
      size: 410 * 1024,
    }),
  ];

  const characterStats = [
    { id: 'CHARACTER.IRONCLAD', name: '铁甲战士', wins: 34, losses: 19, maxAscension: 12, playtime: 128000, bestStreak: 6 },
    { id: 'CHARACTER.SILENT', name: '沉默猎手', wins: 27, losses: 22, maxAscension: 8, playtime: 96000, bestStreak: 4 },
    { id: 'CHARACTER.REGENT', name: '摄政王', wins: 11, losses: 14, maxAscension: 3, playtime: 31000, bestStreak: 2 },
    { id: 'CHARACTER.NECROBINDER', name: '缚灵师', wins: 6, losses: 9, maxAscension: 1, playtime: 14500, bestStreak: 2 },
  ];

  const summary = {
    totalPlaytime: 269500,
    floorsClimbed: 8120,
    currentScore: 14862,
    totalUnlocks: 173,
    discoveredCards: 412,
    discoveredRelics: 189,
    epochs: 24,
    characters: characterStats,
    uniqueId: '76561198806528494',
  };

  const saveSlot = (slot, modded, hasData, extra = {}) => ({
    slot,
    modded,
    path: `C:\\Users\\Inlve\\AppData\\Roaming\\SlayTheSpire2\\steam\\76561198806528494\\${modded ? 'modded\\' : ''}${slot}`,
    hasProgress: hasData,
    hasPrefs: hasData,
    empty: !hasData,
    lastModified: hasData ? iso(extra.daysAgo ?? 1) : null,
    size: hasData ? (extra.size || 2.4 * 1024 * 1024) : 0,
    summary: hasData ? { ...summary, ...(extra.summary || {}) } : null,
  });

  const saves = {
    slots: [
      saveSlot('profile1', false, true, { daysAgo: 2 }),
      saveSlot('profile2', false, true, { daysAgo: 11, summary: { currentScore: 6210, floorsClimbed: 2340, totalPlaytime: 61000 } }),
      saveSlot('profile3', false, false),
      saveSlot('profile1', true, true, { daysAgo: 0.2, size: 3.1 * 1024 * 1024 }),
      saveSlot('profile2', true, false),
      saveSlot('profile3', true, false),
    ],
    backups: [
      { name: 'convert_profile1_modded_before_2026-11-20T01-58-00.zip', path: 'C:\\Users\\Inlve\\AppData\\Roaming\\STS2ModManager\\save_backups\\a.zip', size: 3.1 * 1024 * 1024, time: iso(0.1) },
      { name: 'auto_backup_profile1_2026-11-18T22-41-10.zip', path: 'C:\\Users\\Inlve\\AppData\\Roaming\\STS2ModManager\\save_backups\\b.zip', size: 2.4 * 1024 * 1024, time: iso(2) },
      { name: 'auto_backup_profile2_2026-11-09T19-12-44.zip', path: 'C:\\Users\\Inlve\\AppData\\Roaming\\STS2ModManager\\save_backups\\c.zip', size: 1.7 * 1024 * 1024, time: iso(11) },
    ],
  };

  const logLines = [
    'MegaDot v4.5.1.m.14.mono.custom_build - https://godotengine.org',
    'Release Version: v0.113.1',
    '[INFO] Loading mod manifest: BaseLib',
    "[INFO] Finished mod initialization for 'BaseLib' (id: BaseLib)",
    "[INFO] Finished mod initialization for 'Quick Restart' (id: QuickRestart)",
    "[INFO] Finished mod initialization for 'RegentFX' (id: RegentFX)",
    '[INFO] 12 mods loaded, 11 enabled',
    '[INFO] Starting new run: IRONCLAD ascension 12 seed 8PRXK2Q1',
    '[INFO] Entered act 1 - The City',
    '[INFO] Player 1 playing card STRIKE_RED (targeting Monster JAW_WORM)',
    '[INFO] Player 1 playing card BASH (targeting Monster JAW_WORM)',
    '[INFO] Monster JAW_WORM performing move CHOMP',
    '[INFO] Player 1 playing card DEFEND_RED (no target)',
    '[INFO] Encounter CULTIST won',
    '[INFO] Player 1 playing card INFLAME (no target)',
    '[WARN] Relic pool exhausted for RARE, falling back to UNCOMMON',
    '[INFO] Player 1 playing card DEMON_FORM (no target)',
    '[INFO] Player 1 playing card HEAVY_BLADE (targeting Monster LAGAVULIN)',
    '[INFO] Monster LAGAVULIN performing move SLEEP',
    '[ERROR] RelicBaseLibCustom: failed to resolve icon path relics/my_relic.png',
    '[INFO] Player 1 playing card ANGEL_FORM (no target)',
    '[WARN] Screen shake disabled in settings',
    '[INFO] Player 1 playing card BLUDGEON (targeting Monster LAGAVULIN)',
    '[INFO] Encounter LAGAVULIN won',
    '[INFO] Gained 412 gold',
    '[INFO] Player 1 playing card OFFERING (no target)',
    '[INFO] MuteInBackground: FocusOut received [tween=False, windowFocused=False]',
    '[INFO] Limiting background FPS to 30',
    '[INFO] MuteInBackground: FocusIn received [tween=True, msSinceFocusOut=1269, windowFocused=True]',
    '[INFO] Restored foreground FPS',
    '[INFO] Player 1 playing card MANIFEST_AUTHORITY (no target)',
    '[INFO] Player 1 playing card CLOAK_OF_STARS (no target)',
    '[INFO] Player 1 playing card LUNAR_BLAST (targeting 虔誠雕刻師 (index 2))',
    '[INFO] Monster DEVOTED_SCULPTOR performing move SAVAGE_MOVE',
    '[INFO] Player 1 playing card APOTHEOSIS (no target)',
    '[INFO] Encounter DEVOTED_SCULPTOR won',
    '[INFO] Entered act 2 - The Beyond',
    '[INFO] Player 1 playing card OFFERING (no target)',
    '[WARN] Card preview renderer fell back to default shader',
    '[INFO] Player 1 playing card WHIRLWIND (targeting Monster REPTOMANCER)',
    '[INFO] Monster REPTOMANCER performing move DAGGER_SPRAY',
    '[INFO] Player 1 playing card FEED (targeting Monster REPTOMANCER)',
    '[INFO] Encounter REPTOMANCER won',
    '[INFO] Player 1 playing card BARRICADE (no target)',
    '[INFO] Player 1 playing card ENTrench (no target)',
    '[INFO] Player 1 playing card BODY_SLAM (targeting Monster CHAMP)',
    '[INFO] Monster CHAMP performing move EXECUTE',
    '[ERROR] Failed to load texture: res://images/powers/strength_custom.png',
    '[INFO] Player 1 playing card LIMIT_BREAK (no target)',
    '[INFO] Encounter CHAMP won',
    '[INFO] Gained 388 gold',
    '[INFO] MuteInBackground: FocusOut received [tween=False, windowFocused=False]',
    '[INFO] Limiting background FPS to 30',
    '[INFO] MuteInBackground: FocusIn received [tween=True, msSinceFocusOut=846, windowFocused=True]',
    '[INFO] Restored foreground FPS',
    '[INFO] Entered act 3 - The Beyond',
  ];

  const handlers = {
    app_init: () => ({
      gamePath: 'E:\\SteamLibrary\\steamapps\\common\\Slay the Spire 2',
      modsDir: 'E:\\SteamLibrary\\steamapps\\common\\Slay the Spire 2\\mods',
      gameVersion: 'v0.113.1',
      workshopDirs: 1,
    }),
    app_select_game_path: () => null,
    game_get_state: () => 'idle',
    game_get_version: () => ({ version: 'v0.113.1', engine: 'MegaDot 4.5' }),
    game_launch: () => ({ success: true, method: 'steam' }),
    game_analyze_crash: () => ({ issues: [], logFile: null, errorCount: 0, warnCount: 0, involvedMods: [], loadedMods: [], notices: [] }),
    mods_scan: () => mods,
    mods_toggle: () => ({ success: true, mods }),
    mods_uninstall: () => ({ success: true, mods }),
    mods_install: () => ({ success: true, mods }),
    mods_install_drop: () => ({ success: true, mods }),
    mods_backup: () => ({ success: true }),
    mods_restore: () => ({ success: true, mods }),
    profiles_load: () => ({}),
    profiles_save: () => ({ success: true }),
    translations_load: () => ({}),
    translations_save: () => ({ success: true }),
    translate_text: () => ({ success: true, translated: '' }),
    logs_get_latest: () => ({ files: ['godot.log', 'godot2_2026-11-18.log'], content: logLines.join('\n') }),
    logs_read: () => logLines.join('\n'),
    saves_scan: () => saves,
    saves_export: () => ({ success: true }),
    saves_import: () => ({ success: true }),
    saves_delete_backup: () => ({ success: true }),
    saves_convert: () => ({ success: true, slot: 'profile1', direction: 'to_modded', copiedFiles: 42, copiedBytes: 3145728, backupPath: 'backups/convert.zip' }),
    shell_open_mods_dir: () => null,
    shell_open_game_dir: () => null,
    shell_open_logs_dir: () => null,
    shell_open_saves_dir: () => null,
    shell_open_url: () => null,
    window_minimize: () => null,
    window_maximize: () => null,
    window_close: () => null,
  };

  window.__TAURI__ = {
    core: { invoke: async (cmd) => (handlers[cmd] ? handlers[cmd]() : null) },
    window: { getCurrentWindow: () => ({}) },
  };
};

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: VIEWPORT, deviceScaleFactor: 2 });
const page = await context.newPage();
await page.addInitScript(initScript);

const problems = [];
page.on('pageerror', (e) => problems.push('pageerror: ' + e));
page.on('console', (m) => { if (m.type() === 'error') problems.push('console: ' + m.text()); });

await page.goto(URL, { waitUntil: 'domcontentloaded' });
await page.waitForSelector('text=MOD 管理', { timeout: 20000 });
await page.waitForTimeout(1200);

const sidebar = (label) => page.locator('button', { hasText: label }).first();

// 1) MOD 管理：打开一个创意工坊 MOD 的详情面板，展示来源 / 依赖 / 工坊入口
await page.locator('h3', { hasText: 'BaseLib' }).first().click();
await page.waitForTimeout(900);
await page.screenshot({ path: `${OUT_DIR}/preview-mods.png` });
console.log('saved preview-mods.png');

// 2) 存档管理：用更高的视口，让「普通存档 / MOD 存档 / 存档转换」同屏
await sidebar('存档管理').click();
await page.waitForSelector('text=普通存档', { timeout: 15000 });
await page.setViewportSize({ width: VIEWPORT.width, height: 1320 });
await page.waitForTimeout(900);
await page.screenshot({ path: `${OUT_DIR}/preview-saves.png` });
console.log('saved preview-saves.png');

// 3) 游戏日志
await page.setViewportSize(VIEWPORT);
await sidebar('游戏日志').click();
await page.waitForSelector('[data-index]', { timeout: 15000 });
await page.waitForTimeout(900);
await page.screenshot({ path: `${OUT_DIR}/preview-logs.png` });
console.log('saved preview-logs.png');

await browser.close();

if (problems.length) {
  console.log('\n页面错误：');
  problems.slice(0, 5).forEach((p) => console.log('  ' + p));
  process.exit(1);
}
console.log('\n截图完成（2x）');
