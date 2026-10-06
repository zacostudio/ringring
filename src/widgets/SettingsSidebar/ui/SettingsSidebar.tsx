// 설정 창의 왼쪽 줄 — 위에는 링 목록, 아래에는 앱의 페이지들
import type { Ring } from "@/entities/Ring";
import { useT } from "@/shared/i18n";
import { formatShortcut } from "@/shared/lib/shortcut";
import { Icon } from "@/shared/ui/Icon";
import * as S from "./SettingsSidebar.styles";

export type SettingsPageName = "general" | "permissions" | "runs" | "backup" | "about";

/** 지금 본문에 보이는 것. 링 하나이거나 앱의 페이지 하나다. */
export type Selection = { kind: "ring"; ringId: string } | { kind: "page"; page: SettingsPageName };

const PAGES: Array<{ page: SettingsPageName; icon: string; labelKey: string }> = [
	{ page: "general", icon: "sliders-horizontal", labelKey: "nav.general" },
	{ page: "permissions", icon: "shield-check", labelKey: "nav.permissions" },
	{ page: "runs", icon: "scroll-text", labelKey: "nav.runs" },
	{ page: "backup", icon: "arrow-down-up", labelKey: "nav.backup" },
	{ page: "about", icon: "info", labelKey: "nav.about" }
];

export const PAGE_NAMES: readonly string[] = PAGES.map((entry) => entry.page);

interface SettingsSidebarProps {
	rings: Ring[];
	selection: Selection | null;
	onSelect: (selection: Selection) => void;
	onCreateRing: () => void;
	/** 권한 페이지에 손볼 것이 있다 — 키 입력 칸이 있는데 권한이 없다. */
	permissionsNeedAttention: boolean;
	/** 실행 기록에 아직 보지 않은 실패가 있다. */
	runsNeedAttention: boolean;
}

export function SettingsSidebar({
	rings,
	selection,
	onSelect,
	onCreateRing,
	permissionsNeedAttention,
	runsNeedAttention
}: SettingsSidebarProps) {
	const t = useT();
	return (
		<S.Root data-tauri-drag-region>
			<S.Heading>{t("nav.rings")}</S.Heading>
			<S.List>
				{rings.map((ring) => (
					<S.Item
						key={ring.id}
						type="button"
						data-ring={ring.id}
						$selected={selection?.kind === "ring" && selection.ringId === ring.id}
						onClick={() => onSelect({ kind: "ring", ringId: ring.id })}
					>
						<S.ItemName>{ring.name}</S.ItemName>
						{ring.shortcut && <S.ItemMeta>{formatShortcut(ring.shortcut)}</S.ItemMeta>}
					</S.Item>
				))}
				<S.Item type="button" data-role="ring-new" $selected={false} onClick={onCreateRing}>
					<Icon name="plus" size={14} />
					<S.ItemName>{t("nav.newRing")}</S.ItemName>
				</S.Item>
			</S.List>
			<S.Spacer data-tauri-drag-region />
			<S.List>
				{PAGES.map((entry) => (
					<S.Item
						key={entry.page}
						type="button"
						data-page={entry.page}
						$selected={selection?.kind === "page" && selection.page === entry.page}
						onClick={() => onSelect({ kind: "page", page: entry.page })}
					>
						<Icon name={entry.icon} size={16} />
						<S.ItemName>{t(entry.labelKey)}</S.ItemName>
						{((entry.page === "permissions" && permissionsNeedAttention) ||
							(entry.page === "runs" && runsNeedAttention)) && <S.Dot />}
					</S.Item>
				))}
			</S.List>
		</S.Root>
	);
}
