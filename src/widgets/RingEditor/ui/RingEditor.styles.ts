// 링 편집기의 스타일 — 머리줄, 링 그림, 칸 하나. 구분은 간격과 배경으로 하고 선은 긋지 않는다
import styled from "@emotion/styled";

export const Root = styled.div`
    display: flex;
    flex-direction: column;
    min-width: 0;
`;

/* 하위 링을 편집하러 들어왔을 때 돌아가는 줄. */
export const BackRow = styled.div`
    margin: -6px 0 10px -8px;
`;

export const Header = styled.div`
    display: flex;
    align-items: center;
    gap: 10px;
`;

/* 단축키 칸 앞의 짧은 이름. 일반·빠른 두 칸을 가른다. */
export const ShortcutLabel = styled.span`
    flex-shrink: 0;
    font-size: var(--text-meta);
    color: var(--text-2);
`;

/* 링 이름. 제목처럼 보이고, 가리키면 배경이, 고칠 때만 accent 가 보인다. */
export const NameInput = styled.input`
    flex: 1;
    min-width: 0;
    height: 32px;
    padding: 0 9px;
    margin-left: -9px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    font-size: var(--text-title);
    font-weight: 600;
    letter-spacing: -0.01em;
    color: var(--text);
    outline: none;

    &:hover {
        background: var(--bg-hover);
    }

    &:focus {
        background: var(--bg-field);
        box-shadow: 0 0 0 2px var(--accent);
    }
`;

/* 머리줄 아래의 한 줄. 지금 이 링이 어떻게 열리는지, 또는 왜 안 열리는지. */
export const HeaderNote = styled.p<{ $tone?: "error" | "warn" }>`
    min-height: 18px;
    margin-top: 6px;
    font-size: var(--text-meta);
    color: ${(p) => (p.$tone === "error" ? "var(--danger)" : p.$tone === "warn" ? "var(--warn)" : "var(--text-2)")};
`;

/* 머리줄 설명 안의 글 버튼. 설명과 같은 크기의 글이고 면이 없다. */
export const NoteAction = styled.button`
    margin-left: 8px;
    padding: 0;
    border: none;
    background: transparent;
    font-size: var(--text-meta);
    font-weight: 500;
    color: var(--text);
    text-decoration: underline;
    text-underline-offset: 2px;
`;

export const Body = styled.div`
    display: grid;
    grid-template-columns: 272px minmax(0, 1fr);
    column-gap: 44px;
    align-items: start;
    margin-top: 22px;
`;

export const DialColumn = styled.div`
    display: flex;
    flex-direction: column;
    gap: 18px;
`;

export const CountRow = styled.div`
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
`;

export const Confirming = styled.div`
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: var(--text-meta);
    color: var(--text-2);
`;

/* 칸 하나의 편집기. */
export const Slot = styled.div`
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
    max-width: 400px;
`;

export const SlotHead = styled.div`
    display: flex;
    align-items: center;
    gap: 4px;
    height: var(--control-h);
`;

export const SlotTitle = styled.h2`
    flex: 1;
    margin: 0;
    font-size: var(--text-body);
    font-weight: 600;
`;

export const Fields = styled.div`
    display: flex;
    flex-direction: column;
    gap: 14px;
`;

export const NameRow = styled.div`
    display: flex;
    gap: 8px;
`;

/* 지금 고른 아이콘을 보이는 단추. 누르면 아이콘 고르기가 열린다. */
export const IconSwatch = styled.button<{ $open: boolean }>`
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--control-h);
    height: var(--control-h);
    padding: 0;
    flex-shrink: 0;
    border: none;
    border-radius: var(--radius);
    background: ${(p) => (p.$open ? "var(--bg-selected)" : "var(--bg-field)")};
    color: var(--text);

    & > svg {
        flex-shrink: 0;
    }

    &:hover {
        background: var(--bg-selected);
    }

    &:focus-visible {
        outline: 2px solid var(--accent);
        outline-offset: 1px;
    }
`;

export const IconGrid = styled.div`
    display: grid;
    grid-template-columns: repeat(auto-fill, 30px);
    gap: 2px;
    max-height: 132px;
    margin-top: 8px;
    overflow-y: auto;
`;

export const IconCell = styled.button<{ $selected: boolean }>`
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: ${(p) => (p.$selected ? "var(--accent)" : "transparent")};
    color: ${(p) => (p.$selected ? "var(--on-accent)" : "var(--text-2)")};

    & > svg {
        flex-shrink: 0;
    }

    &:hover {
        background: ${(p) => (p.$selected ? "var(--accent)" : "var(--bg-selected)")};
        color: ${(p) => (p.$selected ? "var(--on-accent)" : "var(--text)")};
    }
`;

/* 고른 파일이나 앱의 경로. 길면 가운데가 아니라 앞을 줄인다 — 끝의 이름이 보여야 한다. */
export const PathText = styled.span`
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-family: var(--font-mono);
    font-size: var(--text-meta);
    color: var(--text-2);
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
`;

export const PathRow = styled.div`
    display: flex;
    align-items: center;
    gap: 10px;
`;

export const ComboRow = styled.div`
    display: flex;
    align-items: center;
    gap: 4px;
`;

export const ComboList = styled.div`
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
`;

/* 하위 링으로 고를 링의 줄. 고른 줄만 배경이 있다. */
export const ChoiceList = styled.div`
    display: flex;
    flex-direction: column;
    margin: 0 -8px;
`;

export const Choice = styled.button<{ $selected: boolean }>`
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 30px;
    padding: 0 8px;
    border: none;
    border-radius: var(--radius);
    background: ${(p) => (p.$selected ? "var(--bg-selected)" : "transparent")};
    color: var(--text);
    font-weight: ${(p) => (p.$selected ? 600 : 400)};
    text-align: left;

    & > svg {
        flex-shrink: 0;
        color: ${(p) => (p.$selected ? "var(--accent)" : "var(--text-3)")};
    }

    &:hover:not(:disabled) {
        background: ${(p) => (p.$selected ? "var(--bg-selected)" : "var(--bg-hover)")};
    }

    &:disabled {
        opacity: 0.5;
        cursor: default;
    }
`;

export const ChoiceName = styled.span`
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
`;

export const ChoiceMeta = styled.span`
    flex-shrink: 0;
    font-size: var(--text-meta);
    font-weight: 400;
    color: var(--text-3);
`;

export const ChoiceReason = styled.p`
    padding: 0 8px 4px 32px;
    font-size: var(--text-meta);
    color: var(--text-3);
`;

export const NumberField = styled.div`
    display: flex;
    align-items: center;
    gap: 8px;
    width: 120px;
`;
