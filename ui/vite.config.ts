import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  build: {
    rollupOptions: {
      output: {
        manualChunks(id: string) {
          if (!id.includes("node_modules")) return undefined;

          if (id.includes("/@tiptap/") || id.includes("/tiptap-markdown/")) {
            return "editor-tiptap";
          }
          if (id.includes("/prosemirror-")) {
            return "editor-prosemirror";
          }
          if (id.includes("/marked/") || id.includes("/dompurify/")) {
            return "preview-markdown";
          }
          if (id.includes("/@tauri-apps/")) {
            return "tauri";
          }
          if (id.includes("/lucide-react/")) {
            return "icons";
          }
          if (id.includes("/react/") || id.includes("/react-dom/") || id.includes("/scheduler/")) {
            return "react";
          }
          if (id.includes("/zustand/")) {
            return "state";
          }

          return "vendor";
        },
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
