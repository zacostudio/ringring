// Tauri IPC 의 얇은 포장 — command 호출과 이벤트 구독. 컴포넌트는 이 파일만 거쳐 Rust 와 말한다
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
	return tauriInvoke<T>(command, args);
}

type Unlisten = () => void;

/**
 * 이벤트를 구독하고 푸는 함수를 돌려준다. 구독이 걸리기 전에 풀어도 된다.
 * `thisWindow` 면 이 창에 온 이벤트만 듣는다.
 */
export function subscribe<T>(event: string, handler: (payload: T) => void, thisWindow = false): Unlisten {
	let disposed = false;
	let off: Unlisten | null = null;
	const pending = thisWindow
		? getCurrentWebviewWindow().listen<T>(event, (received) => handler(received.payload))
		: tauriListen<T>(event, (received) => handler(received.payload));
	void pending.then((unlisten) => {
		if (disposed) unlisten();
		else off = unlisten;
	});
	return () => {
		disposed = true;
		off?.();
	};
}

/** 이 창의 라벨. `ring` 또는 `settings`. */
export function currentWindowLabel(): string {
	return getCurrentWebviewWindow().label;
}
