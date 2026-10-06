// 실행 기록 — 셸 명령 칸의 최근 실행과, 어떤 칸이든 실패한 실행. 줄을 누르면 까닭과 출력을 편다. 기록은 메모리에만 있다
import styled from "@emotion/styled";
import { useCallback, useEffect, useState } from "react";
import { appRepository } from "@/entities/AppState";
import type { RunRecord } from "@/entities/AppState";
import { useLocale, useT } from "@/shared/i18n";
import { EVENTS } from "@/shared/tauri/events";
import { subscribe } from "@/shared/tauri/ipc";
import { Button } from "@/shared/ui/Button";
import { Icon } from "@/shared/ui/Icon";
import * as F from "@/shared/ui/form.styles";

/** 걸린 시간을 짧게 적는다. 1초 아래는 ms, 그 위는 초다. */
function formatDuration(ms: number): string {
	return ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(1)} s`;
}

export function RunsPage() {
	const t = useT();
	const locale = useLocale();
	const [runs, setRuns] = useState<RunRecord[]>([]);
	const [openId, setOpenId] = useState<number | null>(null);

	const reload = useCallback(() => {
		void appRepository.runs().then(setRuns, (error) => console.error("[runs] failed to load:", error));
		// 이 페이지가 떠 있으면 사용자가 실패를 본 것이다. 트레이와 왼쪽 줄의 표시를 내린다.
		void appRepository.runsSeen().catch(() => {});
	}, []);

	useEffect(() => {
		reload();
		return subscribe<void>(EVENTS.runsChanged, reload);
	}, [reload]);

	const time = new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit", second: "2-digit" });

	return (
		<F.Page data-role="page-runs">
			<F.PageTitle>{t("nav.runs")}</F.PageTitle>
			<F.PageLead>{t("runs.lead")}</F.PageLead>

			{runs.length === 0 ? (
				<F.Hint data-role="runs-empty">{t("runs.empty")}</F.Hint>
			) : (
				<>
					<List>
						{runs.map((run) => {
							const open = run.id === openId;
							return (
								<div key={run.id} data-role="run" data-ok={run.ok}>
									<Row type="button" $open={open} onClick={() => setOpenId(open ? null : run.id)}>
										<Mark $ok={run.ok}>
											<Icon name={run.ok ? "check" : "x"} size={14} />
										</Mark>
										<Name>{run.label || run.error}</Name>
										<Meta>
											{run.exitCode !== null &&
												run.exitCode !== 0 &&
												t("runs.exitCode", { code: run.exitCode })}
										</Meta>
										{run.shell && <Meta>{formatDuration(run.durationMs)}</Meta>}
										<Meta>{time.format(new Date(run.finishedAt))}</Meta>
									</Row>
									{open && (
										<Detail>
											{run.error && <F.Status $tone="error">{run.error}</F.Status>}
											{run.output.trim() ? (
												<Output className="selectable">{run.output}</Output>
											) : (
												run.shell && <F.Hint>{t("runs.noOutput")}</F.Hint>
											)}
										</Detail>
									)}
								</div>
							);
						})}
					</List>
					<Foot>
						<Button variant="quiet" data-role="runs-clear" onClick={() => void appRepository.clearRuns()}>
							{t("runs.clear")}
						</Button>
					</Foot>
				</>
			)}
		</F.Page>
	);
}

const List = styled.div`
    display: flex;
    flex-direction: column;
    gap: 1px;
    margin: 0 -8px;
`;

const Row = styled.button<{ $open: boolean }>`
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 32px;
    padding: 0 8px;
    border: none;
    border-radius: var(--radius);
    background: ${(p) => (p.$open ? "var(--bg-selected)" : "transparent")};
    text-align: left;

    &:hover {
        background: ${(p) => (p.$open ? "var(--bg-selected)" : "var(--bg-hover)")};
    }
`;

const Mark = styled.span<{ $ok: boolean }>`
    display: inline-flex;
    color: ${(p) => (p.$ok ? "var(--ok)" : "var(--danger)")};

    & > svg {
        flex-shrink: 0;
    }
`;

const Name = styled.span`
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
`;

const Meta = styled.span`
    flex-shrink: 0;
    font-size: var(--text-meta);
    font-variant-numeric: tabular-nums;
    color: var(--text-3);
`;

const Detail = styled.div`
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px 8px 12px 32px;
`;

const Output = styled.pre`
    max-height: 220px;
    margin: 0;
    overflow: auto;
    font-family: var(--font-mono);
    font-size: var(--text-meta);
    line-height: 1.5;
    color: var(--text-2);
    white-space: pre-wrap;
    word-break: break-all;
`;

const Foot = styled.div`
    margin: 12px 0 0 -8px;
`;
