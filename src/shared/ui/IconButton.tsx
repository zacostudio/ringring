// 아이콘만 있는 버튼 — 면은 hover 에서만 보인다. `label` 은 화면 낭독기와 tooltip 의 글이다
import styled from "@emotion/styled";
import type { ButtonHTMLAttributes } from "react";
import { Icon } from "./Icon";

interface IconButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
	icon: string;
	label: string;
}

export function IconButton({ icon, label, type = "button", ...props }: IconButtonProps) {
	return (
		<Root type={type} aria-label={label} title={label} {...props}>
			<Icon name={icon} size={16} />
		</Root>
	);
}

const Root = styled.button`
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--control-h);
    height: var(--control-h);
    /* WebKit 의 기본 padding 이 남으면 아이콘이 눌려 안 보인다. */
    padding: 0;
    flex-shrink: 0;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-2);

    & > svg {
        flex-shrink: 0;
    }

    &:hover:not(:disabled) {
        background: var(--bg-selected);
        color: var(--text);
    }

    &:focus-visible {
        outline: 2px solid var(--accent);
        outline-offset: 1px;
    }

    &:disabled {
        opacity: 0.35;
        cursor: default;
    }
`;
