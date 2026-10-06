// Rust 의 `CommandError` 를 읽는다 — 거절은 이유 코드를 싣고 온다

/** Rust 의 `error.rs` 가 보내는 모양. */
export interface CommandError {
	kind: "refused" | "incomplete" | "failed";
	/** `Refusal::code`. 거절이 아니면 없다. */
	code: string | null;
	/** 영어 원문. 화면에 그대로 보이지 않는다. */
	message: string;
	/** 문장에 끼워 넣을 이름. */
	subject: string | null;
}

/** 잡은 값이 `CommandError` 면 그 값, 아니면 null. */
export function asCommandError(error: unknown): CommandError | null {
	if (typeof error !== "object" || error === null) return null;
	const candidate = error as Record<string, unknown>;
	if (candidate.kind !== "refused" && candidate.kind !== "incomplete" && candidate.kind !== "failed") return null;
	if (typeof candidate.message !== "string") return null;
	return {
		kind: candidate.kind,
		code: typeof candidate.code === "string" ? candidate.code : null,
		message: candidate.message,
		subject: typeof candidate.subject === "string" ? candidate.subject : null
	};
}
