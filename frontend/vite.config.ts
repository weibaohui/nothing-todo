import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: 'dist',
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
