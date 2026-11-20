/**
 * 日志视图 UI 冒烟测试（Playwright + 本地 dev server）
 *
 * 为什么需要 mock：前端通过 window.__TAURI__ 走 Tauri IPC，
 * 直接用浏览器打开时不存在，tauri-api.js 会立刻抛错、React 挂不上。
 * 这里在页面脚本执行前注入一份假实现，把命令换成可控数据。
 *
 * 用法：
 *   1) 另开一个终端跑 `npm run dev`（:3000）
 *   2) node tests/log-viewer.smoke.mjs
 */
import { chromium } from 'playwright';

const URL = process.env.SMOKE_URL || 'http://localhost:3000';
const TOTAL_LINES = 20000;
const ROW_ESTIMATE = 29;
/** 该行内含 400 个不可断字符，用来验证折行 + 动态测高 */
const LONG_LINE_NO = 777;
const OUT_DIR = 'tests/artifacts';

const failures = [];
function check(name, ok, detail) {
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? '  — ' + detail : ''}`);
  if (!ok) failures.push(name);
}
function note(msg) {
  console.log('      ' + msg);
}

const initScript = ({ totalLines }) => {
  const longToken = 'X'.repeat(600);
  const buildLines = (from, to) => {
    const lines = [];
    for (let i = from; i <= to; i++) {
      if (i === 3) lines.push(`[ERROR] Synthetic failure at line ${i} ${longToken}`);
      else if (i % 1000 === 0) lines.push(`[WARN] periodic warning ${i}`);
      else if (i === 777) lines.push(`[INFO] long line ${i} ${'Y'.repeat(400)}`);
      else lines.push(`[INFO] Monster DEVOTED_SCULPTOR performing move SAVAGE_MOVE seq=${i}`);
    }
    return lines;
  };

  let lines = buildLines(1, totalLines);
  let appended = 0;

  const handlers = {
    app_init: () => ({ gamePath: 'E:\\SteamLibrary\\steamapps\\common\\Slay the Spire 2', modsDir: 'mods', gameVersion: 'v0.113.1', workshopDirs: 1 }),
    app_select_game_path: () => null,
    game_get_state: () => 'idle',
    game_get_version: () => ({ version: 'v0.113.1', engine: 'Godot 4' }),
    game_launch: () => ({ success: true }),
    game_analyze_crash: () => ({ issues: [], logFile: null, errorCount: 0, warnCount: 0, involvedMods: [], loadedMods: [], notices: [] }),
    mods_scan: () => [],
    profiles_load: () => ({}),
    profiles_save: () => ({ success: true }),
    translations_load: () => ({}),
    translations_save: () => ({ success: true }),
    logs_get_latest: () => ({ files: ['godot.log'], content: lines.join('\n') + '\n' }),
    // 每次都追加新行，模拟游戏持续写日志
    logs_read: () => {
      appended += 1;
      lines = lines.concat(buildLines(totalLines + appended * 50 - 49, totalLines + appended * 50));
      return lines.join('\n') + '\n';
    },
    shell_open_logs_dir: () => null,
    shell_open_mods_dir: () => null,
    shell_open_game_dir: () => null,
    shell_open_saves_dir: () => null,
    shell_open_url: () => null,
    saves_scan: () => ({ slots: [], backups: [] }),
  };

  window.__TAURI__ = {
    core: { invoke: async (cmd) => (handlers[cmd] ? handlers[cmd]() : null) },
    window: { getCurrentWindow: () => ({}) },
  };
};

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 1 });
const page = await context.newPage();
await page.addInitScript(initScript, { totalLines: TOTAL_LINES });

const pageErrors = [];
page.on('pageerror', (e) => pageErrors.push(String(e)));
page.on('console', (m) => { if (m.type() === 'error') pageErrors.push('console: ' + m.text()); });

await page.goto(URL, { waitUntil: 'domcontentloaded' });
await page.locator('button', { hasText: '游戏日志' }).first().click();
await page.waitForSelector('[data-index]', { timeout: 15000 });
await page.waitForTimeout(600);

const probe = () => page.evaluate(() => {
  const first = document.querySelector('[data-index]');
  const scroller = first.closest('div.overflow-y-auto');
  const spacer = first.parentElement;
  return {
    rows: document.querySelectorAll('[data-index]').length,
    spacerHeight: Math.round(parseFloat(spacer.style.height || '0')),
    scrollTop: Math.round(scroller.scrollTop),
    scrollHeight: Math.round(scroller.scrollHeight),
    clientHeight: Math.round(scroller.clientHeight),
    scrollWidth: Math.round(scroller.scrollWidth),
    clientWidth: Math.round(scroller.clientWidth),
    atBottom: Math.abs(scroller.scrollTop - (scroller.scrollHeight - scroller.clientHeight)) <= 40,
    hasBackBtn: Array.from(document.querySelectorAll('button')).some(b => b.textContent.includes('回到底部')),
    pausedLabel: document.body.innerText.includes('已暂停跟随'),
    countText: document.body.innerText.match(/(\d+)(?:\s*\/\s*\d+)?\s*行/)?.[0] || '(未找到)',
  };
});

const initial = await probe();
console.log('\n--- 初始状态（20,000 行日志） ---');
note(`DOM 渲染行数: ${initial.rows}（总行数 ${TOTAL_LINES}）`);
note(`虚拟容器高度: ${initial.spacerHeight}px = ${TOTAL_LINES} × ${ROW_ESTIMATE}px`);
note(`scrollTop=${initial.scrollTop} / max=${initial.scrollHeight - initial.clientHeight}`);
note(`scrollWidth=${initial.scrollWidth}, clientWidth=${initial.clientWidth}`);
note(`顶部统计: ${initial.countText}`);
note('（headless Chromium 用覆盖式滚动条，offset/client 差值恒为 0，无法在此测滚动条厚度 → 见文末说明）');

console.log('\n--- 断言 ---');
check('虚拟化生效（DOM 行数远小于总行数）', initial.rows > 0 && initial.rows < TOTAL_LINES / 10, `${initial.rows} vs ${TOTAL_LINES}`);
check('虚拟容器高度 = 行数 × 单行预估', Math.abs(initial.spacerHeight - TOTAL_LINES * ROW_ESTIMATE) < 200, `${initial.spacerHeight}px`);
check('首次进入自动贴底', initial.atBottom);
check('无横向溢出（长行折行而非撑宽）', initial.scrollWidth <= initial.clientWidth + 2, `${initial.scrollWidth} vs ${initial.clientWidth}`);
check('顶部行数统计正确', initial.countText.includes(String(TOTAL_LINES)), initial.countText);
check('无 JS 运行时错误', pageErrors.length === 0, pageErrors.slice(0, 2).join(' | '));

// ── 滚动条样式：headless 用覆盖式滚动条（不占布局），但伪元素样式仍可查询 ──
// 这是"横竖必须同时指定 width/height"那条修复的直接验证
const scrollbarStyle = await page.evaluate(() => {
  const first = document.querySelector('[data-index]');
  const scroller = first.closest('div.overflow-y-auto');
  const s = getComputedStyle(scroller, '::-webkit-scrollbar');
  return { width: s.width, height: s.height };
});
console.log('\n--- 滚动条伪元素样式 ---');
note(`::-webkit-scrollbar → width=${scrollbarStyle.width}, height=${scrollbarStyle.height}`);
check('纵向滚动条宽度为 6px', scrollbarStyle.width === '6px', scrollbarStyle.width);
check('横向滚动条高度同为 6px（与纵向一致）', scrollbarStyle.height === '6px', scrollbarStyle.height);

// ── 动态测高：滚到含超长 token 的那一行，验证它确实被测量成多行高 ──
await page.evaluate((no) => {
  const first = document.querySelector('[data-index]');
  const scroller = first.closest('div.overflow-y-auto');
  scroller.scrollTop = Math.max(0, (no - 1) * 29 - 200);
}, LONG_LINE_NO);
await page.waitForTimeout(700);

const longLine = await page.evaluate(() => {
  const rows = Array.from(document.querySelectorAll('[data-index]'));
  const target = rows.find(el => el.textContent.includes('Y'.repeat(50)));
  const first = document.querySelector('[data-index]');
  const scroller = first.closest('div.overflow-y-auto');
  return {
    found: !!target,
    height: target ? Math.round(target.getBoundingClientRect().height) : 0,
    text: target ? target.textContent.slice(0, 40) : '',
    scrollWidth: Math.round(scroller.scrollWidth),
    clientWidth: Math.round(scroller.clientWidth),
  };
});
console.log('\n--- 动态测高（第 777 行，含 400 字符不可断 token） ---');
note(longLine.found ? `命中行高 ${longLine.height}px（单行预估 ${ROW_ESTIMATE}px）` : '未在视口内找到目标行');
note(`该位置 scrollWidth=${longLine.scrollWidth}, clientWidth=${longLine.clientWidth}`);
check('长行被渲染且动态测高生效（行高 > 单行预估）', longLine.found && longLine.height > ROW_ESTIMATE, `${longLine.height}px`);
check('长行折行、不产生横向滚动', longLine.scrollWidth <= longLine.clientWidth + 2, `${longLine.scrollWidth} vs ${longLine.clientWidth}`);

// ── 上翻暂停跟随 ──
await page.locator('button', { hasText: '回到底部' }).first().click().catch(() => {});
await page.waitForTimeout(500);
await page.evaluate(() => {
  const first = document.querySelector('[data-index]');
  const scroller = first.closest('div.overflow-y-auto');
  scroller.scrollTop = scroller.scrollTop - 3000;
});
await page.waitForTimeout(400);
const paused = await probe();
check('上翻后出现「回到底部」按钮', paused.hasBackBtn);
check('上翻后显示「已暂停跟随」', paused.pausedLabel);

// ── 暂停跟随时自动刷新写入新行，视口不应跳动 ──
const beforeRefresh = await probe();
await page.waitForTimeout(4500); // 跨越一次 3s 自动刷新
const afterRefresh = await probe();
note(`刷新前后：scrollHeight ${beforeRefresh.scrollHeight} → ${afterRefresh.scrollHeight}，scrollTop ${beforeRefresh.scrollTop} → ${afterRefresh.scrollTop}`);
check(
  '暂停跟随期间确实有新日志写入',
  afterRefresh.scrollHeight > beforeRefresh.scrollHeight,
  `增加 ${afterRefresh.scrollHeight - beforeRefresh.scrollHeight}px`
);
check(
  '暂停跟随期间视口不跳动',
  Math.abs(afterRefresh.scrollTop - beforeRefresh.scrollTop) <= 40,
  `位移 ${Math.abs(afterRefresh.scrollTop - beforeRefresh.scrollTop)}px`
);

await page.screenshot({ path: `${OUT_DIR}/log-viewer-paused.png` });

// ── 回到底部 → 恢复跟随 ──
await page.locator('button', { hasText: '回到底部' }).first().click();
await page.waitForTimeout(700);
const resumed = await probe();
check('点「回到底部」后到达底部', resumed.atBottom);
check('恢复跟随后按钮消失', !resumed.hasBackBtn);
check('恢复跟随后「已暂停跟随」提示消失', !resumed.pausedLabel);

await page.screenshot({ path: `${OUT_DIR}/log-viewer-bottom.png` });

await browser.close();

console.log(`\n===== 结果：${failures.length === 0 ? '全部通过' : failures.length + ' 项失败'} =====`);
if (failures.length) {
  console.log('失败项：');
  failures.forEach(f => console.log('  - ' + f));
  process.exit(1);
}
