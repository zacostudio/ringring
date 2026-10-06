// 링을 그리는 좌표 계산 — 도넛, 켜진 칸의 부채꼴, 칸의 자리. 어느 칸이 켜질지는 Rust 가 정한다

/** 링의 바깥 반지름. Rust 의 `geometry::OUTER_RADIUS` 와 같다. */
export const RING_OUTER_RADIUS = 128;
/** 가운데 구멍의 반지름. Rust 의 `geometry::DEAD_ZONE_RADIUS` 와 같다. */
export const RING_HOLE_RADIUS = 30;
/** 링을 그리는 상자의 한 변. 가장자리 선이 잘리지 않게 반지름보다 8 크다. */
export const RING_BOX = 272;
const CENTER = RING_BOX / 2;

function point(radius: number, degrees: number): [number, number] {
	const radians = (degrees * Math.PI) / 180;
	return [CENTER + radius * Math.cos(radians), CENTER + radius * Math.sin(radians)];
}

/** `position` 칸의 중심 각도. 0번 칸이 12시(-90도)이고 시계 방향으로 간다. */
function slotAngle(position: number, slotCount: number): number {
	return -90 + (position * 360) / slotCount;
}

function sector(from: number, to: number, inner: number, outer: number): string {
	const [x0, y0] = point(outer, from);
	const [x1, y1] = point(outer, to);
	const [x2, y2] = point(inner, to);
	const [x3, y3] = point(inner, from);
	const large = to - from > 180 ? 1 : 0;
	return [
		"M",
		x0,
		y0,
		"A",
		outer,
		outer,
		0,
		large,
		1,
		x1,
		y1,
		"L",
		x2,
		y2,
		"A",
		inner,
		inner,
		0,
		large,
		0,
		x3,
		y3,
		"Z"
	].join(" ");
}

/** 가운데가 뚫린 링 전체. `fill-rule: evenodd` 로 그린다. */
export function donutPath(): string {
	const outer = RING_OUTER_RADIUS;
	const hole = RING_HOLE_RADIUS;
	return [
		"M",
		CENTER - outer,
		CENTER,
		"a",
		outer,
		outer,
		0,
		1,
		0,
		2 * outer,
		0,
		"a",
		outer,
		outer,
		0,
		1,
		0,
		-2 * outer,
		0,
		"Z",
		"M",
		CENTER - hole,
		CENTER,
		"a",
		hole,
		hole,
		0,
		1,
		1,
		2 * hole,
		0,
		"a",
		hole,
		hole,
		0,
		1,
		1,
		-2 * hole,
		0,
		"Z"
	].join(" ");
}

/** 켜진 칸의 부채꼴. 링 가장자리와 옆 칸에서 조금 들어와 있다. */
export function highlightPath(position: number, slotCount: number): string {
	const step = 360 / slotCount;
	const middle = slotAngle(position, slotCount);
	const pad = (3.2 * 6) / slotCount + 1.4;
	return sector(middle - step / 2 + pad, middle + step / 2 - pad, RING_HOLE_RADIUS + 9, RING_OUTER_RADIUS - 8);
}

/** 칸 하나가 차지하는 부채꼴 전체. 설정의 미리보기에서 누르는 영역이다. */
export function slotPath(position: number, slotCount: number): string {
	const step = 360 / slotCount;
	const middle = slotAngle(position, slotCount);
	return sector(middle - step / 2, middle + step / 2, RING_HOLE_RADIUS, RING_OUTER_RADIUS);
}

/** 칸의 아이콘과 이름이 놓일 중심. 칸이 많으면 조금 더 바깥에 둔다. */
export function slotCenter(position: number, slotCount: number): { left: number; top: number } {
	const [left, top] = point(slotCount >= 7 ? 86 : 80, slotAngle(position, slotCount));
	return { left, top };
}

/** 하위 링을 여는 칸의 chevron 자리와 방향. */
export function chevronPlace(position: number, slotCount: number): { left: number; top: number; rotate: number } {
	const angle = slotAngle(position, slotCount);
	const [left, top] = point(RING_OUTER_RADIUS - 13, angle);
	return { left, top, rotate: angle };
}
