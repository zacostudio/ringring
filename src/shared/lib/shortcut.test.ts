// 단축키 글 만들기와 표시의 테스트
import { describe, expect, test } from "bun:test";
import { chordToShortcut, formatShortcut, isModifierOnly, shortcutParts } from "./shortcut";

const chord = (code: string, mods: Partial<Record<"metaKey" | "ctrlKey" | "altKey" | "shiftKey", boolean>> = {}) => ({
	code,
	metaKey: false,
	ctrlKey: false,
	altKey: false,
	shiftKey: false,
	...mods
});

describe("chordToShortcut", () => {
	test("builds the stored text from the physical key", () => {
		expect(chordToShortcut(chord("KeyG", { metaKey: true, shiftKey: true }))).toBe("CmdOrCtrl+Shift+KeyG");
		expect(chordToShortcut(chord("Space", { altKey: true }))).toBe("Alt+Space");
		expect(chordToShortcut(chord("F18", { ctrlKey: true, altKey: true, shiftKey: true }))).toBe(
			"Ctrl+Alt+Shift+F18"
		);
	});

	test("keeps command and control apart when both are held", () => {
		expect(chordToShortcut(chord("KeyK", { metaKey: true, ctrlKey: true }))).toBe("CmdOrCtrl+Ctrl+KeyK");
	});

	test("a bare key is still a combination — Rust decides whether it may be used", () => {
		expect(chordToShortcut(chord("KeyA"))).toBe("KeyA");
		expect(chordToShortcut(chord("Escape"))).toBe("Escape");
	});

	test("refuses keys the parser does not know", () => {
		expect(chordToShortcut(chord("Lang1", { metaKey: true }))).toBeNull();
		expect(chordToShortcut(chord("IntlBackslash", { metaKey: true }))).toBeNull();
		expect(chordToShortcut(chord("ShiftLeft", { shiftKey: true }))).toBeNull();
	});
});

describe("isModifierOnly", () => {
	test("is true only while a modifier key itself is the pressed key", () => {
		expect(isModifierOnly(chord("MetaLeft", { metaKey: true }))).toBe(true);
		expect(isModifierOnly(chord("AltRight", { altKey: true }))).toBe(true);
		expect(isModifierOnly(chord("KeyA", { metaKey: true }))).toBe(false);
	});
});

describe("formatShortcut", () => {
	test("shows modifier symbols and short key names", () => {
		expect(shortcutParts("CmdOrCtrl+Shift+KeyG")).toEqual(["⌘", "⇧", "G"]);
		expect(formatShortcut("CmdOrCtrl+Shift+KeyG")).toBe("⌘⇧G");
		expect(formatShortcut("Alt+Space")).toBe("⌥ Space");
		expect(formatShortcut("Ctrl+Alt+Shift+F18")).toBe("⌃⌥⇧ F18");
		expect(formatShortcut("Alt+ArrowUp")).toBe("⌥↑");
		expect(formatShortcut("Cmd+Digit4")).toBe("⌘4");
	});

	test("reads the spellings Rust also accepts", () => {
		expect(formatShortcut("Control+Option+KeyT")).toBe("⌃⌥T");
	});
});
