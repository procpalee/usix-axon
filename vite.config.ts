import { defineConfig } from "vite";

// Tauri dev 서버 + 멀티페이지(메인 창 + 재무제표·검색·그래프·임포트 위저드 창).
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    rollupOptions: {
      input: {
        main: "index.html",
        statement: "statement.html",
        search: "search.html",
        chart: "chart.html",
        wizard: "wizard.html",
        workpaper: "workpaper.html",
      },
    },
  },
});
