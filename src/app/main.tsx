// 프런트의 진입점 — 창의 라벨을 보고 링 페이지나 설정 페이지를 그린다
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { startAppState } from "@/entities/AppState";
import { RingPage } from "@/pages/RingPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { currentWindowLabel } from "@/shared/tauri/ipc";
import "./global.css";

const isRing = currentWindowLabel() === "ring";
document.documentElement.dataset.window = isRing ? "ring" : "settings";

// 언어와 테마는 Rust 가 정한다. 창마다 한 번 읽고, 바뀔 때마다 따라간다.
void startAppState();

const root = document.getElementById("root");
if (root) {
	createRoot(root).render(<StrictMode>{isRing ? <RingPage /> : <SettingsPage />}</StrictMode>);
}
