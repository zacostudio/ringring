// 확인 물음에서 무엇을 실행하는지 보이는 글 — 길면 넘겨 보고, 아래에 더 있으면 그것을 보인다
import { useEffect, useRef, useState } from "react";
import { Icon } from "@/shared/ui/Icon";
import * as S from "./RingPage.styles";

/** 끝에 닿았다고 보는 여유. 소수점 스크롤 위치 때문에 딱 맞지 않는다. */
const END_SLACK = 2;

export function ConfirmDetail({ text }: { text: string }) {
	const box = useRef<HTMLDivElement>(null);
	// 아래에 아직 안 보이는 글이 있는가.
	const [more, setMore] = useState(false);

	const measure = () => {
		const el = box.current;
		if (el) setMore(el.scrollHeight - el.scrollTop - el.clientHeight > END_SLACK);
	};
	// 글이 바뀌면 다시 잰다.
	useEffect(measure, [text]);

	return (
		<S.ConfirmDetailWrap>
			<S.ConfirmDetail ref={box} $more={more} data-role="confirm-detail" data-more={more} onScroll={measure}>
				{text}
			</S.ConfirmDetail>
			{more && (
				<S.ConfirmMore aria-hidden>
					<Icon name="chevron-down" size={14} />
				</S.ConfirmMore>
			)}
		</S.ConfirmDetailWrap>
	);
}
