#!/usr/bin/env node
/**
 * 把版本号同步到三个声明处。
 *
 * 这个项目的版本散落在 package.json、src-tauri/tauri.conf.json、
 * src-tauri/Cargo.toml，手工改容易漏，漏了就会出现"安装包版本和代码版本不一致"。
 *
 * 用法：
 *   node scripts/bump-version.mjs 1.2.0
 *
 * 只负责改版本号；CHANGELOG 与 git tag 由发布流程里人工确认。
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..');

const version = (process.argv[2] || '').trim().replace(/^v/i, '');
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error('用法: node scripts/bump-version.mjs <x.y.z>');
  process.exit(2);
}

/** 逐个文件替换，替换不到就报错，避免"以为改了其实没改" */
function patch(relPath, apply) {
  const path = resolve(root, relPath);
  const before = readFileSync(path, 'utf8');
  const after = apply(before);
  if (after === before) {
    console.error(`未在 ${relPath} 中找到可替换的版本号`);
    process.exit(1);
  }
  writeFileSync(path, after, 'utf8');
  console.log(`  ${relPath}`);
}

console.log(`设置版本号为 ${version}：`);

patch('package.json', (text) =>
  text.replace(/("version"\s*:\s*")[^"]+(")/, `$1${version}$2`)
);

patch('src-tauri/tauri.conf.json', (text) =>
  text.replace(/("version"\s*:\s*")[^"]+(")/, `$1${version}$2`)
);

patch('src-tauri/Cargo.toml', (text) => {
  // 只改 [package] 段里的 version，别碰依赖的版本号
  const block = /(\[package\][\s\S]*?)(?=\n\[)/;
  const m = text.match(block);
  if (!m) return text;
  const patched = m[1].replace(/^version\s*=\s*"[^"]+"/m, `version = "${version}"`);
  return text.replace(block, patched);
});

console.log(`
后续步骤：
  1. 在 CHANGELOG.md 里把 [Unreleased] 的内容归入 [${version}] 并补上日期
  2. git commit -am "chore(release): ${version}"
  3. git tag v${version} && git push origin main --follow-tags
`);
