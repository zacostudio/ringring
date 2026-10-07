// RingRepository 의 Tauri IPC 구현 — Rust 의 `commands/rings.rs` · `ring_window.rs` · `transfer.rs`
import { invoke } from "@/shared/tauri/ipc";
import type { RingRepository } from "../domain/repository";
import type {
	Ring,
	RingImportSummary,
	RingKey,
	RingLimits,
	RingLinkChoices,
	RingShow,
	RingSlot,
	RingSlotRef,
	RingSubRingCreated
} from "../domain/types";

export const ringRepository: RingRepository = {
	list: () => invoke<Ring[]>("rings_list"),
	create: () => invoke<Ring>("ring_create"),
	createStarter: () => invoke<Ring>("ring_create_starter"),
	save: (ringId, name, slotCount) => invoke<Ring>("ring_save", { ringId, name, slotCount }),
	setShortcut: (ringId, kind, shortcut) => invoke<void>("ring_set_shortcut", { ringId, kind, shortcut }),
	saveSlot: (ringId: string, slot: RingSlot) => invoke<Ring>("slot_save", { ringId, slot }),
	clearSlot: (ringId, position) => invoke<Ring>("slot_clear", { ringId, position }),
	reorderSlots: (ringId, order) => invoke<Ring>("slots_reorder", { ringId, order }),
	delete: (ringId) => invoke<RingSlotRef[]>("ring_delete", { ringId }),
	linkCandidates: (ringId, position) => invoke<RingLinkChoices>("ring_link_candidates", { ringId, position }),
	createSubRing: (ringId, position, label, icon) =>
		invoke<RingSubRingCreated>("ring_create_sub", { ringId, position, label, icon }),
	limits: () => invoke<RingLimits>("ring_limits"),
	chooseOpenTarget: (target) => invoke<string | null>("open_target_choose", { target }),
	show: (ringId) => invoke<void>("ring_show", { ringId }),
	exportTo: (path) => invoke<number>("rings_export", { path }),
	importPreview: (path) => invoke<RingImportSummary>("rings_import_preview", { path }),
	importFrom: (path) => invoke<RingImportSummary>("rings_import", { path }),

	current: () => invoke<RingShow | null>("ring_current"),
	pick: () => invoke<void>("ring_pick"),
	key: (key: RingKey) => invoke<void>("ring_key", { key }),
	confirm: (accepted) => invoke<void>("ring_confirm", { accepted }),
	hide: () => invoke<void>("ring_hide"),
	painted: (seq) => invoke<void>("ring_painted", { report: { seq } })
};
