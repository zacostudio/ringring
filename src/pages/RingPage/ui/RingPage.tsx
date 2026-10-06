// 링 창의 페이지 — Rust 가 보낸 링과 켜진 칸을 그리고, 클릭과 키를 그대로 Rust 에 넘긴다
//
// 어느 칸이 켜지는지, 무엇을 실행하는지는 이 페이지가 정하지 않는다. 클릭은 좌표 없이 넘긴다 — Rust 가 커서
// 자리로 정한다.
import { useEffect, useRef, useState } from "react";
import { RingDial, ringRepository } from "@/entities/Ring";
import type { RingKey, RingShow } from "@/entities/Ring";
import { useT } from "@/shared/i18n";
import { EVENTS } from "@/shared/tauri/events";
import { subscribe } from "@/shared/tauri/ipc";
import { Icon } from "@/shared/ui/Icon";
import { ConfirmDetail } from "./ConfirmDetail";
import * as S from "./RingPage.styles";

/** 키보드의 키를 Rust 에 넘길 모양으로 바꾼다. 링이 듣지 않는 키는 null. */
function toRingKey(event: KeyboardEvent): RingKey | null {
	if (event.key >= "1" && event.key <= "8") return { kind: "digit", value: Number(event.key) };
	switch (event.key) {
		case "ArrowUp":
			return { kind: "arrow", direction: "up" };
		case "ArrowDown":
			return { kind: "arrow", direction: "down" };
		case "ArrowLeft":
			return { kind: "arrow", direction: "left" };
		case "ArrowRight":
			return { kind: "arrow", direction: "right" };
		case "Enter":
		case " ":
			return { kind: "enter" };
		case "Backspace":
			return { kind: "back" };
		case "Escape":
			return { kind: "escape" };
		default:
			return null;
	}
}

export function RingPage() {
	const t = useT();
	const [shown, setShown] = useState<RingShow | null>(null);
	const [hovered, setHovered] = useState<number | null>(null);
	// 늦게 도착한 예전 보이기가 새 링을 덮지 않게 한다.
	const lastSeq = useRef(0);
	// 확인 물음이 떠 있는가. 키 listener 가 읽는다.
	const confirming = useRef(false);
	confirming.current = shown?.mode === "confirm";

	useEffect(() => {
		let disposed = false;

		const apply = (next: RingShow | null) => {
			if (disposed) return;
			if (next === null) {
				setShown(null);
				setHovered(null);
				return;
			}
			if (next.seq < lastSeq.current) return;
			lastSeq.current = next.seq;
			setShown(next);
			setHovered(next.hovered);
			// 그린 뒤에 알린다. Rust 가 띄운 뒤 그릴 때까지 걸린 시간을 로그에 남긴다.
			requestAnimationFrame(() => {
				void ringRepository.painted(next.seq).catch(() => {});
			});
		};

		const unlisten = [
			subscribe<RingShow>(EVENTS.ringShow, apply, true),
			subscribe<number | null>(
				EVENTS.ringHover,
				(position) => {
					if (!disposed) setHovered(position);
				},
				true
			),
			subscribe<void>(EVENTS.ringHide, () => apply(null), true)
		];
		// listener 를 걸기 전에 `ring-show` 가 나갔을 수 있다. 지금 상태를 한 번 묻는다.
		void ringRepository.current().then(apply, (error) => {
			console.error("[ring] failed to read the current ring:", error);
		});

		return () => {
			disposed = true;
			for (const off of unlisten) off();
		};
	}, []);

	useEffect(() => {
		const onKeyDown = (event: KeyboardEvent) => {
			const key = toRingKey(event);
			if (!key) return;
			// 확인 물음에서 Enter 와 Space 는 focus 가 있는 버튼의 것이다. 가로채면 "취소" 에 focus 를
			// 두고 Enter 를 눌러도 실행된다. 버튼의 click 이 답을 보낸다.
			if (confirming.current && key.kind === "enter") return;
			event.preventDefault();
			void ringRepository.key(key).catch(() => {});
		};
		window.addEventListener("keydown", onKeyDown);
		return () => window.removeEventListener("keydown", onKeyDown);
	}, []);

	if (!shown) return <S.Root />;

	if (shown.mode === "confirm" && shown.confirm) {
		return (
			<S.Root>
				<S.Stage key={`confirm-${shown.seq}`}>
					<S.Confirm>
						<S.ConfirmTitle>
							<Icon name={shown.confirm.icon} size={16} />
							<span>{shown.confirm.label}</span>
						</S.ConfirmTitle>
						<ConfirmDetail text={shown.confirm.detail} />
						<S.ConfirmActions>
							{/* 처음 focus 는 "취소" 다. 물음이 뜨자마자 누른 Enter 가 실행이 되지 않는다. */}
							<S.ConfirmButton
								type="button"
								data-role="cancel"
								autoFocus
								onClick={() => void ringRepository.hide().catch(() => {})}
							>
								{t("common.cancel")}
							</S.ConfirmButton>
							<S.ConfirmButton
								type="button"
								data-role="run"
								$primary
								onClick={() => void ringRepository.confirm(true).catch(() => {})}
							>
								{t("ring.run")}
							</S.ConfirmButton>
						</S.ConfirmActions>
					</S.Confirm>
				</S.Stage>
			</S.Root>
		);
	}

	return (
		<S.Root onMouseDown={() => void ringRepository.pick().catch(() => {})}>
			{/* 링이 바뀔 때(하위 링)만 다시 나타난다. 칸의 강조가 바뀔 때는 움직이지 않는다. */}
			<S.Stage key={`${shown.ring.id}:${shown.depth}`}>
				<RingDial
					slotCount={shown.ring.slotCount}
					slots={shown.ring.slots}
					hovered={hovered}
					showBack={shown.depth > 1}
					shadow
				/>
			</S.Stage>
		</S.Root>
	);
}
