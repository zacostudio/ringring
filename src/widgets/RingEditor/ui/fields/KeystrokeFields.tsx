// "키 입력" 동작의 입력 — 보낼 조합의 목록과, 손쉬운 사용 권한이 없을 때의 한 줄
import { useState } from "react";
import { appRepository, useAppState } from "@/entities/AppState";
import type { RingAction, RingLimits } from "@/entities/Ring";
import { useT } from "@/shared/i18n";
import { Button } from "@/shared/ui/Button";
import { Icon } from "@/shared/ui/Icon";
import { IconButton } from "@/shared/ui/IconButton";
import { ShortcutField } from "@/shared/ui/ShortcutField";
import * as F from "@/shared/ui/form.styles";
import * as S from "../RingEditor.styles";
import type { ActionChange } from "./actionChange";

interface KeystrokeFieldsProps {
	action: Extract<RingAction, { kind: "keystroke" }>;
	limits: RingLimits;
	onChange: ActionChange;
	/** 권한 페이지로 간다. */
	onOpenPermissions: () => void;
}

export function KeystrokeFields({ action, limits, onChange, onOpenPermissions }: KeystrokeFieldsProps) {
	const t = useT();
	const appState = useAppState();
	// 아직 조합을 받지 않은 새 줄. 받으면 `combos` 로 들어간다.
	const [adding, setAdding] = useState(action.combos.length === 0);
	const texts = { empty: t("keystroke.empty"), capturing: t("shortcut.capturing"), clear: t("common.clear") };
	const capture = (active: boolean, id: number) => void appRepository.shortcutCapture(active, id).catch(() => {});

	// 같은 조합이 두 번 올 수 있다. 몇 번째로 나온 것인지를 key 에 붙여 서로 다르게 한다.
	const seen = new Map<string, number>();
	const keys = action.combos.map((combo) => {
		const count = (seen.get(combo) ?? 0) + 1;
		seen.set(combo, count);
		return `${combo}#${count}`;
	});

	const setCombos = (combos: string[]) => onChange({ kind: "keystroke", combos }, combos.length > 0);
	const canAdd = action.combos.length < limits.maxCombos;

	return (
		<>
			<F.Stack>
				<S.ComboList>
					{action.combos.map((combo, index) => (
						<S.ComboRow key={keys[index]}>
							<ShortcutField
								mode="send"
								value={combo}
								texts={texts}
								onCapture={capture}
								onChange={(next) =>
									setCombos(action.combos.map((old, i) => (i === index ? next : old)))
								}
							/>
							<IconButton
								icon="x"
								label={t("keystroke.remove")}
								onClick={() => setCombos(action.combos.filter((_, i) => i !== index))}
							/>
						</S.ComboRow>
					))}
					{adding && canAdd && (
						<ShortcutField
							mode="send"
							value={null}
							texts={texts}
							onCapture={capture}
							onChange={(next) => {
								setAdding(false);
								setCombos([...action.combos, next]);
							}}
						/>
					)}
					{!adding && canAdd && (
						<F.QuietActions>
							<Button variant="quiet" data-role="combo-add" onClick={() => setAdding(true)}>
								<Icon name="plus" size={14} />
								{t("keystroke.add")}
							</Button>
						</F.QuietActions>
					)}
				</S.ComboList>
				<F.Hint>{t("keystroke.hint")}</F.Hint>
			</F.Stack>
			{appState && !appState.accessibilityTrusted && (
				<F.Stack>
					<F.Status $tone="warn">
						<Icon name="triangle-alert" size={14} />
						<span>{t("keystroke.accessMissing")}</span>
					</F.Status>
					<F.Actions>
						<Button onClick={onOpenPermissions}>{t("keystroke.accessOpen")}</Button>
					</F.Actions>
				</F.Stack>
			)}
		</>
	);
}
