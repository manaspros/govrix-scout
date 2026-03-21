import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 3000,
    proxy: {
      '/api': { target: 'http://localhost:4001', changeOrigin: true },
      '/health': { target: 'http://localhost:4001', changeOrigin: true },
      '/ready': { target: 'http://localhost:4001', changeOrigin: true },
      '/metrics': { target: 'http://localhost:4001', changeOrigin: true },
    },
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
    rollupOptions: {
      output: {
        manualChunks: {
          'react-vendor': ['react', 'react-dom', 'react-router-dom'],
          'query-vendor': ['@tanstack/react-query'],
          'chart-vendor': ['recharts'],
          'util-vendor': ['date-fns', 'clsx', 'lucide-react'],
        },
      },
    },
  },
})
