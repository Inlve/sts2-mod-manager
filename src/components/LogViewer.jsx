import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { FileText, RefreshCw, ChevronDown, ArrowDown } from 'lucide-react';

function colorLine(line) {
  if (line.includes('[ERROR]')) return 'text-red-600 bg-red-50';
  if (line.includes('[WARN]')) return 'text-amber-600 bg-amber-50';
  if (line.includes('[INFO]')) return 'text-gray-600';
  return 'text-gray-400';
}

/** 单行预估高度：leading-6(24) + py-0.5(4) + border-b(1) */
const ROW_ESTIMATE = 29;
/** 距底部多少像素内算"贴在底部"（留余量吸收动态测高的误差） */
const BOTTOM_THRESHOLD = 32;
/** 视口外多渲染几行，避免快速滚动时出现空白 */
const OVERSCAN = 24;
/** 自动刷新间隔 */
const AUTO_REFRESH_MS = 3000;
/** 搜索输入防抖 */
const SEARCH_DEBOUNCE_MS = 180;

/**
 * 单行日志。memo 让虚拟列表重渲染时跳过内容未变的行。
 *
 * whitespace-pre-wrap：保留前导缩进（堆栈跟踪靠它对齐），同时允许换行。
 * break-words：超长不可断词（长路径、base64）折行而不是横向溢出。
 *
 * 之所以不让它横向溢出：虚拟列表只渲染视口内的行，若靠横向滚动条显示长行，
 * 滚动条会随"当前视口里有没有长行"时有时无，非常跳。折行则永远不需要横向滚动。
 */
const LogRow = React.memo(function LogRow({ no, text }) {
  return (
    <div className={`px-4 py-0.5 border-b border-gray-50 whitespace-pre-wrap break-words ${colorLine(text)}`}>
      <span className="text-gray-300 mr-3 select-none">{String(no).padStart(5, ' ')}</span>
      {text}
    </div>
  );
});

export default function LogViewer() {
  const [files, setFiles] = useState([]);
  const [selectedFile, setSelectedFile] = useState('');
  const [content, setContent] = useState('');
  const [filterLevel, setFilterLevel] = useState('all');
  const [search, setSearch] = useState('');
  const [query, setQuery] = useState(''); // 防抖后的搜索词，真正参与过滤
  const [loading, setLoading] = useState(false);

  // 是否"贴在底部"跟随最新消息：用户往上翻就停，滚回底部自动恢复
  const [followBottom, setFollowBottom] = useState(true);
  const [autoRefresh, setAutoRefresh] = useState(true);

  const scrollRef = useRef(null);

  /**
   * 解析一次、复用多次：split / 去 CRLF / 小写化 / 错误告警计数
   * 只在 content 变化时执行，避免每次按键或每次渲染都重解析整份日志。
   */
  const parsed = useMemo(() => {
    const lines = content.split('\n');
    // 文件以换行结尾时会多出一个空元素，会渲染成多余的空行
    if (lines.length > 1 && lines[lines.length - 1] === '') lines.pop();

    let errorCount = 0;
    let warnCount = 0;
    const rows = new Array(lines.length);
    for (let i = 0; i < lines.length; i++) {
      // 兼容 CRLF 日志，去掉行尾的 \r
      const raw = lines[i];
      const text = raw.endsWith('\r') ? raw.slice(0, -1) : raw;
      if (text.includes('[ERROR]')) errorCount++;
      else if (text.includes('[WARN]')) warnCount++;
      rows[i] = { no: i + 1, text, lower: text.toLowerCase() };
    }
    return { rows, errorCount, warnCount };
  }, [content]);

  const { rows, errorCount, warnCount } = parsed;

  // 轻量过滤：复用预计算的小写串
  const entries = useMemo(() => {
    if (filterLevel === 'all' && !query) return rows;
    const out = [];
    for (let i = 0; i < rows.length; i++) {
      const r = rows[i];
      if (filterLevel === 'error' && !r.text.includes('[ERROR]')) continue;
      if (filterLevel === 'warn' && !r.text.includes('[WARN]') && !r.text.includes('[ERROR]')) continue;
      if (query && !r.lower.includes(query)) continue;
      out.push(r);
    }
    return out;
  }, [rows, filterLevel, query]);

  /**
   * 虚拟列表：只渲染视口内的行，因此日志有多少行都不影响 DOM 节点数。
   * 行高会被动态测量（长行换行会更高），先按单行高度预估。
   */
  const rowVirtualizer = useVirtualizer({
    count: entries.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_ESTIMATE,
    overscan: OVERSCAN,
    // 用日志真实行号做 key：过滤/刷新后测量结果仍能对上同一行
    getItemKey: (index) => entries[index]?.no ?? index,
  });

  // 数据集整体变化（切文件/改筛选/改搜索）时，旧的行高测量已失效，重置掉
  const datasetKey = `${selectedFile}\u0000${filterLevel}\u0000${query}`;
  useEffect(() => {
    rowVirtualizer.measure();
  }, [datasetKey, rowVirtualizer]);

  // 跟随底部：滚动到最后一行；动态测高会修正总高度，故下一帧再贴一次
  useEffect(() => {
    if (!followBottom || entries.length === 0) return;
    rowVirtualizer.scrollToIndex(entries.length - 1, { align: 'end' });
    const raf = requestAnimationFrame(() => {
      const el = scrollRef.current;
      if (el) el.scrollTop = el.scrollHeight;
    });
    return () => cancelAnimationFrame(raf);
  }, [entries, followBottom, rowVirtualizer]);

  // 监听滚动，判断是否还贴在底部
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const onScroll = () => {
      const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight <= BOTTOM_THRESHOLD;
      setFollowBottom(prev => (prev === atBottom ? prev : atBottom));
    };
    el.addEventListener('scroll', onScroll, { passive: true });
    return () => el.removeEventListener('scroll', onScroll);
  }, []);

  // 搜索防抖
  useEffect(() => {
    if (search === '') {
      setQuery('');
      return;
    }
    const t = setTimeout(() => setQuery(search.trim().toLowerCase()), SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(t);
  }, [search]);

  const loadLogs = useCallback(async () => {
    setLoading(true);
    const data = await window.api.getLatestLogs();
    setFiles(data.files);
    setContent(data.content);
    setFollowBottom(true);
    if (data.files.length > 0) setSelectedFile(data.files[0]);
    setLoading(false);
  }, []);

  useEffect(() => { loadLogs(); }, [loadLogs]);

  const handleFileChange = async (fileName) => {
    setSelectedFile(fileName);
    setLoading(true);
    setFollowBottom(true);
    const c = await window.api.readLog(fileName);
    setContent(c);
    setLoading(false);
  };

  // 自动刷新：内容没变时 setContent 传入同一字符串，React 会跳过重渲染，
  // 因此不会打断用户翻看历史
  useEffect(() => {
    if (!autoRefresh || !selectedFile) return;
    const id = setInterval(async () => {
      try {
        const c = await window.api.readLog(selectedFile);
        setContent(prev => (prev === c ? prev : c));
      } catch (e) {
        // 游戏正在写日志时可能读到半行，忽略这一轮即可
      }
    }, AUTO_REFRESH_MS);
    return () => clearInterval(id);
  }, [autoRefresh, selectedFile]);

  const jumpToBottom = () => setFollowBottom(true);

  const filtered = entries.length !== rows.length;

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {/* Header */}
      <div className="px-8 pt-6 pb-4">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h1 className="text-2xl font-bold">游戏日志</h1>
            <p className="text-sm text-gray-500 mt-1">
              {errorCount > 0 && <span className="text-red-500 font-medium">{errorCount} 个错误</span>}
              {errorCount > 0 && warnCount > 0 && ' · '}
              {warnCount > 0 && <span className="text-amber-500 font-medium">{warnCount} 个警告</span>}
              {errorCount === 0 && warnCount === 0 && '暂无错误或警告'}
            </p>
          </div>
          <div className="flex items-center gap-2">
            <label className="flex items-center gap-1.5 px-3 py-2 border border-gray-200 rounded-lg text-sm text-gray-600 cursor-pointer hover:bg-gray-100 transition-colors select-none">
              <input type="checkbox" checked={autoRefresh} onChange={(e) => setAutoRefresh(e.target.checked)}
                className="w-3.5 h-3.5 rounded border-gray-300 text-blue-500" />
              自动刷新
            </label>
            <button onClick={loadLogs}
              className="flex items-center gap-2 px-4 py-2 border border-gray-200 rounded-lg text-sm font-medium hover:bg-gray-100 transition-colors">
              <RefreshCw size={16} className={loading ? 'animate-spin' : ''} /> 刷新
            </button>
            <button onClick={() => window.api.openLogsDir()}
              className="flex items-center gap-2 px-4 py-2 border border-gray-200 rounded-lg text-sm font-medium hover:bg-gray-100 transition-colors">
              <FileText size={16} /> 打开文件夹
            </button>
          </div>
        </div>

        {/* Controls */}
        <div className="flex items-center gap-3">
          {/* File selector */}
          <div className="relative">
            <select value={selectedFile} onChange={(e) => handleFileChange(e.target.value)}
              className="appearance-none bg-white border border-gray-200 rounded-lg pl-3 pr-8 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-gray-200">
              {files.map(f => <option key={f} value={f}>{f}</option>)}
            </select>
            <ChevronDown size={14} className="absolute right-2.5 top-1/2 -translate-y-1/2 text-gray-400 pointer-events-none" />
          </div>

          {/* Filter */}
          <div className="flex bg-gray-100 rounded-lg p-0.5">
            {[['all', '全部'], ['warn', '警告+'], ['error', '仅错误']].map(([key, label]) => (
              <button key={key}
                onClick={() => setFilterLevel(key)}
                className={`px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${
                  filterLevel === key ? 'bg-white text-gray-900 shadow-sm' : 'text-gray-500 hover:text-gray-700'
                }`}>
                {label}
              </button>
            ))}
          </div>

          {/* Search */}
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="搜索日志..."
            className="flex-1 max-w-xs px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-gray-200"
          />

          <span className="text-[11px] text-gray-400 ml-auto">
            {filtered ? `${entries.length} / ${rows.length} 行` : `${rows.length} 行`}
            {!followBottom && <span className="ml-2 text-amber-500 font-medium">已暂停跟随</span>}
          </span>
        </div>
      </div>

      {/* Log content */}
      <div className="flex-1 overflow-hidden px-8 pb-6">
        <div className="relative h-full bg-white rounded-xl border border-gray-100 overflow-hidden">
          <div
            ref={scrollRef}
            className="h-full overflow-y-auto overflow-x-auto font-mono text-xs leading-6"
          >
            {entries.length === 0 ? (
              <div className="px-4 py-8 text-center text-gray-400 font-sans">暂无日志内容</div>
            ) : (
              <div style={{ height: rowVirtualizer.getTotalSize(), width: '100%', position: 'relative' }}>
                {rowVirtualizer.getVirtualItems().map((vItem) => {
                  const row = entries[vItem.index];
                  if (!row) return null;
                  return (
                    <div
                      key={vItem.key}
                      data-index={vItem.index}
                      ref={rowVirtualizer.measureElement}
                      style={{
                        position: 'absolute',
                        top: 0,
                        left: 0,
                        width: '100%',
                        transform: `translateY(${vItem.start}px)`,
                      }}
                    >
                      <LogRow no={row.no} text={row.text} />
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          {/* 回到底部：只在用户往上翻时出现 */}
          {!followBottom && (
            <button onClick={jumpToBottom}
              className="absolute bottom-4 right-5 flex items-center gap-1.5 px-3 py-2 rounded-full bg-gray-900 text-white text-xs font-medium shadow-lg hover:bg-gray-800 transition-colors">
              <ArrowDown size={13} /> 回到底部
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
