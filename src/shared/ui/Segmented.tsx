// 몇 개 가운데 하나를 고르는 줄 — 고른 것만 면이 올라온다
import styled from "@emotion/styled";
import { Icon } from "./Icon";

export interface SegmentOption<T extends string | number> {
	value: T;
	label: string;
	icon?: string;
}

interface SegmentedProps<T extends string | number> {
	options: SegmentOption<T>[];
	/** 고른 값. 아직 고르지 않았으면 null. */
	value: T | null;
	onChange: (value: T) => void;
	/** 화면 낭독기가 읽는 이름. */
	label: string;
}

export function Segmented<T extends string | number>({ options, value, onChange, label }: SegmentedProps<T>) {
	return (
		<Root role="radiogroup" aria-label={label}>
			{options.map((option) => (
				<Option
					key={option.value}
					type="button"
					role="radio"
					aria-checked={option.value === value}
					data-value={option.value}
					$selected={option.value === value}
					onClick={() => onChange(option.value)}
				>
					{option.icon && <Icon name={option.icon} size={14} />}
					{option.label}
				</Option>
			))}
		</Root>
	);
}

const Root = styled.div`
    display: inline-flex;
    /* 세로로 쌓인 칸 안에서도 제 폭만 차지한다. */
    align-self: flex-start;
    max-width: 100%;
    /* 좁은 칸에서는 줄을 바꾼다. 넘쳐서 잘리지 않는다. */
    flex-wrap: wrap;
    gap: 2px;
    padding: 2px;
    border-radius: 9px;
    background: var(--bg-field);
`;

const Option = styled.button<{ $selected: boolean }>`
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 24px;
    min-width: 30px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius);
    font-size: var(--text-body);
    font-weight: ${(p) => (p.$selected ? 600 : 500)};
    white-space: nowrap;
    color: ${(p) => (p.$selected ? "var(--text)" : "var(--text-2)")};
    background: ${(p) => (p.$selected ? "var(--bg-float)" : "transparent")};
    box-shadow: ${(p) => (p.$selected ? "0 1px 2px rgba(0, 0, 0, 0.14)" : "none")};

    & > svg {
        flex-shrink: 0;
    }

    &:hover {
        color: var(--text);
    }

    &:focus-visible {
        outline: 2px solid var(--accent);
        outline-offset: 1px;
    }
`;
