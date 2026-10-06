// 설정 창 왼쪽 줄의 스타일 — 바탕 색 하나로 본문과 갈린다. 선은 없고, 고른 줄만 배경이 있다
import styled from "@emotion/styled";

export const Root = styled.nav`
    display: flex;
    flex-direction: column;
    width: 212px;
    flex-shrink: 0;
    /* 위쪽 48px 는 창 단추의 자리다. */
    padding: var(--window-top) 10px 12px;
    background: var(--bg-side);
`;

export const Heading = styled.h2`
    margin: 0;
    padding: 0 8px 6px;
    font-size: var(--text-meta);
    font-weight: 600;
    color: var(--text-3);
`;

export const List = styled.div`
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-height: 0;
    overflow-y: auto;
`;

export const Item = styled.button<{ $selected: boolean }>`
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 30px;
    flex-shrink: 0;
    padding: 0 8px;
    border: none;
    border-radius: var(--radius);
    background: ${(p) => (p.$selected ? "var(--bg-selected)" : "transparent")};
    color: ${(p) => (p.$selected ? "var(--text)" : "var(--text-2)")};
    font-weight: ${(p) => (p.$selected ? 600 : 500)};
    text-align: left;

    & > svg {
        flex-shrink: 0;
    }

    &:hover {
        background: ${(p) => (p.$selected ? "var(--bg-selected)" : "var(--bg-hover)")};
        color: var(--text);
    }

    &:focus-visible {
        outline: 2px solid var(--accent);
        outline-offset: -2px;
    }
`;

export const ItemName = styled.span`
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
`;

export const ItemMeta = styled.span`
    flex-shrink: 0;
    font-size: var(--text-meta);
    font-weight: 400;
    letter-spacing: 0.04em;
    color: var(--text-3);
`;

/* 손볼 것이 있다는 점 하나. */
export const Dot = styled.span`
    width: 7px;
    height: 7px;
    flex-shrink: 0;
    border-radius: 50%;
    background: var(--warn);
`;

export const Spacer = styled.div`
    flex: 1;
    min-height: 16px;
`;
