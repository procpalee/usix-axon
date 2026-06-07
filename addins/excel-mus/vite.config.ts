import { defineConfig } from "vite";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

// Office 작업창은 dev 에서도 HTTPS 필수. office-addin-dev-certs 가 발급한 localhost
// 인증서를 읽어 https 서버로 띄운다. (`npm run certs` 로 최초 1회 설치.)
function devCerts(): { key: Buffer; cert: Buffer } | undefined {
  try {
    const dir = join(homedir(), ".office-addin-dev-certs");
    return {
      key: readFileSync(join(dir, "localhost.key")),
      cert: readFileSync(join(dir, "localhost.crt")),
    };
  } catch {
    return undefined; // 인증서 미설치 — `npm run certs` 안내. build 에는 무관.
  }
}

export default defineConfig({
  // .wasm 을 에셋으로 취급해 wasm-pkg 의 init() 이 fetch 할 수 있게.
  assetsInclude: ["**/*.wasm"],
  server: {
    port: 3000,
    strictPort: true,
    https: devCerts(),
  },
  build: {
    rollupOptions: {
      input: {
        taskpane: "src/taskpane.html",
        commands: "src/commands.html",
      },
    },
  },
});
