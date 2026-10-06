// 동작 종류 목록 — 표시 순서, 아이콘, 이름의 key, 그리고 종류를 처음 골랐을 때의 빈 동작
import type { RingAction, RingActionKind, RingLimits } from "@/entities/Ring";

export const ACTION_KINDS: Array<{ kind: RingActionKind; icon: string; labelKey: string }> = [
	{ kind: "open", icon: "app-window", labelKey: "kind.open" },
	{ kind: "shell", icon: "terminal", labelKey: "kind.shell" },
	{ kind: "keystroke", icon: "keyboard", labelKey: "kind.keystroke" },
	{ kind: "open_ring", icon: "circle-dot", labelKey: "kind.open_ring" }
];

/** "열기" 의 대상마다의 기본 아이콘. */
export const OPEN_TARGET_ICON: Record<"app" | "file" | "url", string> = {
	app: "app-window",
	file: "file-text",
	url: "globe"
};

/**
 * 사용자가 고른 아이콘이 아니라 편집기가 채운 아이콘인가. 그런 아이콘은 종류나 대상을 바꾸면 따라 바뀐다.
 * 사용자가 직접 고른 아이콘은 그대로 둔다.
 */
export function isAutoIcon(icon: string): boolean {
	return (
		icon === "" ||
		ACTION_KINDS.some((entry) => entry.icon === icon) ||
		Object.values(OPEN_TARGET_ICON).includes(icon)
	);
}

/** 그 종류의 기본 아이콘. 칸의 아이콘을 아직 고르지 않았을 때 쓴다. */
export function kindIcon(kind: RingActionKind): string {
	return ACTION_KINDS.find((entry) => entry.kind === kind)?.icon ?? "circle";
}

/** 종류를 처음 골랐을 때의 빈 동작. 값은 사용자가 채운다. 기본 제한 시간은 Rust 가 준 값이다. */
export function blankAction(kind: RingActionKind, limits: RingLimits): RingAction {
	switch (kind) {
		case "open":
			return { kind, target: "app", value: "" };
		case "shell":
			return { kind, command: "", working_dir: "", timeout_secs: limits.defaultShellTimeoutSecs };
		case "keystroke":
			return { kind, combos: [] };
		case "open_ring":
			return { kind, ring_id: "" };
	}
}

/** `order[새 자리] = 예전 자리`. `position` 칸을 한 칸 옆과 맞바꾼다. 끝에서는 반대쪽 끝과 바꾼다. */
export function swapOrder(
	slotCount: number,
	position: number,
	clockwise: boolean
): { order: number[]; target: number } {
	const target = (position + (clockwise ? 1 : slotCount - 1)) % slotCount;
	const order = Array.from({ length: slotCount }, (_, index) => index);
	order[position] = target;
	order[target] = position;
	return { order, target };
}

/** 경로의 마지막 이름. `.app` 은 뗀다. 칸 이름의 기본값으로 쓴다. */
export function baseName(path: string): string {
	const last = path.split("/").filter(Boolean).pop() ?? "";
	return last.replace(/\.app$/, "");
}
