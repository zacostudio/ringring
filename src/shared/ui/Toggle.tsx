// 켜고 끄는 스위치
import styled from "@emotion/styled";

interface ToggleProps {
	checked: boolean;
	onChange: (checked: boolean) => void;
	/** 화면 낭독기가 읽는 이름. */
	label: string;
	disabled?: boolean;
}

export function Toggle({ checked, onChange, label, disabled = false }: ToggleProps) {
	return (
		<Root
			type="button"
			role="switch"
			aria-checked={checked}
			aria-label={label}
			disabled={disabled}
			$checked={checked}
			onClick={() => onChange(!checked)}
		>
			<Knob $checked={checked} />
		</Root>
	);
}

const Root = styled.button<{ $checked: boolean }>`
    position: relative;
    width: 36px;
    height: 22px;
    padding: 0;
    flex-shrink: 0;
    border: none;
    border-radius: 11px;
    background: ${(p) => (p.$checked ? "var(--accent)" : "var(--bg-selected)")};
    transition: background-color 120ms ease;

    &:focus-visible {
        outline: 2px solid var(--accent);
        outline-offset: 2px;
    }

    &:disabled {
        opacity: 0.45;
        cursor: default;
    }
`;

const Knob = styled.span<{ $checked: boolean }>`
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #ffffff;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
    transform: translateX(${(p) => (p.$checked ? 14 : 0)}px);
    transition: transform 120ms ease;
`;
