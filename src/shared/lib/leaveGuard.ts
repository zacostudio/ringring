// 떠나기 전에 묻는 곳 — 저장하지 않은 입력을 가진 편집기가 "떠나도 되는가" 를 답한다
//
// 글 입력 칸은 포커스를 잃을 때 저장한다. 다른 칸·링·페이지로 가거나 창을 닫을 때는 그 저장이 끝났는지,
// 거절되지 않았는지를 먼저 본다. 저장할 수 없는 입력이 남았으면 떠나지 않는다 — 친 글이 말없이 사라지지 않는다.

/** 저장하지 않은 입력을 저장해 본다. 떠나도 되면 true. 안 되면 까닭을 화면에 보이고 false. */
export type LeaveGuard = () => Promise<boolean>;

const guards = new Set<LeaveGuard>();

/** 편집기가 떠 있는 동안 건다. 돌려주는 함수로 푼다. */
export function registerLeaveGuard(guard: LeaveGuard): () => void {
	guards.add(guard);
	return () => {
		guards.delete(guard);
	};
}

/** 지금 화면을 떠나도 되는가. 모든 편집기가 저장을 끝내야 true 다. 하나가 실패해도 나머지는 저장해 본다. */
export async function canLeave(): Promise<boolean> {
	const answers = await Promise.all([...guards].map((guard) => guard().catch(() => false)));
	return answers.every(Boolean);
}
