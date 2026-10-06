// 세 언어의 사전이 같은 key 를 갖는지, 코드가 쓰는 key 가 사전에 있는지 본다
import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { RING_REFUSAL_CODES } from "../../entities/Ring/lib/refusal";
import { en } from "./en";
import { ja } from "./ja";
import { ko } from "./ko";
import { translate } from "./translate";

const SRC = path.resolve(import.meta.dir, "../..");

/** `src/` 아래의 소스 파일 전부 (테스트와 사전은 뺀다). */
function sourceFiles(dir: string): string[] {
	return readdirSync(dir).flatMap((name) => {
		const full = path.join(dir, name);
		if (statSync(full).isDirectory()) return sourceFiles(full);
		if (!/\.tsx?$/.test(name) || name.endsWith(".test.ts") || dir.endsWith("i18n")) return [];
		return [full];
	});
}

/** 한 문장에 든 `{이름}` 자리들. */
function placeholders(text: string): string[] {
	return [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort();
}

describe("dictionaries", () => {
	test("the three languages have exactly the same keys", () => {
		const keys = Object.keys(ko).sort();
		expect(Object.keys(en).sort()).toEqual(keys);
		expect(Object.keys(ja).sort()).toEqual(keys);
	});

	test("no sentence is empty", () => {
		for (const dictionary of [ko, en, ja]) {
			for (const [key, text] of Object.entries(dictionary)) expect(text.trim(), key).not.toBe("");
		}
	});

	test("a sentence uses the same placeholders in every language", () => {
		for (const key of Object.keys(ko)) {
			expect(placeholders(en[key]), key).toEqual(placeholders(ko[key]));
			expect(placeholders(ja[key]), key).toEqual(placeholders(ko[key]));
		}
	});

	test("every refusal code Rust can send has a sentence", () => {
		for (const code of RING_REFUSAL_CODES) expect(ko[`refusal.${code}`], code).toBeDefined();
	});

	test("every refusal sentence belongs to a code", () => {
		const known = new Set<string>(RING_REFUSAL_CODES);
		for (const key of Object.keys(ko).filter((candidate) => candidate.startsWith("refusal."))) {
			expect(known.has(key.slice("refusal.".length)), key).toBe(true);
		}
	});

	test("every key written in the source exists in the dictionary", () => {
		const used = new Set<string>();
		for (const file of sourceFiles(SRC)) {
			const text = readFileSync(file, "utf8");
			for (const match of text.matchAll(/\bt\(\s*"([\w.]+)"/g)) used.add(match[1]);
			for (const match of text.matchAll(/(?:labelKey|Key): "([\w.]+)"/g)) used.add(match[1]);
			// `t(조건 ? "a" : "b")` 처럼 고르는 key.
			for (const match of text.matchAll(/[?:]\s*"((?:[a-zA-Z]\w*\.)+[a-zA-Z]\w*)"/g)) used.add(match[1]);
		}
		const missing = [...used].filter((key) => !(key in ko)).sort();
		expect(missing).toEqual([]);
	});
});

describe("translate", () => {
	test("fills placeholders and leaves unknown ones as written", () => {
		expect(translate(ko, "slot.title", { n: 3 })).toBe("3번 칸");
		expect(translate(en, "ring.deleteBlockedBy", { ring: "Work", n: 2 })).toBe("Work · slot 2");
		expect(translate(en, "slot.title")).toBe("Slot {n}");
	});

	test("an unknown key comes back as the key", () => {
		expect(translate(ko, "no.such.key")).toBe("no.such.key");
	});
});
