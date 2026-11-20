// 前端唯一的原生能力入口：把 Rust 侧的 Tauri 命令包装成 window.api。
// 所有 UI 组件只认这一个对象，不直接触碰 window.__TAURI__。

const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;

const appWindow = getCurrentWindow();

// Game state polling
let gameStateCallback = null;
let gameExitedCallback = null;
let pollingInterval = null;
let lastGameState = 'idle';

function startGameStatePolling() {
  if (pollingInterval) return;
  pollingInterval = setInterval(async () => {
    try {
      const state = await invoke('game_get_state');
      if (state !== lastGameState) {
        const prevState = lastGameState;
        lastGameState = state;
        if (gameStateCallback) gameStateCallback(state);
        // Detect game exit
        if (prevState === 'running' && state === 'idle') {
          if (gameExitedCallback) gameExitedCallback({ quick: false });
        }
        if (prevState === 'launching' && state === 'idle') {
          if (gameExitedCallback) gameExitedCallback({ quick: true });
        }
      }
      // Stop polling if idle
      if (state === 'idle' && pollingInterval) {
        clearInterval(pollingInterval);
        pollingInterval = null;
      }
    } catch (e) {
      // ignore polling errors
    }
  }, 2000);
}

window.api = {
  // App
  init: () => invoke('app_init'),
  selectGamePath: () => invoke('app_select_game_path'),

  // Window
  minimize: () => invoke('window_minimize'),
  maximize: () => invoke('window_maximize'),
  close: () => invoke('window_close'),

  // Mods
  scanMods: () => invoke('mods_scan'),
  // 启停需要带上 mod 的 id / 来源 / 位置：
  // 创意工坊 Mod 只能改游戏设置，本地 Mod 可能还需要搬目录
  toggleMod: (modInfo) => invoke('mods_toggle', {
    modInfo: {
      isFolder: modInfo.isFolder,
      folderName: modInfo.folderName,
      files: modInfo.files,
      enabled: modInfo.enabled,
      id: modInfo.id ?? null,
      source: modInfo.source ?? null,
      location: modInfo.location ?? null,
    },
  }),
  uninstallMod: (modInfo) => invoke('mods_uninstall', {
    modInfo: {
      isFolder: modInfo.isFolder,
      folderName: modInfo.folderName,
      files: modInfo.files,
      enabled: modInfo.enabled,
      id: modInfo.id ?? null,
      source: modInfo.source ?? null,
      location: modInfo.location ?? null,
    },
  }),
  installMod: () => invoke('mods_install'),
  installDrop: (filePaths) => invoke('mods_install_drop', { filePaths }),
  backupMods: () => invoke('mods_backup'),
  restoreMods: () => invoke('mods_restore'),

  // Shell
  openModsDir: () => invoke('shell_open_mods_dir'),
  openGameDir: () => invoke('shell_open_game_dir'),
  openLogsDir: () => invoke('shell_open_logs_dir'),
  openSavesDir: () => invoke('shell_open_saves_dir'),
  openUrl: (url) => invoke('shell_open_url', { url }),

  // Game
  launchGame: async () => {
    const result = await invoke('game_launch');
    if (result.success) {
      lastGameState = 'launching';
      startGameStatePolling();
    }
    return result;
  },
  getGameState: () => invoke('game_get_state'),
  getGameVersion: () => invoke('game_get_version'),
  analyzeCrash: () => invoke('game_analyze_crash'),
  onGameStateChanged: (cb) => { gameStateCallback = cb; },
  onGameExited: (cb) => { gameExitedCallback = cb; },

  // Logs
  getLatestLogs: () => invoke('logs_get_latest'),
  readLog: (fileName) => invoke('logs_read', { fileName }),

  // Profiles
  loadProfiles: () => invoke('profiles_load'),
  saveProfiles: (profiles) => invoke('profiles_save', { profiles }),

  // Translate
  translateText: (text) => invoke('translate_text', { text }),
  loadTranslations: () => invoke('translations_load'),
  saveTranslations: (data) => invoke('translations_save', { data }),

  // Saves
  scanSaves: () => invoke('saves_scan'),
  exportSave: (opts) => invoke('saves_export', { opts }),
  importSave: (opts) => invoke('saves_import', { opts }),
  deleteBackup: (backupPath) => invoke('saves_delete_backup', { backupPath }),
  // 普通存档 ↔ MOD 存档 转换
  convertSave: (opts) => invoke('saves_convert', { opts }),
};
