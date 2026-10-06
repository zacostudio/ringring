// 설정 화면이 같이 쓰는 배치 조각 — 페이지, 줄, 이름표, 덧붙이는 글. 구분은 간격으로 한다
import styled from "@emotion/styled";

/* 페이지 하나. 본문 폭을 정한다. */
export const Page = styled.div`
    max-width: 600px;
`;

/* 페이지 제목. 한 페이지에 하나다. */
export const PageTitle = styled.h1`
    margin: 0 0 4px;
    font-size: var(--text-title);
    font-weight: 600;
    line-height: 1.3;
    letter-spacing: -0.01em;
`;

export const PageLead = styled.p`
    margin-bottom: 24px;
    color: var(--text-2);
`;

/* 묶음 하나. 묶음 사이는 간격으로만 가른다. */
export const Group = styled.section`
    display: flex;
    flex-direction: column;
    gap: 14px;

    & + & {
        margin-top: 30px;
    }
`;

export const GroupTitle = styled.h2`
    margin: 0;
    font-size: var(--text-meta);
    font-weight: 600;
    color: var(--text-3);
`;

/* 이름표가 왼쪽, 조작이 오른쪽인 줄. */
export const Row = styled.div`
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    min-height: var(--control-h);
`;

export const RowText = styled.div`
    min-width: 0;
`;

export const Label = styled.div`
    font-weight: 500;
`;

export const Hint = styled.p`
    font-size: var(--text-meta);
    color: var(--text-2);
`;

/* 이름표가 위, 조작이 아래인 칸. */
export const Stack = styled.div`
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
`;

/* 상태 한 줄. 아이콘과 글이다. 상자가 아니다. */
export const Status = styled.div<{ $tone: "ok" | "warn" | "error" | "plain" }>`
    display: flex;
    align-items: flex-start;
    gap: 7px;
    font-size: var(--text-meta);
    color: ${(p) =>
		p.$tone === "ok"
			? "var(--ok)"
			: p.$tone === "warn"
				? "var(--warn)"
				: p.$tone === "error"
					? "var(--danger)"
					: "var(--text-2)"};

    & > svg {
        flex-shrink: 0;
        margin-top: 2px;
    }
`;

/* 면 없는 글 버튼만 있는 줄. 글자의 왼쪽이 본문과 맞게 버튼의 안쪽 여백만큼 당긴다. */
export const QuietActions = styled.div`
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: -8px;
`;

export const Actions = styled.div`
    display: flex;
    align-items: center;
    gap: 8px;
`;
