// 아이콘 고르기 — 검색 입력과 lucide 아이콘 격자. 검색어가 없으면 추천 목록을 보인다
import { icons } from "lucide-react";
import { useMemo, useState } from "react";
import { useT } from "@/shared/i18n";
import { Icon } from "@/shared/ui/Icon";
import { TextField } from "@/shared/ui/TextField";
import { SUGGESTED_ICONS } from "../lib/suggestedIcons";
import * as S from "./RingEditor.styles";

/** 검색 결과로 보일 아이콘의 최대 수. */
const MAX_RESULTS = 96;

/** `ArrowDownToLine` → `arrow-down-to-line`. `Icon` 이 받는 이름이다. */
function toKebab(name: string): string {
	return name
		.replace(/([a-z])([A-Z0-9])/g, "$1-$2")
		.replace(/([0-9])([A-Z])/g, "$1-$2")
		.toLowerCase();
}

interface IconPickerProps {
	value: string;
	onPick: (icon: string) => void;
}

export function IconPicker({ value, onPick }: IconPickerProps) {
	const t = useT();
	const [query, setQuery] = useState("");
	const all = useMemo(() => Object.keys(icons).map(toKebab), []);
	const shown = useMemo(() => {
		const wanted = query.trim().toLowerCase();
		if (!wanted) return SUGGESTED_ICONS;
		return all.filter((name) => name.includes(wanted)).slice(0, MAX_RESULTS);
	}, [all, query]);

	return (
		<div data-role="icon-picker">
			<TextField
				value={query}
				placeholder={t("slot.iconSearch")}
				autoFocus
				onChange={(event) => setQuery(event.target.value)}
			/>
			<S.IconGrid>
				{shown.map((name) => (
					<S.IconCell
						key={name}
						type="button"
						title={name}
						data-icon={name}
						$selected={name === value}
						onClick={() => onPick(name)}
					>
						<Icon name={name} size={16} />
					</S.IconCell>
				))}
			</S.IconGrid>
		</div>
	);
}
