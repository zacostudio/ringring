// 버튼 — 면이 있는 기본, 강조, 그리고 면 없이 글만 있는 quiet. 높이는 control 높이 하나다
import styled from "@emotion/styled";
import type { ButtonHTMLAttributes } from "react";

type Variant = "default" | "primary" | "quiet";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
	variant?: Variant;
	/** 지우기처럼 되돌릴 수 없는 일이다. 글자를 danger 색으로 쓴다. */
	danger?: boolean;
}

export function Button({ variant = "default", danger = false, type = "button", ...props }: ButtonProps) {
	return <Root type={type} $variant={variant} $danger={danger} {...props} />;
}

const Root = styled.button<{ $variant: Variant; $danger: boolean }>`
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: var(--control-h);
    padding: 0 ${(p) => (p.$variant === "quiet" ? 8 : 12)}px;
    flex-shrink: 0;
    border: none;
    border-radius: var(--radius);
    font-size: var(--text-body);
    font-weight: 500;
    white-space: nowrap;
    color: ${(p) =>
		p.$danger
			? "var(--danger)"
			: p.$variant === "primary"
				? "var(--on-accent)"
				: p.$variant === "quiet"
					? "var(--text-2)"
					: "var(--text)"};
    background: ${(p) =>
		p.$variant === "primary" ? "var(--accent)" : p.$variant === "quiet" ? "transparent" : "var(--bg-field)"};

    & > svg {
        flex-shrink: 0;
    }

    &:hover:not(:disabled) {
        background: ${(p) => (p.$variant === "primary" ? "var(--accent-hover)" : "var(--bg-selected)")};
        color: ${(p) => (p.$danger ? "var(--danger)" : p.$variant === "primary" ? "var(--on-accent)" : "var(--text)")};
    }

    &:focus-visible {
        outline: 2px solid var(--accent);
        outline-offset: 1px;
    }

    &:disabled {
        opacity: 0.45;
        cursor: default;
    }
`;
