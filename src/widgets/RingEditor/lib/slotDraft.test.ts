// 칸 초안 도우미의 테스트
import { describe, expect, test } from "bun:test";
import type { Ring, RingSlot } from "@/entities/Ring";
import { draftOf, savedSlot, slotSnapshot } from "./slotDraft";

const slot: RingSlot = {
	position: 2,
	label: "사이트",
	icon: "globe",
	action: { kind: "open", target: "url", value: "https://example.com" },
	confirm: false
};

const ring: Ring = { id: "a", name: "작업", shortcut: null, quickShortcut: null, slotCount: 6, slots: [slot] };

describe("slot draft", () => {
	test("finds the slot stored at a position", () => {
		expect(savedSlot(ring, 2)).toBe(slot);
		expect(savedSlot(ring, 0)).toBeNull();
	});

	test("an empty position gives an empty draft with no action", () => {
		expect(draftOf(null)).toEqual({ label: "", icon: "", action: null, confirm: false });
	});

	test("a stored slot gives a draft with its content", () => {
		expect(draftOf(slot)).toEqual({ label: "사이트", icon: "globe", action: slot.action, confirm: false });
	});

	test("the snapshot ignores the position and follows the content", () => {
		expect(slotSnapshot({ ...slot, position: 5 })).toBe(slotSnapshot(slot));
		expect(slotSnapshot({ ...slot, label: "다른 이름" })).not.toBe(slotSnapshot(slot));
		expect(slotSnapshot({ ...slot, confirm: true })).not.toBe(slotSnapshot(slot));
		expect(slotSnapshot(null)).toBe("");
	});
});
