// 여러 줄 글 입력 — 셸 명령처럼 고정폭으로 쓰는 글에 쓴다
import styled from "@emotion/styled";
import type { TextareaHTMLAttributes } from "react";

export function TextArea(props: TextareaHTMLAttributes<HTMLTextAreaElement>) {
	return <Root autoCapitalize="off" autoCorrect="off" spellCheck={false} {...props} />;
}

const Root = styled.textarea`
    display: block;
    width: 100%;
    min-height: 76px;
    padding: 8px 10px;
    border: none;
    border-radius: var(--radius);
    background: var(--bg-field);
    font-family: var(--font-mono);
    font-size: var(--text-meta);
    line-height: 1.5;
    color: var(--text);
    resize: vertical;
    outline: none;

    &::placeholder {
        font-family: var(--font);
        font-size: var(--text-body);
        color: var(--text-3);
    }

    /* 가만히 있을 때는 선이 없다. 글을 넣는 동안에만 accent 로 두른다. */
    &:focus {
        box-shadow: 0 0 0 2px var(--accent);
    }
`;
