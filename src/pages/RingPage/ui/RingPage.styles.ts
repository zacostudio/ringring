// 링 창의 스타일 — 창은 투명하고 링만 보인다. 그림자는 링 모양을 따라 그린다
import { keyframes } from "@emotion/react";
import styled from "@emotion/styled";
import { RING_BOX, RING_SHADOW } from "@/entities/Ring";

const appear = keyframes`
    from {
        opacity: 0;
        transform: scale(0.92);
    }
    to {
        opacity: 1;
        transform: scale(1);
    }
`;

export const Root = styled.main`
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
`;

/* 링이 뜰 때만 움직인다. 칸의 강조는 바로 바뀐다. */
export const Stage = styled.div`
    position: relative;
    width: ${RING_BOX}px;
    height: ${RING_BOX}px;
    animation: ${appear} 90ms ease-out;
`;

/* 확인 물음. 링과 같은 크기의 면 하나다. */
export const Confirm = styled.div`
    position: absolute;
    inset: 8px;
    border-radius: 50%;
    background: var(--ring-surface);
    box-shadow: ${RING_SHADOW};
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 0 38px;
`;

export const ConfirmTitle = styled.div`
    display: flex;
    align-items: center;
    gap: 7px;
    max-width: 100%;
    font-size: var(--text-meta);
    font-weight: 600;
    color: var(--text);

    svg {
        flex-shrink: 0;
    }

    span {
        min-width: 0;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
`;

/* 글과, 아래에 더 있다는 표시를 겹쳐 두는 자리. */
export const ConfirmDetailWrap = styled.div`
    position: relative;
    max-width: 100%;
`;

/* 아래에 글이 더 있다. 글의 끝이 흐려지고 그 위에 아래 화살표가 놓인다. 끝까지 내리면 사라진다. */
export const ConfirmMore = styled.div`
    position: absolute;
    left: 0;
    right: 0;
    bottom: -9px;
    display: flex;
    justify-content: center;
    color: var(--text);
    pointer-events: none;

    svg {
        flex-shrink: 0;
    }
`;

export const ConfirmDetail = styled.div<{ $more: boolean }>`
    max-width: 100%;
    /* 여섯 줄. 더 긴 명령은 이 안에서 넘겨 본다 — 자르지 않는다. */
    max-height: 108px;
    overflow-y: auto;
    ${(p) =>
		p.$more
			? "mask-image: linear-gradient(to bottom, #000 calc(100% - 30px), transparent); -webkit-mask-image: linear-gradient(to bottom, #000 calc(100% - 30px), transparent);"
			: ""}
    font-family: var(--font-mono);
    font-size: var(--text-meta);
    line-height: 1.5;
    color: var(--text-2);
    text-align: center;
    word-break: break-all;
    white-space: pre-wrap;
`;

export const ConfirmActions = styled.div`
    display: flex;
    gap: 8px;
`;

export const ConfirmButton = styled.button<{ $primary?: boolean }>`
    height: var(--control-h);
    padding: 0 12px;
    border: none;
    border-radius: 6px;
    font-size: var(--text-meta);
    font-weight: 600;
    color: ${(p) => (p.$primary ? "var(--on-accent)" : "var(--text)")};
    background: ${(p) => (p.$primary ? "var(--accent)" : "var(--bg-selected)")};

    &:hover {
        background: ${(p) => (p.$primary ? "var(--accent-hover)" : "var(--bg-hover)")};
    }

    /* Enter 와 Space 는 focus 가 있는 버튼을 누른다. 어느 버튼인지 보여야 한다. */
    &:focus {
        outline: 2px solid var(--accent);
        outline-offset: 2px;
    }
`;
