// 저장된 링 목록을 읽고, 바뀔 때마다 다시 읽는 hook — 설정 창이 쓴다
import { useCallback, useEffect, useState } from "react";
import { EVENTS } from "@/shared/tauri/events";
import { subscribe } from "@/shared/tauri/ipc";
import type { Ring, RingLimits } from "../domain/types";
import { ringRepository } from "../infrastructure/tauriRingRepository";

interface RingsState {
	/** 아직 읽지 못했으면 null. */
	rings: Ring[] | null;
	limits: RingLimits | null;
	reload: () => Promise<void>;
	/** Rust 가 돌려준 링으로 목록의 그 링을 바꾼다. */
	replace: (ring: Ring) => void;
}

export function useRings(): RingsState {
	const [rings, setRings] = useState<Ring[] | null>(null);
	const [limits, setLimits] = useState<RingLimits | null>(null);

	const reload = useCallback(async () => {
		try {
			setRings(await ringRepository.list());
		} catch (error) {
			console.error("[rings] failed to load:", error);
			setRings((current) => current ?? []);
		}
	}, []);

	useEffect(() => {
		void reload();
		void ringRepository.limits().then(setLimits, (error) => console.error("[rings] failed to read limits:", error));
		// 트레이나 가져오기가 링을 고쳐도 이 화면이 따라간다.
		return subscribe<void>(EVENTS.ringsChanged, () => void reload());
	}, [reload]);

	const replace = useCallback((next: Ring) => {
		setRings((current) => current?.map((ring) => (ring.id === next.id ? next : ring)) ?? null);
	}, []);

	return { rings, limits, reload, replace };
}
