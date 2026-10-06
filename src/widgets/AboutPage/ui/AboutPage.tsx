// 정보 — 앱 이름, 버전, 로그 파일의 자리
import styled from "@emotion/styled";
import type { AppState } from "@/entities/AppState";
import { useT } from "@/shared/i18n";
import * as F from "@/shared/ui/form.styles";

interface AboutPageProps {
	appState: AppState;
}

export function AboutPage({ appState }: AboutPageProps) {
	const t = useT();
	return (
		<F.Page data-role="page-about">
			<F.PageTitle>{t("app.name")}</F.PageTitle>
			<F.PageLead>{t("about.lead")}</F.PageLead>

			<F.Group>
				<F.Row>
					<F.Label>{t("about.version")}</F.Label>
					<Value className="selectable" data-role="version">
						{appState.version}
						{appState.isDev && ` · ${t("about.devBuild")}`}
					</Value>
				</F.Row>
				<F.Row>
					<F.Label>{t("about.log")}</F.Label>
					<Value className="selectable" $mono>
						{appState.logPath}
					</Value>
				</F.Row>
			</F.Group>

			<F.Group>
				<F.Hint>{t("about.storage")}</F.Hint>
				<F.Hint>{t("about.copyright")}</F.Hint>
			</F.Group>
		</F.Page>
	);
}

const Value = styled.span<{ $mono?: boolean }>`
    min-width: 0;
    overflow: hidden;
    font-family: ${(p) => (p.$mono ? "var(--font-mono)" : "inherit")};
    font-size: ${(p) => (p.$mono ? "var(--text-meta)" : "var(--text-body)")};
    color: var(--text-2);
    text-overflow: ellipsis;
    white-space: nowrap;
`;
