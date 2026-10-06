// "셸 명령" 동작의 입력 — 명령, 작업 폴더, 제한 시간
import type { RingAction, RingLimits } from "@/entities/Ring";
import { useT } from "@/shared/i18n";
import { TextArea } from "@/shared/ui/TextArea";
import { TextField } from "@/shared/ui/TextField";
import * as F from "@/shared/ui/form.styles";
import * as S from "../RingEditor.styles";
import type { ActionChange } from "./actionChange";

interface ShellFieldsProps {
	action: Extract<RingAction, { kind: "shell" }>;
	limits: RingLimits;
	onChange: ActionChange;
	onCommit: () => void;
}

export function ShellFields({ action, limits, onChange, onCommit }: ShellFieldsProps) {
	const t = useT();
	return (
		<>
			<F.Stack>
				<TextArea
					data-field="shell-command"
					value={action.command}
					placeholder={t("shell.commandPlaceholder")}
					onChange={(event) => onChange({ ...action, command: event.target.value }, false)}
					onBlur={onCommit}
				/>
				<F.Hint>{t("shell.commandHint")}</F.Hint>
			</F.Stack>
			<F.Stack>
				<F.Label>{t("shell.workingDir")}</F.Label>
				<TextField
					mono
					data-field="shell-working-dir"
					value={action.working_dir}
					placeholder={t("shell.workingDirPlaceholder")}
					onChange={(event) => onChange({ ...action, working_dir: event.target.value }, false)}
					onBlur={onCommit}
				/>
			</F.Stack>
			<F.Row>
				<F.Label>{t("shell.timeout")}</F.Label>
				<S.NumberField>
					<TextField
						type="number"
						data-field="shell-timeout"
						min={1}
						max={limits.maxShellTimeoutSecs}
						step={1}
						value={action.timeout_secs}
						onChange={(event) =>
							onChange({ ...action, timeout_secs: Math.trunc(Number(event.target.value)) || 0 }, false)
						}
						onBlur={onCommit}
					/>
					<F.Hint>{t("shell.seconds")}</F.Hint>
				</S.NumberField>
			</F.Row>
		</>
	);
}
