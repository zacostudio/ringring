// 컴포넌트 밖에 두는 작은 상태 저장소 — React Context 없이 `useSyncExternalStore` 로 읽는다
export interface ExternalStore<T> {
	get: () => T;
	set: (next: T) => void;
	subscribe: (listener: () => void) => () => void;
}

export function createStore<T>(initial: T): ExternalStore<T> {
	let value = initial;
	const listeners = new Set<() => void>();
	return {
		get: () => value,
		set(next) {
			if (Object.is(next, value)) return;
			value = next;
			for (const listener of listeners) listener();
		},
		subscribe(listener) {
			listeners.add(listener);
			return () => listeners.delete(listener);
		}
	};
}
