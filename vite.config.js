import { defineConfig } from 'vite'
import { resolve } from 'path'
import removeConsole from 'vite-plugin-remove-console'

export default defineConfig({
  root: 'src',
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },

  envPrefix: ['VITE_', 'TAURI_'],

  // 插件配置
  plugins: [
    // 判断是否为生产环境
    process.env.NODE_ENV === 'production' || (!process.env.TAURI_DEBUG && process.env.NODE_ENV !== 'development')
      ? removeConsole({
        includes: ['log', 'debug', 'info'],
        excludes: ['error', 'warn']
      })
      : null
  ].filter(Boolean),
  build: {
    outDir: '../dist',
    target: process.env.TAURI_PLATFORM == 'windows' ? 'chrome105' : 'safari13',
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'src/index.html'),
        settings: resolve(__dirname, 'src/settings.html'),
        preview: resolve(__dirname, 'src/preview.html'),
        screenshot: resolve(__dirname, 'src/screenshot.html'),
        textEditor: resolve(__dirname, 'src/textEditor.html'),
      },
    },
  },
})