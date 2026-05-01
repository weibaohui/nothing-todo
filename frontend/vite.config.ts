import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: 'dist',
    assetsInlineLimit: 4096, // 4kb以下的资源内联
    rollupOptions: {
      output: {
        manualChunks: {
          // 第三方库分割
          'vendor-react': ['react', 'react-dom'],
          'vendor-antd': ['antd', '@ant-design/icons', '@ant-design/pro-components'],
          'vendor-markdown': ['cherry-markdown'],
        },
      },
    },
  },
  server: {
    port: 5173,
    strictPort: false,
    proxy: {
      '/xyz': {
        target: 'http://127.0.0.1:8088',
        changeOrigin: true,
        ws: true,
      },
    },
  },
});
