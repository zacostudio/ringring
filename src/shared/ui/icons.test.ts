// 코드에 적힌 아이콘 이름이 설치된 lucide 에 실제로 있는지 본다 — 없는 이름은 화면에서 빈 자리가 된다
import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { SUGGESTED_ICONS } from "../../widgets/RingEditor/lib/suggestedIcons";
import { iconExists, toPascalCase } from "./Icon";

const ROOT = path.resolve(import.meta.dir, "../../..");

function files(dir: string, pattern: RegExp): string[] {
	return readdirSync(dir).flatMap((name) => {
		const full = path.join(dir, name);
		if (statSync(full).isDirectory()) return files(full, pattern);
		return pattern.test(name) && !name.endsWith(".test.ts") ? [full] : [];
	});
}

/** 소스에서 아이콘 이름으로 쓰인 글자들. */
function iconNames(text: string): string[] {
	const patterns = [
		/\bicon[:=]\s*"([a-z0-9-]+)"/g,
		/<Icon name="([a-z0-9-]+)"/g,
		/<Icon name=\{[^}]*?"([a-z0-9-]+)"\s*:\s*"([a-z0-9-]+)"/g,
		/_ICON: &str = "([a-z0-9-]+)"/g,
		/Icon: Record<[^>]+> = \{([^}]+)\}/g
	];
	const names: string[] = [];
	for (const pattern of patterns) {
		for (const match of text.matchAll(pattern)) {
			for (const group of match.slice(1)) {
				if (!group) continue;
				// `{ app: "app-window", … }` 처럼 묶음으로 잡힌 것은 안의 값들을 꺼낸다.
				if (group.includes(":")) names.push(...[...group.matchAll(/"([a-z0-9-]+)"/g)].map((m) => m[1]));
				else names.push(group);
			}
		}
	}
	return names;
}

describe("icons", () => {
	test("kebab-case names map to lucide's export names", () => {
		expect(toPascalCase("chevron-right")).toBe("ChevronRight");
		expect(toPascalCase("arrow-down-up")).toBe("ArrowDownUp");
	});

	test("every suggested icon exists", () => {
		expect(SUGGESTED_ICONS.filter((name) => !iconExists(name))).toEqual([]);
		expect(new Set(SUGGESTED_ICONS).size).toBe(SUGGESTED_ICONS.length);
	});

	test("every icon name written in the frontend and in Rust exists", () => {
		const sources = [
			...files(path.join(ROOT, "src"), /\.tsx?$/),
			...files(path.join(ROOT, "src-tauri/src"), /\.rs$/)
		];
		const used = new Set(sources.flatMap((file) => iconNames(readFileSync(file, "utf8"))));
		expect(used.size).toBeGreaterThan(20);
		expect([...used].filter((name) => !iconExists(name)).sort()).toEqual([]);
	});
});
