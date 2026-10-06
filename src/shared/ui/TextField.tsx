// 한 줄 글 입력 — 선 없이 배경으로만 칸임을 보인다. 자동 대문자와 맞춤법 검사는 끈다
import styled from "@emotion/styled";
import type { InputHTMLAttributes } from "react";

interface TextFieldProps extends InputHTMLAttributes<HTMLInputElement> {
	/** 고정폭 글꼴로 쓴다 (경로, 주소). */
	mono?: boolean;
}

export function TextField({ mono = false, ...props }: TextFieldProps) {
	return <Root $mono={mono} autoCapitalize="off" autoCorrect="off" spellCheck={false} {...props} />;
}

const Root = styled.input<{ $mono: boolean }>`
    width: 100%;
    min-width: 0;
    height: var(--control-h);
    padding: 0 10px;
    border: none;
    border-radius: var(--radius);
    background: var(--bg-field);
    font-family: ${(p) => (p.$mono ? "var(--font-mono)" : "inherit")};
    font-size: ${(p) => (p.$mono ? "var(--text-meta)" : "var(--text-body)")};
    color: var(--text);
    outline: none;

    /* 안내 글은 고정폭으로 쓰지 않는다. 한글이 벌어져 보인다. */
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
