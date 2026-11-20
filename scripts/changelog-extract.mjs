#!/usr/bin/env node
/**
 * 从 CHANGELOG.md 里取出指定版本的段落，输出到 stdout。
 *
 * 用途：Release 工作流用它生成 GitHub Release 的正文，
 * 让发布说明始终来自仓库里唯一的一份 CHANGELOG，而不是另开单版本文件。
 *
 * 用法：
 *   node scripts/changelog-extract.mjs 1.2.0
 *   node scripts/changelog-extract.mjs v1.2.0
 *   node scripts/changelog-extract.mjs Unreleased
 */
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const changelogPath = resolve(here, '..', 'CHANGELOG.md');

const want = (process.argv[2] || '').trim().replace(/^v/i, '');
if (!want) {
  console.error('用法: node scripts/changelog-extract.mjs <version|Unreleased>');
  process.exit(2);
}

const lines = readFileSync(changelogPath, 'utf8').split(/\r?\n/);

/** `## [1.2.0] - 2026-01-01` / `## [Unreleased]` */
const heading = /^##\s+\[([^\]]+)\]/;
const nameMatches = (name) => name.trim().toLowerCase() === want.toLowerCase();

let start = -1;
for (let i = 0; i < lines.length; i++) {
  const m = lines[i].match(heading);
  if (!m) continue;
  if (nameMatches(m[1])) {
    start = i;
    break;
  }
}

if (start === -1) {
  console.error(`CHANGELOG.md 里没有版本 "${process.argv[2]}" 的段落`);
  process.exit(1);
}

// 段落结束：下一个 "## [...]" 标题
let end = lines.length;
for (let i = start + 1; i < lines.length; i++) {
  if (heading.test(lines[i])) {
    end = i;
    break;
  }
}

const body = lines
  .slice(start + 1, end)
  .join('\n')
  // 去掉段落末尾的链接定义块
  .replace(/\n\[[^\]]+\]:\s+https?:\/\/\S+\s*$/gm, '')
  .trim();

if (!body) {
  console.error(`版本 "${process.argv[2]}" 的段落是空的`);
  process.exit(1);
}

process.stdout.write(body + '\n');
