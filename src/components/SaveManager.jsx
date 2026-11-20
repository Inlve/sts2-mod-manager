import React, { useState, useEffect } from 'react';
import { Download, Upload, RefreshCw, Trash2, HardDrive, Gamepad2, FolderOpen, Clock, Trophy, Sword, Layers, CreditCard, AlertTriangle } from 'lucide-react';

function formatSize(bytes) {
  if (!bytes) return '0 B';
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

function formatTime(iso) {
  if (!iso) return '—';
  const d = new Date(iso);
  return d.toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' });
}

function formatPlaytime(seconds) {
  if (!seconds) return '0 分钟';
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return `${h} 小时 ${m} 分钟`;
  return `${m} 分钟`;
}

const SLOT_LABELS = { profile1: '存档 1', profile2: '存档 2', profile3: '存档 3' };

function SaveSummary({ summary, accent }) {
  if (!summary) return null;
  const accentColor = accent === 'purple' ? 'purple' : 'emerald';
  return (
    <div className="space-y-3 mb-4">
      {/* Stats row */}
      <div className="grid grid-cols-2 gap-2">
        <div className="bg-gray-50 rounded-lg px-3 py-2">
          <div className="flex items-center gap-1.5 text-gray-400 mb-0.5">
            <Clock size={11} /><span className="text-[10px] uppercase font-semibold">游戏时长</span>
          </div>
          <p className="text-xs font-medium text-gray-700">{formatPlaytime(summary.totalPlaytime)}</p>
        </div>
        <div className="bg-gray-50 rounded-lg px-3 py-2">
          <div className="flex items-center gap-1.5 text-gray-400 mb-0.5">
            <Trophy size={11} /><span className="text-[10px] uppercase font-semibold">得分</span>
          </div>
          <p className="text-xs font-medium text-gray-700">{summary.currentScore.toLocaleString()}</p>
        </div>
        <div className="bg-gray-50 rounded-lg px-3 py-2">
          <div className="flex items-center gap-1.5 text-gray-400 mb-0.5">
            <Layers size={11} /><span className="text-[10px] uppercase font-semibold">爬塔层数</span>
          </div>
          <p className="text-xs font-medium text-gray-700">{summary.floorsClimbed}</p>
        </div>
        <div className="bg-gray-50 rounded-lg px-3 py-2">
          <div className="flex items-center gap-1.5 text-gray-400 mb-0.5">
            <CreditCard size={11} /><span className="text-[10px] uppercase font-semibold">发现</span>
          </div>
          <p className="text-xs font-medium text-gray-700">{summary.discoveredCards} 卡 / {summary.discoveredRelics} 遗物</p>
        </div>
      </div>

      {/* Characters */}
      {summary.characters.length > 0 && (
        <div>
          <p className="text-[10px] font-semibold text-gray-400 uppercase mb-1.5">角色战绩</p>
          <div className="space-y-1">
            {summary.characters.map(c => (
              <div key={c.id} className="flex items-center justify-between text-xs px-2 py-1 bg-gray-50 rounded-md">
                <span className="font-medium text-gray-700">{c.name}</span>
                <span className="text-gray-500">
                  <span className={`text-${accentColor}-600 font-medium`}>{c.wins}胜</span>
                  <span className="mx-1">/</span>
                  <span className="text-red-400">{c.losses}负</span>
                  {c.maxAscension > 0 && <span className="ml-1.5 text-amber-500">A{c.maxAscension}</span>}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Runs */}
      {summary.epochs > 0 && (
        <p className="text-[10px] text-gray-400">共完成 {summary.epochs} 轮游戏</p>
      )}
    </div>
  );
}

function SlotCard({ slotName, slot, modded, onExport, onImport }) {
  const isEmpty = !slot || slot.empty;
  const accent = modded ? 'purple' : 'emerald';
  const borderClass = isEmpty ? 'border-gray-100' : modded ? 'border-purple-200 shadow-sm' : 'border-gray-200 shadow-sm';

  return (
    <div className={`bg-white rounded-xl border p-5 transition-colors ${borderClass}`}>
      <div className="flex items-center justify-between mb-3">
        <h3 className="font-semibold text-gray-900">
          {SLOT_LABELS[slotName]}
          {modded && <span className="text-xs text-purple-500 font-normal ml-1.5">MOD</span>}
        </h3>
        <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${
          isEmpty ? 'bg-gray-100 text-gray-400'
            : modded ? 'bg-purple-50 text-purple-600' : 'bg-emerald-50 text-emerald-600'
        }`}>
          {isEmpty ? '空' : '有数据'}
        </span>
      </div>

      {!isEmpty && slot.summary && (
        <SaveSummary summary={slot.summary} accent={accent} />
      )}

      {!isEmpty && !slot.summary && (
        <div className="text-xs text-gray-500 space-y-1 mb-4">
          <p>大小: {formatSize(slot.size)}</p>
          <p>最后修改: {formatTime(slot.lastModified)}</p>
        </div>
      )}

      {isEmpty && <p className="text-xs text-gray-400 mb-4">暂无存档数据</p>}

      {!isEmpty && (
        <div className="flex items-center gap-3 text-[10px] text-gray-400 mb-3">
          <span>{formatSize(slot.size)}</span>
          <span>·</span>
          <span>{formatTime(slot.lastModified)}</span>
        </div>
      )}

      <div className="flex gap-2">
        <button onClick={() => onExport(slotName, modded)}
          disabled={isEmpty}
          className={`flex-1 flex items-center justify-center gap-1.5 px-3 py-2 rounded-lg text-xs font-medium transition-colors ${
            isEmpty
              ? 'bg-gray-50 text-gray-300 cursor-not-allowed'
              : modded ? 'bg-purple-600 text-white hover:bg-purple-500' : 'bg-gray-900 text-white hover:bg-gray-800'
          }`}>
          <Download size={13} /> 导出
        </button>
        <button onClick={() => onImport(slotName, modded)}
          className="flex-1 flex items-center justify-center gap-1.5 px-3 py-2 rounded-lg text-xs font-medium border border-gray-200 text-gray-700 hover:bg-gray-50 transition-colors">
          <Upload size={13} /> 导入
        </button>
      </div>
    </div>
  );
}

export default function SaveManager() {
  const [data, setData] = useState({ slots: [], backups: [] });
  const [loading, setLoading] = useState(false);
  const [toast, setToast] = useState(null);
  // 存档转换：按后端是否真的注册了该命令来决定是否显示入口
  const canConvert = typeof window.api?.convertSave === 'function';
  const [includeReplays, setIncludeReplays] = useState(true);
  const [backupFirst, setBackupFirst] = useState(true);
  const [converting, setConverting] = useState(false);
  const [confirmConvert, setConfirmConvert] = useState(null);

  const showToast = (msg, type = 'success') => {
    setToast({ msg, type });
    setTimeout(() => setToast(null), 3000);
  };

  const refresh = async () => {
    setLoading(true);
    const result = await window.api.scanSaves();
    setData(result);
    setLoading(false);
  };

  useEffect(() => { refresh(); }, []);

  const handleConvert = async (slot, direction) => {
    setConfirmConvert(null);
    setConverting(true);
    try {
      const result = await window.api.convertSave({
        slot,
        direction,
        includeReplays,
        backup: backupFirst,
      });
      if (result.success) {
        const mb = (result.copiedBytes / (1024 * 1024)).toFixed(1);
        showToast(
          `${SLOT_LABELS[slot]} 已${direction === 'to_modded' ? '转为 MOD 存档' : '转为普通存档'}`
          + `（${result.copiedFiles} 个文件 / ${mb} MB${result.backupPath ? '，原存档已备份' : ''}）`
        );
        refresh();
      } else if (result.error) {
        showToast(result.error, 'error');
      }
    } catch (e) {
      showToast(e.message || '转换失败', 'error');
    }
    setConverting(false);
  };

  const handleExport = async (slot, modded) => {
    const result = await window.api.exportSave({ slot, modded });
    if (result.success) {
      showToast('存档导出成功');
      refresh();
    } else if (result.error) {
      showToast(result.error, 'error');
    }
  };

  const handleImport = async (slot, modded) => {
    const result = await window.api.importSave({ slot, modded });
    if (result.success) {
      showToast('存档导入成功（已自动备份原存档）');
      refresh();
    } else if (result.error) {
      showToast(result.error, 'error');
    }
  };

  const handleDeleteBackup = async (backupPath) => {
    const result = await window.api.deleteBackup(backupPath);
    if (result.success) {
      showToast('备份已删除');
      refresh();
    } else if (result.error) {
      showToast(result.error, 'error');
    }
  };

  const normalSlots = data.slots.filter(s => !s.modded);
  const moddedSlots = data.slots.filter(s => s.modded);

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {/* Header */}
      <div className="px-8 pt-6 pb-4">
        <div className="flex items-center justify-between mb-2">
          <div>
            <h1 className="text-2xl font-bold">存档管理</h1>
            <p className="text-sm text-gray-500 mt-1">
              导出、导入和备份你的游戏存档
            </p>
          </div>
          <div className="flex items-center gap-2">
            <button onClick={refresh}
              className="flex items-center gap-2 px-4 py-2 border border-gray-200 rounded-lg text-sm font-medium hover:bg-gray-100 transition-colors">
              <RefreshCw size={16} className={loading ? 'animate-spin' : ''} /> 刷新
            </button>
            <button onClick={() => window.api.openSavesDir()}
              className="flex items-center gap-2 px-4 py-2 border border-gray-200 rounded-lg text-sm font-medium hover:bg-gray-100 transition-colors">
              <FolderOpen size={16} /> 打开文件夹
            </button>
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-8 pb-6 space-y-6">
        {/* Normal saves */}
        <section>
          <h2 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3 flex items-center gap-2">
            <HardDrive size={14} /> 普通存档
          </h2>
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
            {['profile1', 'profile2', 'profile3'].map(slotName => (
              <SlotCard key={slotName}
                slotName={slotName}
                slot={normalSlots.find(s => s.slot === slotName)}
                modded={false}
                onExport={handleExport}
                onImport={handleImport}
              />
            ))}
          </div>
        </section>

        {/* Modded saves */}
        <section>
          <h2 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3 flex items-center gap-2">
            <Gamepad2 size={14} /> MOD 存档
          </h2>
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
            {['profile1', 'profile2', 'profile3'].map(slotName => (
              <SlotCard key={slotName}
                slotName={slotName}
                slot={moddedSlots.find(s => s.slot === slotName)}
                modded={true}
                onExport={handleExport}
                onImport={handleImport}
              />
            ))}
          </div>
        </section>

        {/* 普通 ↔ MOD 存档转换 */}
        {canConvert && (
          <section>
            <h2 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3 flex items-center gap-2">
              <Download size={14} /> 存档转换
            </h2>
            <div className="bg-white rounded-xl border border-gray-100 p-5">
              <p className="text-xs text-gray-500 leading-relaxed mb-3">
                游戏把两套进度完全隔离：普通存档和 MOD 存档互不可见。转换会把源槽位的
                存档（进度、偏好、历史记录，可选回放）复制到目标槽位并替换其内容。
              </p>

              <div className="flex items-center gap-4 mb-3">
                <label className="flex items-center gap-1.5 text-xs text-gray-600 cursor-pointer">
                  <input type="checkbox" checked={includeReplays} onChange={e => setIncludeReplays(e.target.checked)}
                    className="w-3.5 h-3.5 rounded border-gray-300 text-blue-500" />
                  同时转换回放
                </label>
                <label className="flex items-center gap-1.5 text-xs text-gray-600 cursor-pointer">
                  <input type="checkbox" checked={backupFirst} onChange={e => setBackupFirst(e.target.checked)}
                    className="w-3.5 h-3.5 rounded border-gray-300 text-blue-500" />
                  覆盖前自动备份原存档
                </label>
              </div>

              <div className="divide-y divide-gray-50">
                {['profile1', 'profile2', 'profile3'].map(slotName => {
                  const normal = normalSlots.find(s => s.slot === slotName);
                  const modded = moddedSlots.find(s => s.slot === slotName);
                  const hasNormal = normal && !normal.empty;
                  const hasModded = modded && !modded.empty;
                  return (
                    <div key={slotName} className="flex items-center justify-between py-2.5">
                      <div className="min-w-0">
                        <p className="text-xs font-medium text-gray-800">{SLOT_LABELS[slotName]}</p>
                        <p className="text-[11px] text-gray-400 mt-0.5">
                          普通：{hasNormal ? '有数据' : '空'}
                          <span className="mx-1.5 text-gray-200">|</span>
                          MOD：{hasModded ? '有数据' : '空'}
                        </p>
                      </div>
                      <div className="flex items-center gap-2 flex-shrink-0">
                        <button
                          disabled={!hasModded || converting}
                          onClick={() => setConfirmConvert({ slot: slotName, direction: 'to_normal' })}
                          className={`px-2.5 py-1.5 rounded-lg text-[11px] font-medium transition-colors ${
                            !hasModded || converting
                              ? 'bg-gray-50 text-gray-300 cursor-not-allowed'
                              : 'bg-purple-50 text-purple-700 hover:bg-purple-100'
                          }`}>
                          MOD → 普通
                        </button>
                        <button
                          disabled={!hasNormal || converting}
                          onClick={() => setConfirmConvert({ slot: slotName, direction: 'to_modded' })}
                          className={`px-2.5 py-1.5 rounded-lg text-[11px] font-medium transition-colors ${
                            !hasNormal || converting
                              ? 'bg-gray-50 text-gray-300 cursor-not-allowed'
                              : 'bg-emerald-50 text-emerald-700 hover:bg-emerald-100'
                          }`}>
                          普通 → MOD
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>

              {converting && (
                <p className="mt-3 text-[11px] text-amber-600 flex items-center gap-1.5">
                  <RefreshCw size={11} className="animate-spin" /> 正在转换，请勿关闭程序…
                </p>
              )}
            </div>
          </section>
        )}

        {/* Backups */}
        {data.backups.length > 0 && (
          <section>
            <h2 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3">
              自动备份记录
            </h2>
            <div className="bg-white rounded-xl border border-gray-100 divide-y divide-gray-50">
              {data.backups.map(b => (
                <div key={b.name} className="flex items-center justify-between px-5 py-3">
                  <div>
                    <p className="text-sm font-medium text-gray-800">{b.name}</p>
                    <p className="text-xs text-gray-400">{formatTime(b.time)} · {formatSize(b.size)}</p>
                  </div>
                  <button onClick={() => handleDeleteBackup(b.path)}
                    className="p-1.5 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-lg transition-colors"
                    title="删除备份">
                    <Trash2 size={14} />
                  </button>
                </div>
              ))}
            </div>
          </section>
        )}

        {data.slots.length === 0 && !loading && (
          <div className="flex flex-col items-center justify-center py-20 text-gray-400">
            <HardDrive size={48} className="mb-4" />
            <p className="text-lg font-medium">未检测到游戏存档</p>
            <p className="text-sm mt-1">请先运行一次游戏以创建存档目录</p>
          </div>
        )}
      </div>

      {/* 转换确认 */}
      {confirmConvert && (
        <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-50" onClick={() => setConfirmConvert(null)}>
          <div className="bg-white rounded-2xl shadow-2xl w-[420px] overflow-hidden" onClick={e => e.stopPropagation()}>
            <div className="flex items-center gap-3 px-6 py-4 border-b border-gray-100">
              <div className="w-10 h-10 rounded-full bg-amber-50 flex items-center justify-center">
                <AlertTriangle size={20} className="text-amber-500" />
              </div>
              <h3 className="font-bold text-gray-900">
                {SLOT_LABELS[confirmConvert.slot]}
                {confirmConvert.direction === 'to_modded' ? ' 普通 → MOD' : ' MOD → 普通'}
              </h3>
            </div>
            <div className="px-6 py-4 space-y-2">
              <p className="text-sm text-gray-600 leading-relaxed">
                目标槽位 <b>「{SLOT_LABELS[confirmConvert.slot]}」</b> 的存档将被源槽位内容
                <b>替换</b>，原有的当前局和历史记录会被覆盖。
              </p>
              <p className="text-xs text-gray-500 leading-relaxed">
                {backupFirst
                  ? '覆盖前会自动把目标槽位打包备份到 save_backups 目录，可随时恢复。'
                  : '⚠ 已关闭自动备份，此操作不可撤销。'}
              </p>
              <p className="text-xs text-gray-400">游戏正在运行时无法转换，请先退出游戏。</p>
            </div>
            <div className="px-6 py-4 border-t border-gray-100 flex gap-2">
              <button onClick={() => handleConvert(confirmConvert.slot, confirmConvert.direction)}
                className="flex-1 py-2.5 rounded-lg text-sm font-medium bg-gray-900 text-white hover:bg-gray-800 transition-colors">
                开始转换
              </button>
              <button onClick={() => setConfirmConvert(null)}
                className="flex-1 py-2.5 rounded-lg text-sm font-medium border border-gray-200 text-gray-700 hover:bg-gray-50 transition-colors">
                取消
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Toast */}
      {toast && (
        <div className={`fixed bottom-6 right-6 px-4 py-3 rounded-xl shadow-lg text-sm font-medium z-50 ${
          toast.type === 'error' ? 'bg-red-50 text-red-700 border border-red-200' : 'bg-emerald-50 text-emerald-700 border border-emerald-200'
        }`}>
          {toast.msg}
        </div>
      )}
    </div>
  );
}
