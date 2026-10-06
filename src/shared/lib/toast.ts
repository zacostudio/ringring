// 화면 아래에 잠깐 보이는 한 줄 — 저장 실패처럼 자리에 붙일 곳이 없는 알림에 쓴다
import { createStore } from "./externalStore";

export interface Toast {
	id: number;
	text: string;
	/** 덧붙이는 글 (여러 줄일 수 있다). */
	detail?: string;
	tone: "error" | "info";
}

export const toastStore = createStore<Toast | null>(null);

let nextId = 1;
let timer: ReturnType<typeof setTimeout> | null = null;

/** 보이는 시간. 덧붙이는 글이 있으면 더 오래 둔다. */
const SHORT_MS = 4000;
const LONG_MS = 8000;

export function showToast(text: string, options: { detail?: string; tone?: Toast["tone"] } = {}) {
	if (timer) clearTimeout(timer);
	const toast: Toast = { id: nextId++, text, detail: options.detail, tone: options.tone ?? "error" };
	toastStore.set(toast);
	timer = setTimeout(() => dismissToast(toast.id), options.detail ? LONG_MS : SHORT_MS);
}

export function dismissToast(id: number) {
	if (toastStore.get()?.id === id) toastStore.set(null);
}
