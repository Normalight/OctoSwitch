import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 1420,
    strictPort: true
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.indexOf("node_modules/recharts") >= 0) return "vendor-recharts";
          if (id.indexOf("node_modules/@tauri-apps") >= 0) return "vendor-tauri";
          if (id.indexOf("node_modules") >= 0) return "vendor";
          return undefined;
        }
      }
    }
  }
});
