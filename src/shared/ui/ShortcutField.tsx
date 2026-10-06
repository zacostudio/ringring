// 키 조합을 입력받는 칸 — 누르면 다음에 누른 조합을 받는다. 쓸 수 있는 조합인지는 Rust 가 정한다
import styled from "@emotion/styled";
import { useEffect, useRef, useState } from "react";
import { chordToShortcut, formatShortcut, isModifierOnly } from "../lib/shortcut";
import { Icon } from "./Icon";

interface ShortcutFieldProps {
	/** 지금 조합. 없으면 null. */
	value: string | null;
	onChange: (shortcut: string) => void;
	/** 주면 값이 있을 때 지우는 단추가 보인다. */
	onClear?: () => void;
	/**
	 * `global` 은 전역 단축키다 — Esc 는 입력 취소다.
	 * `send` 는 다른 앱에 보낼 키다 — Esc 도 받을 키다.
	 */
	mode: "global" | "send";
	/**
	 * 입력받기 시작하고 끝날 때 부른다. 부른 쪽이 그동안 전역 단축키를 뗀다.
	 * `id` 는 그 입력의 번호다. 시작과 끝이 같은 번호를 갖고, 다음 입력은 더 큰 번호를 갖는다.
	 */
	onCapture?: (active: boolean, id: number) => void;
	texts: { empty: string; capturing: string; clear: string };
}

// 입력의 번호. 시각에서 시작해 늘기만 한다 — 페이지를 다시 읽어도 앞의 번호보다 크다.
// 시작과 끝이 Rust 에 뒤바뀌어 도착해도 Rust 가 이 번호로 바로잡는다.
let lastCaptureId = 0;
function nextCaptureId(): number {
	lastCaptureId = Math.max(Date.now(), lastCaptureId + 1);
	return lastCaptureId;
}

export function ShortcutField({ value, onChange, onClear, mode, onCapture, texts }: ShortcutFieldProps) {
	const [capturing, setCapturing] = useState(false);
	// 최신 callback 을 읽는다. 입력받는 동안 부모가 다시 그려도 listener 를 다시 걸지 않는다.
	const latest = useRef({ onChange, onCapture, mode });
	latest.current = { onChange, onCapture, mode };

	useEffect(() => {
		if (!capturing) return;
		const id = nextCaptureId();
		latest.current.onCapture?.(true, id);
		const stop = () => setCapturing(false);
		const onKeyDown = (event: KeyboardEvent) => {
			event.preventDefault();
			event.stopPropagation();
			if (event.code === "Escape" && latest.current.mode === "global") {
				stop();
				return;
			}
			// 수식키만 눌렸다. 조합이 아직 끝나지 않았다.
			if (isModifierOnly(event)) return;
			const shortcut = chordToShortcut(event);
			stop();
			if (shortcut) latest.current.onChange(shortcut);
		};
		window.addEventListener("keydown", onKeyDown, true);
		window.addEventListener("blur", stop);
		return () => {
			window.removeEventListener("keydown", onKeyDown, true);
			window.removeEventListener("blur", stop);
			latest.current.onCapture?.(false, id);
		};
	}, [capturing]);

	return (
		<Root>
			<Capture
				type="button"
				data-role="shortcut-capture"
				$capturing={capturing}
				$empty={!value}
				onClick={() => setCapturing(!capturing)}
				onBlur={() => setCapturing(false)}
			>
				{capturing ? texts.capturing : value ? formatShortcut(value) : texts.empty}
			</Capture>
			{onClear && value && !capturing && (
				<Clear type="button" aria-label={texts.clear} title={texts.clear} onClick={onClear}>
					<Icon name="x" size={14} />
				</Clear>
			)}
		</Root>
	);
}

const Root = styled.div`
    position: relative;
    display: inline-flex;
    align-items: center;
`;

const Capture = styled.button<{ $capturing: boolean; $empty: boolean }>`
    height: var(--control-h);
    min-width: 132px;
    padding: 0 29px 0 11px;
    border: none;
    border-radius: var(--radius);
    background: ${(p) => (p.$capturing ? "color-mix(in srgb, var(--accent) 14%, transparent 86%)" : "var(--bg-field)")};
    font-size: var(--text-body);
    font-weight: ${(p) => (p.$empty || p.$capturing ? 400 : 600)};
    letter-spacing: ${(p) => (p.$empty || p.$capturing ? "normal" : "0.08em")};
    text-align: left;
    white-space: nowrap;
    color: ${(p) => (p.$capturing ? "var(--accent)" : p.$empty ? "var(--text-3)" : "var(--text)")};

    &:focus-visible {
        outline: 2px solid var(--accent);
        outline-offset: 1px;
    }
`;

const Clear = styled.button`
    position: absolute;
    right: 4px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--text-3);

    & > svg {
        flex-shrink: 0;
    }

    &:hover {
        background: var(--bg-selected);
        color: var(--text);
    }
`;
