// Rust 가 거절하며 보낸 이유 코드를 사용자 언어의 문장으로 옮긴다
//
// 거절할지와 그 이유는 Rust 가 정했다 (`ring/model.rs` 의 `Refusal`). 여기는 그 코드에
// 맞는 문장을 고를 뿐이다. 코드를 늘리면 두 파일과 번역 세 벌을 같이 연다.
import { asCommandError } from "../../../shared/lib/commandError";
import type { Translate } from "../../../shared/i18n/translate";
import type { RingLimits } from "../domain/types";

/** Rust 의 `Refusal::code` 가 내려보내는 값 전부. */
export const RING_REFUSAL_CODES = [
	"open_value_missing",
	"url_not_http",
	"command_missing",
	"timeout_out_of_range",
	"combos_missing",
	"combos_too_many",
	"combo_invalid",
	"ring_not_chosen",
	"slot_outside_ring",
	"ring_name_missing",
	"ring_name_too_long",
	"slot_name_missing",
	"slot_name_too_long",
	"icon_missing",
	"slot_count_out_of_range",
	"link_self",
	"link_cycle",
	"link_too_deep",
	"ring_gone",
	"ring_empty",
	"order_not_permutation",
	"shortcut_invalid",
	"shortcut_needs_modifier",
	"shortcut_taken",
	"shortcut_unavailable",
	"import_unreadable",
	"import_version_unsupported",
	"import_too_large",
	"import_empty"
] as const;

export type RingRefusalCode = (typeof RING_REFUSAL_CODES)[number];

/** 화면에 보일 거절 한 줄. */
export interface RingNotice {
	text: string;
	/** 틀린 값이 아니라 아직 채우지 않은 값이다. 오류 색으로 보이지 않는다. */
	incomplete: boolean;
}

function isRefusalCode(code: string | null): code is RingRefusalCode {
	return code !== null && (RING_REFUSAL_CODES as readonly string[]).includes(code);
}

/** 문장에 끼워 넣을 값. 한도는 Rust 가 준 값이다. */
function refusalVars(
	code: RingRefusalCode,
	limits: RingLimits,
	subject: string | null
): Record<string, string | number> {
	switch (code) {
		case "timeout_out_of_range":
			return { max: limits.maxShellTimeoutSecs };
		case "combos_too_many":
			return { max: limits.maxCombos };
		case "ring_name_too_long":
		case "slot_name_too_long":
			return { max: limits.nameMaxChars };
		case "slot_count_out_of_range":
			return { min: limits.minSlots, max: limits.maxSlots };
		case "link_too_deep":
			return { max: limits.maxDepth };
		case "shortcut_taken":
			return { name: subject ?? "" };
		default:
			return {};
	}
}

/** 이유 코드의 문장. 모르는 코드면 `null`. */
export function ringRefusalText(
	code: string | null,
	limits: RingLimits,
	t: Translate,
	subject: string | null = null
): string | null {
	if (!isRefusalCode(code)) return null;
	return t(`refusal.${code}`, refusalVars(code, limits, subject));
}

/**
 * 잡은 값이 링의 거절이면 그 한 줄. 아니면 `null` — DB 오류 같은 것이고, 부른 쪽이 자기 문장을 쓴다.
 *
 * `error` 는 잡은 값을 그대로 넘긴다. `String(error)` 로 넘기면 `[object Object]` 가 된다.
 */
export function ringNotice(error: unknown, limits: RingLimits, t: Translate): RingNotice | null {
	const commandError = asCommandError(error);
	if (!commandError || commandError.kind === "failed") return null;
	const text = ringRefusalText(commandError.code, limits, t, commandError.subject);
	return text === null ? null : { text, incomplete: commandError.kind === "incomplete" };
}

/** 실패의 한 줄. 거절이면 그 이유고, 아니면 `fallbackKey` 의 문장이다. */
export function ringFailureText(error: unknown, limits: RingLimits, t: Translate, fallbackKey: string): string {
	return ringNotice(error, limits, t)?.text ?? t(fallbackKey);
}
