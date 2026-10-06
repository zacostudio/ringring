// 칸 편집기의 초안 — 저장된 칸에서 초안을 만들고, 그 자리의 칸이 바뀌었는지 알아본다
import type { Ring, RingAction, RingSlot } from "@/entities/Ring";

/** 편집 중인 칸. 동작을 아직 고르지 않았으면 `action` 이 null 이다. */
export interface SlotDraft {
	label: string;
	icon: string;
	action: RingAction | null;
	confirm: boolean;
}

/** `position` 자리에 저장된 칸. 빈 칸이면 null. */
export function savedSlot(ring: Ring, position: number): RingSlot | null {
	return ring.slots.find((slot) => slot.position === position) ?? null;
}

/** 저장된 칸으로 초안을 만든다. 빈 칸이면 빈 초안이다. */
export function draftOf(slot: RingSlot | null): SlotDraft {
	return slot
		? { label: slot.label, icon: slot.icon, action: slot.action, confirm: slot.confirm }
		: { label: "", icon: "", action: null, confirm: false };
}

/**
 * 그 자리에 저장된 칸의 내용을 글자로 적는다. 빈 칸은 빈 문자열이다.
 *
 * 편집기는 초안을 만들 때의 이 값을 들고 있다가, 값이 달라지면 초안을 버리고 새로 만든다.
 * 순서 바꾸기, 가져오기, 칸 비우기가 모두 이 값을 바꾼다. 자리 번호는 넣지 않는다 —
 * 같은 내용의 칸이 그 자리에 있으면 초안을 버릴 이유가 없다.
 */
export function slotSnapshot(slot: RingSlot | null): string {
	return slot ? JSON.stringify([slot.label, slot.icon, slot.action, slot.confirm]) : "";
}
