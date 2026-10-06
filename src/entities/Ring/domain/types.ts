// 링의 타입 — Rust(`ring/model.rs`, `ring/controller.rs`)가 주는 모양 그대로다. 검증과 판정은 모두 Rust 에 있다

/** 칸 하나가 하는 일. Rust 의 `RingAction` 과 같은 모양이다 (serde tag `kind`). */
export type RingAction =
	// `args` 는 앱에 넘기는 인자다. 앱일 때만 쓰고, 없으면 key 가 없다.
	| { kind: "open"; target: "app" | "file" | "url"; value: string; args?: string }
	| { kind: "shell"; command: string; working_dir: string; timeout_secs: number }
	| { kind: "keystroke"; combos: string[] }
	| { kind: "open_ring"; ring_id: string };

export type RingActionKind = RingAction["kind"];

/** 칸 하나. `position` 0 이 12시 방향이고 시계 방향으로 자란다. */
export interface RingSlot {
	position: number;
	label: string;
	/** lucide 아이콘 이름 (kebab-case). */
	icon: string;
	action: RingAction;
	/** 실행 전에 확인을 묻는가. */
	confirm: boolean;
}

/** 링 하나. `slots` 에 없는 자리는 빈 칸이다. */
export interface Ring {
	id: string;
	name: string;
	/** 전역 단축키. 없으면 다른 링의 칸이나 트레이 메뉴로 연다. */
	shortcut: string | null;
	slotCount: number;
	slots: RingSlot[];
}

/** 어떤 링을 여는 칸. 그 링을 지우려 할 때 Rust 가 돌려준다. */
export interface RingSlotRef {
	ringId: string;
	ringName: string;
	position: number;
	label: string;
}

/** 하위 링으로 고를 수 있는 링. `blocked` 가 있으면 고를 수 없다 — 그 이유의 코드다. */
export interface RingLinkCandidate {
	id: string;
	name: string;
	/** 그 링에 채워진 칸의 수. 0 이면 열어도 고를 것이 없다. */
	filledSlots: number;
	blocked: string | null;
}

/** 한 칸이 하위 링으로 고를 수 있는 것 전부. */
export interface RingLinkChoices {
	candidates: RingLinkCandidate[];
	/** 이 칸에서 새 하위 링을 만들 수 없으면 그 이유의 코드다. */
	createBlocked: string | null;
}

/** 자리에서 만든 하위 링과, 그 링을 열게 된 칸의 링. */
export interface RingSubRingCreated {
	ring: Ring;
	subRing: Ring;
}

/** 설정 화면이 쓰는 한도와 기본값. 화면은 이 값을 받아 쓰고 스스로 정하지 않는다. */
export interface RingLimits {
	minSlots: number;
	maxSlots: number;
	nameMaxChars: number;
	maxCombos: number;
	defaultShellTimeoutSecs: number;
	maxShellTimeoutSecs: number;
	/** "실행 전에 확인" 이 기본으로 켜지는 동작 종류. */
	confirmByDefaultKinds: string[];
	/** "실행 전에 확인" 이 뜻이 없는 동작 종류. 이 종류에서는 그 토글을 보이지 않는다. */
	kindsWithoutConfirm: string[];
	/** 링에서 하위 링으로 내려가는 최대 단계. */
	maxDepth: number;
}

/** 가져오기가 무엇을 들이는지. Rust 의 `ImportSummary`. */
export interface RingImportSummary {
	rings: number;
	slots: number;
	shellSlots: number;
	keystrokeSlots: number;
	/** 파일·폴더나 앱을 여는 칸. 이것도 "실행 전에 확인" 이 켜진 채로 들어온다. */
	openSlots: number;
	skippedSlots: number;
	droppedShortcuts: number;
}

/** 링 창이 그릴 칸. 동작의 내용은 오지 않는다. */
export interface RingSlotView {
	position: number;
	label: string;
	icon: string;
	opensRing: boolean;
}

export interface RingView {
	id: string;
	name: string;
	slotCount: number;
	slots: RingSlotView[];
}

export interface RingConfirmView {
	label: string;
	icon: string;
	/** 무엇을 실행하는지. 셸 명령이면 그 명령이다. */
	detail: string;
}

/** `ring-show` 의 payload. */
export interface RingShow {
	/** 띄울 때마다 자란다. 더 작은 값은 버린다. */
	seq: number;
	ring: RingView;
	hovered: number | null;
	mode: "hold" | "open" | "confirm";
	/** 1 이 맨 위 링이다. */
	depth: number;
	confirm: RingConfirmView | null;
}

/** 링 창이 받은 키. 무엇을 할지는 Rust 가 정한다. */
export type RingKey =
	| { kind: "digit"; value: number }
	| { kind: "arrow"; direction: "up" | "down" | "left" | "right" }
	| { kind: "enter" }
	| { kind: "back" }
	| { kind: "escape" };
