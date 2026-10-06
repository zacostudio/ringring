// 링 리포지토리 인터페이스 — 설정 창의 링 편집과 링 창의 입력 전달
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
} from "./types";

export interface RingRepository {
	list(): Promise<Ring[]>;
	/** 빈 링. 6칸이고 단축키는 없다. */
	create(): Promise<Ring>;
	/** 기본 링. 이 기기에 있는 앱과 폴더가 칸으로 들어간다. */
	createStarter(): Promise<Ring>;
	/** 이름과 칸 수. 칸 수를 줄이면 넘치는 칸은 지워진다. */
	save(ringId: string, name: string, slotCount: number): Promise<Ring>;
	/** 전역 단축키. `null` 이면 뗀다. 쓸 수 없는 조합이면 저장되지 않고 reject 된다. */
	setShortcut(ringId: string, shortcut: string | null): Promise<void>;
	saveSlot(ringId: string, slot: RingSlot): Promise<Ring>;
	clearSlot(ringId: string, position: number): Promise<Ring>;
	/** `order[새 자리] = 예전 자리`. */
	reorderSlots(ringId: string, order: number[]): Promise<Ring>;
	/** 지운다. 그 링을 여는 칸이 있으면 지우지 않고 그 칸들을 돌려준다. 빈 배열이면 지운 것이다. */
	delete(ringId: string): Promise<RingSlotRef[]>;
	/** 이 칸이 하위 링으로 열 수 있는 링 목록과, 새 하위 링을 만들 수 있는지. */
	linkCandidates(ringId: string, position: number): Promise<RingLinkChoices>;
	/** 빈 하위 링을 만들고 그 칸에 잇는다. `label` 과 `icon` 은 화면의 초안이고, 비어 있으면 Rust 가 채운다. */
	createSubRing(ringId: string, position: number, label: string, icon: string): Promise<RingSubRingCreated>;
	limits(): Promise<RingLimits>;
	/** "열기" 칸의 대상을 고르는 창을 연다. 고른 경로를 돌려준다. 취소하면 null. */
	chooseOpenTarget(target: "app" | "file" | "url"): Promise<string | null>;
	/** 링을 띄운다. 클릭이나 키로 고르는 상태다. */
	show(ringId: string): Promise<void>;
	/** 링 전체를 파일에 쓴다. 쓴 링의 수를 돌려준다. */
	exportTo(path: string): Promise<number>;
	/** 파일이 무엇을 들이는지. 아무것도 저장하지 않는다. */
	importPreview(path: string): Promise<RingImportSummary>;
	/** 파일의 링을 더한다. */
	importFrom(path: string): Promise<RingImportSummary>;

	// ── 링 창만 부른다 ──
	current(): Promise<RingShow | null>;
	pick(): Promise<void>;
	key(key: RingKey): Promise<void>;
	confirm(accepted: boolean): Promise<void>;
	hide(): Promise<void>;
	painted(seq: number): Promise<void>;
}
