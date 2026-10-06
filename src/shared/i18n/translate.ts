// 번역의 순수 부분 — 사전에서 문장을 찾고 `{이름}` 자리를 채운다
import { IS_MAC } from "../lib/shortcut";

export type Locale = "ko" | "en" | "ja";
export type Dictionary = Record<string, string>;
export type Translate = (key: string, vars?: Record<string, string | number>) => string;

/** macOS 밖에서 `key` 대신 보일 문장의 key. 그런 문장이 있을 때만 쓴다. */
export function platformKey(dictionary: Dictionary, key: string, mac = IS_MAC): string {
	const other = `other:${key}`;
	return !mac && other in dictionary ? other : key;
}

/** 사전에 없는 key 는 key 그대로 돌려준다. 화면에서 바로 눈에 띈다. */
export function translate(dictionary: Dictionary, key: string, vars?: Record<string, string | number>): string {
	const template = dictionary[key] ?? key;
	if (!vars) return template;
	return template.replace(/\{(\w+)\}/g, (whole, name: string) => (name in vars ? String(vars[name]) : whole));
}
