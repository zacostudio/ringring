// 일반 설정 — 언어, 테마, 단축키 일시 정지, 로그인할 때 실행. 값은 Rust 가 저장하고 돌려준다
import { appRepository, setAppState } from "@/entities/AppState";
import type { AppState, Language, Theme } from "@/entities/AppState";
import { useT } from "@/shared/i18n";
import { asCommandError } from "@/shared/lib/commandError";
import { showToast } from "@/shared/lib/toast";
import { Button } from "@/shared/ui/Button";
import { Segmented } from "@/shared/ui/Segmented";
import { Toggle } from "@/shared/ui/Toggle";
import * as F from "@/shared/ui/form.styles";

interface GeneralPageProps {
	appState: AppState;
}

const LANGUAGES: Array<{ value: Language; labelKey?: string; label?: string }> = [
	{ value: "system", labelKey: "general.followSystem" },
	{ value: "ko", label: "한국어" },
	{ value: "en", label: "English" },
	{ value: "ja", label: "日本語" }
];

const THEMES: Array<{ value: Theme; labelKey: string }> = [
	{ value: "system", labelKey: "general.followSystem" },
	{ value: "light", labelKey: "general.theme.light" },
	{ value: "dark", labelKey: "general.theme.dark" }
];

export function GeneralPage({ appState }: GeneralPageProps) {
	const t = useT();

	/** 설정을 바꾼다. Rust 가 돌려준 상태를 그대로 넣는다. */
	const apply = async (change: Promise<AppState>) => {
		try {
			setAppState(await change);
		} catch (error) {
			showToast(t("common.saveFailed"), { detail: asCommandError(error)?.message });
		}
	};

	const login = appState.loginItem;

	return (
		<F.Page data-role="page-general">
			<F.PageTitle>{t("nav.general")}</F.PageTitle>
			<F.PageLead>{t("general.lead")}</F.PageLead>

			<F.Group>
				<F.GroupTitle>{t("general.appearance")}</F.GroupTitle>
				<F.Row>
					<F.Label>{t("general.language")}</F.Label>
					<Segmented
						label={t("general.language")}
						options={LANGUAGES.map((entry) => ({
							value: entry.value,
							label: entry.label ?? t(entry.labelKey ?? "")
						}))}
						value={appState.language}
						onChange={(language) => void apply(appRepository.setLanguage(language))}
					/>
				</F.Row>
				<F.Row>
					<F.Label>{t("general.theme")}</F.Label>
					<Segmented
						label={t("general.theme")}
						options={THEMES.map((entry) => ({ value: entry.value, label: t(entry.labelKey) }))}
						value={appState.theme}
						onChange={(theme) => void apply(appRepository.setTheme(theme))}
					/>
				</F.Row>
			</F.Group>

			<F.Group>
				<F.GroupTitle>{t("general.behavior")}</F.GroupTitle>
				<F.Row>
					<F.RowText>
						<F.Label>{t("general.pause")}</F.Label>
						<F.Hint>{t(appState.shortcutsRegister ? "general.pauseHint" : "general.devShortcuts")}</F.Hint>
					</F.RowText>
					<Toggle
						checked={appState.paused}
						label={t("general.pause")}
						onChange={(paused) => void apply(appRepository.setPaused(paused))}
					/>
				</F.Row>
				<F.Row>
					<F.RowText>
						<F.Label>{t("general.launchAtLogin")}</F.Label>
						<F.Hint>
							{t(
								!login.available
									? "general.launchUnavailable"
									: login.needsApproval
										? "general.launchNeedsApproval"
										: "general.launchHint"
							)}
						</F.Hint>
					</F.RowText>
					<F.Actions>
						{login.needsApproval && (
							<Button onClick={() => void appRepository.openLoginItemsSettings().catch(() => {})}>
								{t("general.openLoginItems")}
							</Button>
						)}
						<Toggle
							checked={login.enabled || login.needsApproval}
							disabled={!login.available}
							label={t("general.launchAtLogin")}
							onChange={(enabled) => void apply(appRepository.setLaunchAtLogin(enabled))}
						/>
					</F.Actions>
				</F.Row>
			</F.Group>
		</F.Page>
	);
}
