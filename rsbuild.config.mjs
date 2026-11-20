import { defineConfig } from '@rsbuild/core';
import { pluginReact } from '@rsbuild/plugin-react';

/**
 * Rsbuild 配置（替代原来的 webpack + babel 链路）
 *
 * - JSX 由 @rsbuild/plugin-react 走 SWC 编译，不再需要 babel-loader
 * - CSS 里的 @tailwind 指令由 postcss.config.js 处理，无需单独的 tailwind CLI 步骤
 * - 产物输出到 dist/，Tauri 的 frontendDist 指向它
 */
export default defineConfig({
  plugins: [pluginReact()],

  html: {
    template: './src/index.html',
  },

  source: {
    entry: { index: './src/index.jsx' },
  },

  output: {
    distPath: { root: 'dist' },
    // Tauri 用自定义协议从产物根目录提供资源，保持默认的绝对路径即可
    cleanDistPath: true,
  },

  server: {
    port: 3000,
    strictPort: true,
  },

  performance: {
    // 桌面应用加载本地文件，不需要拆包优化；保持单文件更好排查
    chunkSplit: { strategy: 'all-in-one' },
  },
});
