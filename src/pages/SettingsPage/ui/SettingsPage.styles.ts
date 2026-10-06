// 설정 창의 뼈대 — 왼쪽 줄과 본문 둘. 둘은 바탕 색으로만 갈린다
import styled from "@emotion/styled";

export const Shell = styled.div`
    display: flex;
    height: 100%;
`;

export const Main = styled.main`
    position: relative;
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    /* 위쪽 48px 는 창을 끄는 자리다. 왼쪽 줄의 첫 글과 본문의 제목이 같은 높이에서 시작한다. */
    padding: 48px 36px 36px;
`;

/* 창을 끌 수 있는 위쪽 띠. 제목 줄이 없는 창이다. */
export const DragStrip = styled.div`
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 44px;
`;
