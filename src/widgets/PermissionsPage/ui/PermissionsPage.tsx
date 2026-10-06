// 권한 — 손쉬운 사용 권한의 지금 상태와, 시스템 설정의 그 화면을 여는 단추 하나
import { appRepository, reloadAppState } from "@/entities/AppState";
import type { AppState } from "@/entities/AppState";
import { useT } from "@/shared/i18n";
import { Button } from "@/shared/ui/Button";
import { Icon } from "@/shared/ui/Icon";
import * as F from "@/shared/ui/form.styles";

interface PermissionsPageProps {
	appState: AppState;
}

export function PermissionsPage({ appState }: PermissionsPageProps) {
	const t = useT();
	const trusted = appState.accessibilityTrusted;
	return (
		<F.Page data-role="page-permissions">
			<F.PageTitle>{t("nav.permissions")}</F.PageTitle>
			<F.PageLead>{t("permissions.lead")}</F.PageLead>

			<F.Group>
				<F.Row>
					<F.RowText>
						<F.Label>{t("permissions.accessibility")}</F.Label>
						<F.Status
							$tone={trusted ? "ok" : "warn"}
							data-role="accessibility-status"
							data-trusted={trusted}
						>
							<Icon name={trusted ? "circle-check" : "triangle-alert"} size={14} />
							<span>{t(trusted ? "permissions.granted" : "permissions.missing")}</span>
						</F.Status>
					</F.RowText>
					<F.Actions>
						<Button variant="quiet" onClick={() => void reloadAppState()}>
							{t("permissions.recheck")}
						</Button>
						<Button
							variant={trusted ? "default" : "primary"}
							data-role="accessibility-open"
							onClick={() => void appRepository.openAccessibilitySettings().catch(() => {})}
						>
							{t("permissions.open")}
						</Button>
					</F.Actions>
				</F.Row>
				<p>{t("permissions.why")}</p>
				{!trusted && <p>{t("permissions.how")}</p>}
				<F.Hint>{t("permissions.without")}</F.Hint>
				{!trusted && appState.usesKeystrokes && <F.Hint>{t("permissions.affected")}</F.Hint>}
				{appState.isDev && <F.Hint>{t("permissions.devNote")}</F.Hint>}
			</F.Group>
		</F.Page>
	);
}
