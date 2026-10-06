// "열기" 동작의 입력 — 앱·파일·폴더는 고르기 창으로 고르고, 주소는 글로 적는다
import { ringRepository } from "@/entities/Ring";
import type { RingAction } from "@/entities/Ring";
import { useT } from "@/shared/i18n";
import { Button } from "@/shared/ui/Button";
import { Segmented } from "@/shared/ui/Segmented";
import { TextField } from "@/shared/ui/TextField";
import * as F from "@/shared/ui/form.styles";
import { OPEN_TARGET_ICON, baseName } from "../../lib/kinds";
import * as S from "../RingEditor.styles";
import type { ActionChange } from "./actionChange";

type OpenAction = Extract<RingAction, { kind: "open" }>;
type OpenTarget = OpenAction["target"];
const TARGETS: OpenTarget[] = ["app", "file", "url"];

interface OpenFieldsProps {
	action: OpenAction;
	onChange: ActionChange;
	/** 무엇을 열지 골랐다. 값은 비운다. 고른 것만으로는 저장하지 않는다. */
	onChooseTarget: (target: OpenTarget) => void;
	onCommit: () => void;
}

export function OpenFields({ action, onChange, onChooseTarget, onCommit }: OpenFieldsProps) {
	const t = useT();

	const choose = async () => {
		// 무엇을 고를 수 있는지는 Rust 가 정한다 — 파일 칸은 파일과 폴더, 앱 칸은 앱.
		const picked = await ringRepository.chooseOpenTarget(action.target).catch(() => null);
		if (picked === null) return;
		onChange({ ...action, value: picked }, true, {
			label: baseName(picked),
			icon: OPEN_TARGET_ICON[action.target]
		});
	};

	return (
		<>
			<Segmented
				label={t("open.target")}
				options={TARGETS.map((target) => ({ value: target, label: t(`open.target.${target}`) }))}
				value={action.target}
				onChange={(target) => target !== action.target && onChooseTarget(target)}
			/>
			{action.target === "url" ? (
				<F.Stack>
					<TextField
						mono
						data-field="open-url"
						value={action.value}
						placeholder="https://"
						onChange={(event) => onChange({ ...action, value: event.target.value }, false)}
						onBlur={onCommit}
					/>
					<F.Hint>{t("open.urlHint")}</F.Hint>
				</F.Stack>
			) : (
				<F.Stack>
					<S.PathRow>
						<Button onClick={() => void choose()}>
							{t(action.target === "app" ? "open.chooseApp" : "open.chooseFile")}
						</Button>
						{action.value && (
							<S.PathText className="selectable" title={action.value}>
								{/* rtl 로 앞을 줄인다. LRM 이 경로의 `/` 가 뒤집히지 않게 한다. */}
								{`‎${action.value}`}
							</S.PathText>
						)}
					</S.PathRow>
					{action.target === "file" && <F.Hint>{t("open.fileHint")}</F.Hint>}
				</F.Stack>
			)}
			{action.target === "app" && (
				<F.Stack>
					<F.Label>{t("open.args")}</F.Label>
					<TextField
						mono
						data-field="open-args"
						value={action.args ?? ""}
						placeholder={t("open.argsPlaceholder")}
						// 빈 인자는 key 째로 뺀다. 저장된 칸에도 그 key 가 없어 둘이 같은 글로 견줘진다.
						onChange={(event) => onChange({ ...action, args: event.target.value || undefined }, false)}
						onBlur={onCommit}
					/>
					<F.Hint>{t("open.argsHint")}</F.Hint>
				</F.Stack>
			)}
		</>
	);
}
