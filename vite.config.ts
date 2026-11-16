import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Vite config tuned for Tauri: relative asset base (the webview loads over a
// custom protocol in production), a fixed dev port Tauri waits on, and we keep
// the console noise down / ignore the Rust target dir from the file watcher.
export default defineConfig({
  plugins: [react()],
  base: "./",
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
