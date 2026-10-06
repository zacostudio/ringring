// 링 그림의 스타일 — 면 하나다. 가장자리에도 칸 사이에도 선을 긋지 않고 켜진 칸만 accent 로 채운다
import styled from "@emotion/styled";
import { RING_BOX, RING_OUTER_RADIUS } from "./ringShape";

export const Box = styled.div<{ $size: number }>`
    position: relative;
    width: ${(p) => p.$size}px;
    height: ${(p) => p.$size}px;
    flex-shrink: 0;
`;

/* 그림은 늘 272px 로 그리고 바깥 상자에 맞춰 줄인다. */
export const Scaled = styled.div<{ $scale: number }>`
    position: absolute;
    left: 0;
    top: 0;
    width: ${RING_BOX}px;
    height: ${RING_BOX}px;
    transform: scale(${(p) => p.$scale});
    transform-origin: 0 0;
`;

/*
 * 링의 그림자. 링 창은 링 바깥으로 32px 뿐이다 (Rust 의 WINDOW_SIZE 320 / 2 - OUTER_RADIUS 128).
 * 그림자는 아래로 6 + 22 = 28px, 옆으로 22px 까지만 닿는다. 이 값을 키우면 창의 네모난 가장자리에서 잘려
 * 창 모양이 보인다 (Tome).
 */
export const RING_SHADOW = "0 6px 22px rgba(0, 0, 0, 0.3), 0 1px 3px rgba(0, 0, 0, 0.22)";

/*
 * 그림자만 그리는 원. box-shadow 는 자기 상자 안쪽에는 그리지 않는다 — 그림자가 바깥 원만 따르고
 * 가운데 구멍에는 지지 않는다. filter: drop-shadow 는 구멍 안에도 그림자를 떨어뜨린다 (Tome).
 */
export const Shadow = styled.div`
    position: absolute;
    left: ${RING_BOX / 2 - RING_OUTER_RADIUS}px;
    top: ${RING_BOX / 2 - RING_OUTER_RADIUS}px;
    width: ${RING_OUTER_RADIUS * 2}px;
    height: ${RING_OUTER_RADIUS * 2}px;
    border-radius: 50%;
    box-shadow: ${RING_SHADOW};
    pointer-events: none;
`;

export const Base = styled.svg`
    position: absolute;
    inset: 0;
    width: ${RING_BOX}px;
    height: ${RING_BOX}px;
    overflow: visible;
`;

export const Surface = styled.path`
    fill: var(--ring-surface);
`;

export const Hub = styled.circle`
    fill: var(--ring-surface);
`;

export const Highlight = styled.path`
    fill: var(--accent);
    stroke: var(--accent);
    stroke-width: 7;
    stroke-linejoin: round;
`;

export const HitArea = styled.path`
    fill: transparent;
    cursor: pointer;

    &:hover {
        fill: var(--bg-hover);
    }
`;

export const Slot = styled.div<{ $on: boolean }>`
    position: absolute;
    width: 76px;
    height: 48px;
    margin: -24px 0 0 -38px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 5px;
    pointer-events: none;
    color: ${(p) => (p.$on ? "var(--on-accent)" : "var(--text-2)")};

    svg {
        flex-shrink: 0;
        color: ${(p) => (p.$on ? "var(--on-accent)" : "var(--text)")};
    }
`;

export const SlotLabel = styled.span<{ $on: boolean }>`
    max-width: 100%;
    font-size: 12px;
    font-weight: ${(p) => (p.$on ? 600 : 500)};
    line-height: 1.2;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
`;

/* 빈 칸의 자리 표시. 설정의 편집기에서만 보인다. */
export const EmptyMark = styled.div<{ $on: boolean }>`
    position: absolute;
    width: 20px;
    height: 20px;
    margin: -10px 0 0 -10px;
    pointer-events: none;
    color: ${(p) => (p.$on ? "var(--on-accent)" : "var(--text-3)")};

    svg {
        display: block;
    }
`;

export const Chevron = styled.div<{ $on: boolean }>`
    position: absolute;
    width: 12px;
    height: 12px;
    margin: -6px 0 0 -6px;
    pointer-events: none;
    color: ${(p) => (p.$on ? "var(--on-accent)" : "var(--text-3)")};

    svg {
        display: block;
    }
`;

export const HubIcon = styled.div`
    position: absolute;
    left: ${RING_BOX / 2}px;
    top: ${RING_BOX / 2}px;
    width: 36px;
    height: 36px;
    margin: -18px 0 0 -18px;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    color: var(--text-3);
`;
