// 화면의 언어 — 지금 언어를 들고, 컴포넌트에 번역 함수를 준다 (React Context 없이)
import { useCallback, useSyncExternalStore } from "react";
import { createStore } from "../lib/externalStore";
import { en } from "./en";
import { ja } from "./ja";
import { ko } from "./ko";
import { type Dictionary, type Locale, type Translate, translate } from "./translate";

export type { Locale, Translate } from "./translate";

export const DICTIONARIES: Record<Locale, Dictionary> = { ko, en, ja };

/** Rust 가 언어를 알려 주기 전의 값. 브라우저의 언어로 짐작한다. */
function guessLocale(): Locale {
	const tag = (typeof navigator === "undefined" ? "" : navigator.language).toLowerCase();
	if (tag.startsWith("ko")) return "ko";
	if (tag.startsWith("ja")) return "ja";
	return "en";
}

const store = createStore<Locale>(guessLocale());

/** 실제로 쓸 언어는 Rust 가 정한다 (`settings.rs` 의 `locale`). 여기는 받은 값을 넣는다. */
export function setLocale(locale: Locale) {
	store.set(locale);
}

export function useLocale(): Locale {
	return useSyncExternalStore(store.subscribe, store.get);
}

/** 지금 언어의 번역 함수. 언어가 바뀌면 다시 그린다. */
export function useT(): Translate {
	const locale = useLocale();
	return useCallback((key, vars) => translate(DICTIONARIES[locale], key, vars), [locale]);
}
