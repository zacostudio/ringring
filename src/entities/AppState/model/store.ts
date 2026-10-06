// 앱 상태의 저장소 — Rust 에서 읽은 값을 들고, 언어와 테마를 문서에 맞춘다
import { useSyncExternalStore } from "react";
import { setLocale } from "@/shared/i18n";
import { createStore } from "@/shared/lib/externalStore";
import { EVENTS } from "@/shared/tauri/events";
import { subscribe } from "@/shared/tauri/ipc";
import type { AppState, Theme } from "../domain/types";
import { appRepository } from "../infrastructure/tauriAppRepository";

const store = createStore<AppState | null>(null);

/** `system` 이면 속성을 뗀다. 그때는 CSS 의 `prefers-color-scheme` 이 정한다. */
function applyTheme(theme: Theme) {
	const root = document.documentElement;
	if (theme === "system") root.removeAttribute("data-theme");
	else root.setAttribute("data-theme", theme);
}

/** Rust 가 돌려준 상태를 넣는다. command 의 답으로 받은 값도 이 길로 넣는다. */
export function setAppState(next: AppState) {
	setLocale(next.locale);
	applyTheme(next.theme);
	document.documentElement.lang = next.locale;
	store.set(next);
}

export async function reloadAppState(): Promise<void> {
	try {
		setAppState(await appRepository.state());
	} catch (error) {
		console.error("[app] failed to read the app state:", error);
	}
}

let started = false;

/** 처음 한 번 읽고, 바뀔 때마다 다시 읽는다. 창마다 한 번 부른다. */
export function startAppState(): Promise<void> {
	if (!started) {
		started = true;
		subscribe<void>(EVENTS.appStateChanged, () => void reloadAppState());
		// 시스템 설정에서 권한을 바꾸고 돌아오면 상태가 달라져 있다.
		window.addEventListener("focus", () => void reloadAppState());
	}
	return reloadAppState();
}

/** 아직 읽지 못했으면 null. */
export function useAppState(): AppState | null {
	return useSyncExternalStore(store.subscribe, store.get);
}
