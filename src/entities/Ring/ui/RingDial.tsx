// 링 하나를 그린다 — 링 창과 설정의 편집기가 같이 쓴다. 받은 칸과 켜진 칸 번호를 그리기만 한다
import { Icon } from "@/shared/ui/Icon";
import type { RingSlotView } from "../domain/types";
import * as S from "./RingDial.styles";
import { RING_BOX, RING_HOLE_RADIUS, chevronPlace, donutPath, highlightPath, slotCenter, slotPath } from "./ringShape";

interface RingDialProps {
	slotCount: number;
	slots: RingSlotView[];
	/** 켜진 칸. 없으면 null. */
	hovered: number | null;
	/** 그릴 크기(한 변). 기본은 원래 크기 272. */
	size?: number;
	/** 가운데에 뒤로 아이콘을 보인다 (하위 링). */
	showBack?: boolean;
	/** 칸 이름을 보인다. 작은 그림에서는 끈다. */
	showLabels?: boolean;
	/** 빈 칸에 자리 표시를 보인다 (설정의 편집기). */
	showEmpty?: boolean;
	/** 링 모양을 따라 그림자를 그린다. */
	shadow?: boolean;
	/** 주면 칸을 누를 수 있다 (설정의 편집기). 빈 칸도 눌린다. */
	onSlotClick?: (position: number) => void;
}

export function RingDial({
	slotCount,
	slots,
	hovered,
	size = RING_BOX,
	showBack = false,
	showLabels = true,
	showEmpty = false,
	shadow = false,
	onSlotClick
}: RingDialProps) {
	const center = RING_BOX / 2;
	const positions = Array.from({ length: slotCount }, (_, position) => position);
	const filled = new Set(slots.map((slot) => slot.position));
	return (
		<S.Box $size={size}>
			<S.Scaled $scale={size / RING_BOX}>
				{shadow && <S.Shadow />}
				<S.Base viewBox={`0 0 ${RING_BOX} ${RING_BOX}`}>
					<S.Surface fillRule="evenodd" d={donutPath()} />
					{showBack && <S.Hub cx={center} cy={center} r={RING_HOLE_RADIUS - 6} />}
					{onSlotClick &&
						positions.map((position) => (
							<S.HitArea
								key={position}
								data-slot={position}
								d={slotPath(position, slotCount)}
								onClick={() => onSlotClick(position)}
							/>
						))}
					{hovered !== null && hovered < slotCount && (
						<S.Highlight d={highlightPath(hovered, slotCount)} pointerEvents="none" />
					)}
				</S.Base>
				{showEmpty &&
					positions
						.filter((position) => !filled.has(position))
						.map((position) => (
							<S.EmptyMark
								key={position}
								$on={position === hovered}
								style={slotCenter(position, slotCount)}
							>
								<Icon name="plus" size={20} strokeWidth={1.6} />
							</S.EmptyMark>
						))}
				{slots
					.filter((slot) => slot.position < slotCount)
					.map((slot) => {
						const on = slot.position === hovered;
						const chevron = chevronPlace(slot.position, slotCount);
						return (
							<div key={slot.position}>
								<S.Slot $on={on} style={slotCenter(slot.position, slotCount)}>
									<Icon name={slot.icon} size={20} strokeWidth={1.8} />
									{showLabels && <S.SlotLabel $on={on}>{slot.label}</S.SlotLabel>}
								</S.Slot>
								{slot.opensRing && (
									<S.Chevron
										$on={on}
										style={{
											left: chevron.left,
											top: chevron.top,
											transform: `rotate(${chevron.rotate}deg)`
										}}
									>
										<Icon name="chevron-right" size={12} strokeWidth={2.4} />
									</S.Chevron>
								)}
							</div>
						);
					})}
				{showBack && (
					<S.HubIcon>
						<Icon name="corner-up-left" size={16} />
					</S.HubIcon>
				)}
			</S.Scaled>
		</S.Box>
	);
}
