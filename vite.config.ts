import { defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";

// 개발 서버 포트는 5185 다. Tome 의 개발 앱이 5175 를 쓴다.
export default defineConfig({
	clearScreen: false,
	base: "./",
	server: {
		port: 5185,
		strictPort: true,
		hmr: { protocol: "ws", host: "localhost" },
		watch: { ignored: ["**/src-tauri/**", "**/docs/**", "**/release/**"] }
	},
	plugins: [
		react({
			jsxImportSource: "@emotion/react",
			plugins: [["@swc/plugin-emotion", {}]]
		})
	],
	resolve: {
		tsconfigPaths: true,
		dedupe: ["react", "react-dom"]
	},
	build: {
		target: "safari16",
		outDir: "dist",
		emptyOutDir: true
	}
});
