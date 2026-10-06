// 단축키 조합의 글 — 키 이벤트를 저장할 글로 만들고, 저장된 글을 화면의 기호로 바꾼다
//
// 쓸 수 있는 조합인지는 Rust 가 정한다 (`shortcuts.rs` 의 `parse_for_registration`). 여기는 글을 만들 뿐이다.

/**
 * Rust 의 조합 파서가 받는 키 토큰 (global-hotkey 의 `parse_key`). `KeyboardEvent.code` 값과 같다.
 * 여기 없는 code(IME 키, ContextMenu, Fn)는 저장해도 읽히지 않으므로 받지 않는다.
 */
const ACCEPTED_CODE =
	/^(?:Key[A-Z]|Digit\d|F([1-9]|1\d|2[0-4])|Numpad(\d|Add|Decimal|Divide|Enter|Equal|Multiply|Subtract)|Arrow(Up|Down|Left|Right)|Backquote|Backslash|BracketLeft|BracketRight|Comma|Equal|Minus|Period|Quote|Semicolon|Slash|Backspace|CapsLock|Enter|Space|Tab|Delete|End|Home|Insert|PageDown|PageUp|PrintScreen|ScrollLock|NumLock|Escape|Pause)$/;

const MODIFIER_CODE = /^(?:Shift|Control|Alt|Meta)(?:Left|Right)$/;

/** code → 화면에 보일 짧은 표기. 없으면 code 를 그대로 쓴다 (F5, Home 등). */
const CODE_SYMBOL: Record<string, string> = {
	Backquote: "`",
	Backslash: "\\",
	BracketLeft: "[",
	BracketRight: "]",
	Comma: ",",
	Equal: "=",
	Minus: "-",
	Period: ".",
	Quote: "'",
	Semicolon: ";",
	Slash: "/",
	ArrowUp: "↑",
	ArrowDown: "↓",
	ArrowLeft: "←",
	ArrowRight: "→",
	Enter: "↩",
	Tab: "⇥",
	Space: "Space",
	Backspace: "⌫",
	Delete: "⌦",
	Escape: "⎋"
};

const MODIFIER_SYMBOL: Record<string, string> = {
	CmdOrCtrl: "⌘",
	Cmd: "⌘",
	Command: "⌘",
	Super: "⌘",
	Meta: "⌘",
	Ctrl: "⌃",
	Control: "⌃",
	Alt: "⌥",
	Option: "⌥",
	Shift: "⇧"
};

/** 키 이벤트에서 읽는 것. `KeyboardEvent` 가 이 모양이다. */
export interface KeyChord {
	code: string;
	metaKey: boolean;
	ctrlKey: boolean;
	altKey: boolean;
	shiftKey: boolean;
}

/** 수식키만 눌린 상태인가. 조합이 아직 끝나지 않았다. */
export function isModifierOnly(chord: KeyChord): boolean {
	return MODIFIER_CODE.test(chord.code);
}

/**
 * 키 이벤트를 저장할 글로 만든다 (`CmdOrCtrl+Shift+KeyG`). 받을 수 없는 키면 null.
 *
 * `key` 가 아니라 `code` 로 만든다. macOS 에서 ⌥ 는 찍히는 글자를 바꾼다 — ⌥⇧G 의 `key` 는 `˝` 다.
 * `code` 는 물리 키라 수식키와 자판 배열에 영향받지 않는다.
 */
export function chordToShortcut(chord: KeyChord): string | null {
	if (!ACCEPTED_CODE.test(chord.code)) return null;
	const parts: string[] = [];
	if (chord.metaKey) parts.push("CmdOrCtrl");
	if (chord.ctrlKey) parts.push("Ctrl");
	if (chord.altKey) parts.push("Alt");
	if (chord.shiftKey) parts.push("Shift");
	parts.push(chord.code);
	return parts.join("+");
}

function formatKey(token: string): string {
	if (CODE_SYMBOL[token]) return CODE_SYMBOL[token];
	const letter = /^Key([A-Z])$/.exec(token);
	if (letter) return letter[1];
	const digit = /^Digit(\d)$/.exec(token);
	if (digit) return digit[1];
	return token;
}

/** 저장된 조합을 화면에 보일 조각들로 바꾼다. `CmdOrCtrl+Shift+KeyG` → `["⌘", "⇧", "G"]`. */
export function shortcutParts(shortcut: string): string[] {
	return shortcut
		.split("+")
		.filter(Boolean)
		.map((token) => MODIFIER_SYMBOL[token] ?? formatKey(token));
}

/** 저장된 조합을 한 줄로. `⌘⇧G`. 수식키는 붙여 쓰고, 이름이 긴 키만 띄운다. */
export function formatShortcut(shortcut: string): string {
	const parts = shortcutParts(shortcut);
	return parts.map((part, index) => (part.length > 1 && index > 0 ? ` ${part}` : part)).join("");
}
