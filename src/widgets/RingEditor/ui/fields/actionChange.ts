// 동작 입력 칸들이 편집기에 변경을 알리는 모양
import type { RingAction } from "@/entities/Ring";

/** 동작을 고르면서 같이 채울 칸의 이름과 아이콘. 비어 있을 때만 쓴다. */
export interface SlotDefaults {
	label?: string;
	icon?: string;
}

/** 동작을 바꾼다. `save` 면 바로 저장한다. 글 입력은 `save` 없이 바꾸고 blur 에서 `onCommit` 을 부른다. */
export type ActionChange = (action: RingAction, save: boolean, defaults?: SlotDefaults) => void;
