// 링의 판정 계산 — 커서가 어느 칸에 있는지, 링을 화면 어디에 둘지 (순수 함수)
//
// 길이는 모두 논리 픽셀이다. 좌표는 화면 좌표라 y 가 아래로 자란다.

/// 링의 바깥 반지름.
pub const OUTER_RADIUS: f64 = 128.0;
/// 가운데 구멍의 반지름. 이 안은 dead zone 이다 — 어느 칸도 켜지지 않는다.
pub const DEAD_ZONE_RADIUS: f64 = 30.0;
/// 링 창의 한 변. 링 지름 256 에 그림자 자리 32 를 양쪽에 둔다.
pub const WINDOW_SIZE: f64 = 320.0;
/// 링과 화면 가장자리 사이의 여백.
pub const EDGE_MARGIN: f64 = 8.0;

/// 한 링의 칸 수 범위.
pub const MIN_SLOTS: usize = 4;
pub const MAX_SLOTS: usize = 8;

/// 커서가 링의 어디에 있는가.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
	/// 커서 방향의 칸. dead zone 안이면 `None`.
	pub slot: Option<usize>,
	/// 바깥 반지름을 넘었는가. hold 에서 하위 링을 여는 조건이다.
	pub beyond_outer: bool,
}

/// 링 중심에서 본 커서의 자리(`dx`, `dy`)로 칸을 정한다.
///
/// 1번 칸(번호 0)의 중심이 12시 방향이고 번호는 시계 방향으로 간다. 거리는 dead zone 을 넘었는지만 본다 —
/// 링 밖으로 멀리 나가도 그 방향의 칸이다.
pub fn hit(dx: f64, dy: f64, slot_count: usize) -> Hit {
	let distance = dx.hypot(dy);
	if slot_count == 0 || distance <= DEAD_ZONE_RADIUS {
		return Hit {
			slot: None,
			beyond_outer: false,
		};
	}
	// 12시가 0 이고 시계 방향으로 자라는 각도. 화면은 y 가 아래로 자라므로 위쪽은 `-dy` 다.
	let angle = dx.atan2(-dy).to_degrees().rem_euclid(360.0);
	let width = 360.0 / slot_count as f64;
	let slot = (((angle + width / 2.0).rem_euclid(360.0)) / width).floor() as usize;
	Hit {
		// 부동소수점 끝값(360 에 닿은 각도)이 칸 수와 같은 번호를 내지 않게 한다.
		slot: Some(slot.min(slot_count - 1)),
		beyond_outer: distance > OUTER_RADIUS,
	}
}

/// 화면의 한 영역. 논리 픽셀이다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
	pub x: f64,
	pub y: f64,
	pub width: f64,
	pub height: f64,
}

/// 링의 중심을 `area` 안으로 밀어 넣는다. 링 전체(반지름 [`OUTER_RADIUS`])가 여백 [`EDGE_MARGIN`] 을 두고
/// 안에 들어온다. 영역이 링보다 작으면 영역의 가운데에 둔다.
///
/// **위쪽만 더 들어온다.** macOS 는 창을 메뉴 막대 위로 올려 주지 않는다. 창의 위쪽 32 는 투명한 그림자
/// 자리인데, 그 자리도 영역 위로 나가지 못한다. 나가게 두면 창이 아래로 밀려 그림이 계산한 중심보다 24
/// 아래에 그려진다 (Tome 에서 실측: 중심 166 에 창 위쪽이 6 이어야 하는데 30 에 놓였다). 그래서 위쪽은 창의 반
/// ([`WINDOW_SIZE`] / 2)만큼 들어온다. 왼쪽·오른쪽·아래는 창이 화면 밖으로 나가도 된다.
pub fn clamp_center(x: f64, y: f64, area: Rect) -> (f64, f64) {
	let inset = OUTER_RADIUS + EDGE_MARGIN;
	(
		clamp_axis(x, area.x, area.width, inset, inset),
		clamp_axis(y, area.y, area.height, WINDOW_SIZE / 2.0, inset),
	)
}

fn clamp_axis(value: f64, start: f64, length: f64, inset_start: f64, inset_end: f64) -> f64 {
	let (min, max) = (start + inset_start, start + length - inset_end);
	if min > max {
		return start + length / 2.0;
	}
	value.clamp(min, max)
}

/// 화살표 키로 켜진 칸을 옮긴다. `filled[i]` 는 i번 칸에 동작이 있는가다. 빈 칸은 건너뛴다.
///
/// `forward` 면 시계 방향이다. 켜진 칸이 없으면 시계 방향은 첫 칸부터, 반대 방향은 끝 칸부터 찾는다.
/// 채운 칸이 하나도 없으면 `None`.
pub fn step(current: Option<usize>, forward: bool, filled: &[bool]) -> Option<usize> {
	let count = filled.len();
	if count == 0 {
		return None;
	}
	let start = match (current, forward) {
		(Some(index), true) => (index + 1) % count,
		(Some(index), false) => (index + count - 1) % count,
		(None, true) => 0,
		(None, false) => count - 1,
	};
	(0..count)
		.map(|offset| {
			if forward {
				(start + offset) % count
			} else {
				(start + count - offset) % count
			}
		})
		.find(|&index| filled[index])
}

#[cfg(test)]
mod tests {
	use super::*;

	/// 12시가 0 이고 시계 방향인 각도와 거리로 `(dx, dy)` 를 만든다.
	fn at(angle_deg: f64, distance: f64) -> (f64, f64) {
		let rad = angle_deg.to_radians();
		(distance * rad.sin(), -distance * rad.cos())
	}

	fn slot_at(angle_deg: f64, slot_count: usize) -> Option<usize> {
		let (dx, dy) = at(angle_deg, 80.0);
		hit(dx, dy, slot_count).slot
	}

	#[test]
	fn first_slot_is_centered_on_twelve_oclock() {
		for count in MIN_SLOTS..=MAX_SLOTS {
			assert_eq!(slot_at(0.0, count), Some(0), "{count} slots");
			assert_eq!(
				hit(0.0, -80.0, count).slot,
				Some(0),
				"{count} slots, straight up"
			);
		}
	}

	#[test]
	fn slots_run_clockwise() {
		// 4칸: 위, 오른쪽, 아래, 왼쪽.
		assert_eq!(hit(80.0, 0.0, 4).slot, Some(1));
		assert_eq!(hit(0.0, 80.0, 4).slot, Some(2));
		assert_eq!(hit(-80.0, 0.0, 4).slot, Some(3));
	}

	#[test]
	fn every_slot_center_maps_to_its_own_index() {
		for count in MIN_SLOTS..=MAX_SLOTS {
			let width = 360.0 / count as f64;
			for index in 0..count {
				assert_eq!(
					slot_at(index as f64 * width, count),
					Some(index),
					"{count} slots, slot {index}"
				);
			}
		}
	}

	#[test]
	fn boundaries_fall_half_a_slot_from_each_center() {
		for count in MIN_SLOTS..=MAX_SLOTS {
			let width = 360.0 / count as f64;
			for index in 0..count {
				let boundary = index as f64 * width + width / 2.0;
				assert_eq!(
					slot_at(boundary - 0.01, count),
					Some(index),
					"{count} slots, just before the boundary after slot {index}"
				);
				assert_eq!(
					slot_at(boundary + 0.01, count),
					Some((index + 1) % count),
					"{count} slots, just after the boundary after slot {index}"
				);
			}
		}
	}

	#[test]
	fn the_last_half_slot_before_twelve_wraps_to_the_first_slot() {
		for count in MIN_SLOTS..=MAX_SLOTS {
			assert_eq!(slot_at(359.99, count), Some(0), "{count} slots");
		}
	}

	#[test]
	fn dead_zone_selects_nothing() {
		for count in MIN_SLOTS..=MAX_SLOTS {
			assert_eq!(hit(0.0, 0.0, count).slot, None);
			// 축 위의 점이다. 비스듬한 점은 sin·cos 의 반올림으로 거리가 반지름을 조금 넘을 수 있다.
			let (dx, dy) = (DEAD_ZONE_RADIUS, 0.0);
			assert_eq!(hit(dx, dy, count).slot, None, "on the dead zone edge");
			let (dx, dy) = at(45.0, DEAD_ZONE_RADIUS + 0.5);
			assert!(
				hit(dx, dy, count).slot.is_some(),
				"just outside the dead zone"
			);
		}
	}

	#[test]
	fn distance_beyond_the_ring_keeps_the_direction() {
		let (dx, dy) = at(90.0, 2000.0);
		let far = hit(dx, dy, 6);
		assert_eq!(far.slot, slot_at(90.0, 6));
		assert!(far.beyond_outer);
	}

	#[test]
	fn beyond_outer_starts_past_the_outer_radius() {
		let (dx, dy) = at(10.0, OUTER_RADIUS);
		assert!(!hit(dx, dy, 6).beyond_outer);
		let (dx, dy) = at(10.0, OUTER_RADIUS + 0.5);
		assert!(hit(dx, dy, 6).beyond_outer);
		assert!(!hit(0.0, 0.0, 6).beyond_outer);
	}

	#[test]
	fn zero_slots_selects_nothing() {
		assert_eq!(hit(50.0, 50.0, 0).slot, None);
	}

	const SCREEN: Rect = Rect {
		x: 0.0,
		y: 25.0,
		width: 1440.0,
		height: 875.0,
	};

	#[test]
	fn a_center_well_inside_stays_put() {
		assert_eq!(clamp_center(700.0, 400.0, SCREEN), (700.0, 400.0));
	}

	#[test]
	fn all_four_corners_are_pushed_inside() {
		let inset = OUTER_RADIUS + EDGE_MARGIN;
		// 위쪽은 창의 반만큼 들어온다 — 창이 영역 위로 나가지 못한다.
		let top = 25.0 + WINDOW_SIZE / 2.0;
		assert_eq!(clamp_center(0.0, 25.0, SCREEN), (inset, top));
		assert_eq!(clamp_center(1440.0, 25.0, SCREEN), (1440.0 - inset, top));
		assert_eq!(clamp_center(0.0, 900.0, SCREEN), (inset, 900.0 - inset));
		assert_eq!(
			clamp_center(1440.0, 900.0, SCREEN),
			(1440.0 - inset, 900.0 - inset)
		);
	}

	#[test]
	fn the_window_never_starts_above_the_area() {
		// 어느 자리에서 눌러도 창의 위쪽(중심 - 창의 반)이 영역의 위쪽보다 위에 있지 않다.
		for y in [-500.0, 0.0, 25.0, 60.0, 184.0, 185.0, 400.0] {
			let (_, center_y) = clamp_center(700.0, y, SCREEN);
			assert!(center_y - WINDOW_SIZE / 2.0 >= SCREEN.y, "pressed at y={y}");
		}
	}

	#[test]
	fn a_monitor_left_of_the_primary_uses_its_own_origin() {
		let left = Rect {
			x: -1920.0,
			y: 0.0,
			width: 1920.0,
			height: 1080.0,
		};
		let inset = OUTER_RADIUS + EDGE_MARGIN;
		assert_eq!(clamp_center(-1919.0, 500.0, left), (-1920.0 + inset, 500.0));
		assert_eq!(clamp_center(-1.0, 500.0, left), (-inset, 500.0));
	}

	#[test]
	fn an_area_smaller_than_the_ring_centers_it() {
		let tiny = Rect {
			x: 100.0,
			y: 100.0,
			width: 200.0,
			height: 200.0,
		};
		assert_eq!(clamp_center(0.0, 0.0, tiny), (200.0, 200.0));
	}

	#[test]
	fn step_skips_empty_slots_and_wraps() {
		let filled = [true, false, true, false, false, true];
		assert_eq!(step(Some(0), true, &filled), Some(2));
		assert_eq!(step(Some(2), true, &filled), Some(5));
		assert_eq!(step(Some(5), true, &filled), Some(0));
		assert_eq!(step(Some(0), false, &filled), Some(5));
		assert_eq!(step(Some(2), false, &filled), Some(0));
	}

	#[test]
	fn step_from_nothing_starts_at_either_end() {
		let filled = [false, true, true, false];
		assert_eq!(step(None, true, &filled), Some(1));
		assert_eq!(step(None, false, &filled), Some(2));
	}

	#[test]
	fn step_with_one_filled_slot_stays_on_it() {
		let filled = [false, false, true, false];
		assert_eq!(step(Some(2), true, &filled), Some(2));
		assert_eq!(step(Some(2), false, &filled), Some(2));
	}

	#[test]
	fn step_with_no_filled_slot_is_none() {
		assert_eq!(step(Some(1), true, &[false; 5]), None);
		assert_eq!(step(None, true, &[]), None);
	}
}
