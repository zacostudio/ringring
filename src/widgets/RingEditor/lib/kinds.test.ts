// 동작 종류 도우미의 테스트
import { describe, expect, test } from "bun:test";
import type { RingLimits } from "@/entities/Ring";
import { ACTION_KINDS, OPEN_TARGET_ICON, baseName, blankAction, isAutoIcon, kindIcon, swapOrder } from "./kinds";

const limits: RingLimits = {
	minSlots: 4,
	maxSlots: 8,
	nameMaxChars: 24,
	maxCombos: 8,
	defaultShellTimeoutSecs: 120,
	maxShellTimeoutSecs: 3600,
	confirmByDefaultKinds: ["shell"],
	kindsWithoutConfirm: ["open_ring"],
	maxDepth: 3
};

describe("action kinds", () => {
	test("there are four kinds and none of Tome's", () => {
		expect(ACTION_KINDS.map((entry) => entry.kind)).toEqual(["open", "shell", "keystroke", "open_ring"]);
	});

	test("a blank action carries the kind and Rust's default time limit", () => {
		for (const { kind } of ACTION_KINDS) expect(blankAction(kind, limits).kind).toBe(kind);
		expect(blankAction("shell", limits)).toEqual({
			kind: "shell",
			command: "",
			working_dir: "",
			timeout_secs: 120
		});
	});

	test("every kind has its own icon", () => {
		expect(new Set(ACTION_KINDS.map((entry) => kindIcon(entry.kind))).size).toBe(ACTION_KINDS.length);
	});
});

describe("isAutoIcon", () => {
	test("an icon the editor filled in follows the kind, one the user picked does not", () => {
		expect(isAutoIcon("")).toBe(true);
		for (const { kind } of ACTION_KINDS) expect(isAutoIcon(kindIcon(kind))).toBe(true);
		for (const icon of Object.values(OPEN_TARGET_ICON)) expect(isAutoIcon(icon)).toBe(true);
		expect(isAutoIcon("rocket")).toBe(false);
	});
});

describe("swapOrder", () => {
	test("swaps a slot with its clockwise neighbour", () => {
		expect(swapOrder(4, 1, true)).toEqual({ order: [0, 2, 1, 3], target: 2 });
	});

	test("swaps a slot with its counter-clockwise neighbour", () => {
		expect(swapOrder(4, 1, false)).toEqual({ order: [1, 0, 2, 3], target: 0 });
	});

	test("wraps around the ring", () => {
		expect(swapOrder(4, 3, true)).toEqual({ order: [3, 1, 2, 0], target: 0 });
		expect(swapOrder(4, 0, false)).toEqual({ order: [3, 1, 2, 0], target: 3 });
	});

	test("always lists every position once", () => {
		for (let count = 4; count <= 8; count++) {
			for (let position = 0; position < count; position++) {
				const sorted = [...swapOrder(count, position, true).order].sort((a, b) => a - b);
				expect(sorted).toEqual(Array.from({ length: count }, (_, index) => index));
			}
		}
	});
});

describe("baseName", () => {
	test("drops the folder and the .app suffix", () => {
		expect(baseName("/Applications/Safari.app")).toBe("Safari");
		expect(baseName("/Users/me/notes.txt")).toBe("notes.txt");
		expect(baseName("/Users/me/Downloads/")).toBe("Downloads");
		expect(baseName("")).toBe("");
	});
});
