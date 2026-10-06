// 가져오기·내보내기 — 링을 JSON 파일 하나로 옮긴다. 가져오기는 무엇이 들어오는지 먼저 보이고 더하기만 한다
import { open, save } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { ringFailureText, ringRepository } from "@/entities/Ring";
import type { RingImportSummary, RingLimits } from "@/entities/Ring";
import { useT } from "@/shared/i18n";
import { Button } from "@/shared/ui/Button";
import { Icon } from "@/shared/ui/Icon";
import * as F from "@/shared/ui/form.styles";

const FILE_FILTER = [{ name: "RingRing", extensions: ["json"] }];

interface BackupPageProps {
	limits: RingLimits;
	ringCount: number;
}

type Message = { tone: "ok" | "error"; text: string };

export function BackupPage({ limits, ringCount }: BackupPageProps) {
	const t = useT();
	const [exported, setExported] = useState<Message | null>(null);
	// 고른 파일과 그 파일이 들일 것. 사용자의 답을 기다린다.
	const [pending, setPending] = useState<{ path: string; summary: RingImportSummary } | null>(null);
	const [imported, setImported] = useState<Message | null>(null);
	const [done, setDone] = useState<RingImportSummary | null>(null);

	const exportAll = async () => {
		const path = await save({ defaultPath: "ringring-rings.json", filters: FILE_FILTER });
		if (!path) return;
		try {
			const count = await ringRepository.exportTo(path);
			setExported({ tone: "ok", text: t("backup.exported", { count }) });
		} catch (error) {
			setExported({ tone: "error", text: ringFailureText(error, limits, t, "backup.exportFailed") });
		}
	};

	const choose = async () => {
		const path = await open({ multiple: false, directory: false, filters: FILE_FILTER });
		if (typeof path !== "string") return;
		setImported(null);
		setDone(null);
		try {
			setPending({ path, summary: await ringRepository.importPreview(path) });
		} catch (error) {
			setPending(null);
			setImported({ tone: "error", text: ringFailureText(error, limits, t, "backup.importFailed") });
		}
	};

	const confirm = async () => {
		if (!pending) return;
		try {
			const summary = await ringRepository.importFrom(pending.path);
			setDone(summary);
			setImported({ tone: "ok", text: t("backup.imported", { count: summary.rings }) });
		} catch (error) {
			setImported({ tone: "error", text: ringFailureText(error, limits, t, "backup.importFailed") });
		}
		setPending(null);
	};

	/** 요약의 덧붙이는 줄들. 0 인 것은 적지 않는다. */
	const notes = (summary: RingImportSummary): string[] =>
		[
			summary.shellSlots + summary.keystrokeSlots + summary.openSlots > 0 &&
				t("backup.summary.confirm", { count: summary.shellSlots + summary.keystrokeSlots + summary.openSlots }),
			summary.droppedShortcuts > 0 && t("backup.summary.shortcuts", { count: summary.droppedShortcuts }),
			summary.skippedSlots > 0 && t("backup.summary.skipped", { count: summary.skippedSlots })
		].filter((line): line is string => typeof line === "string");

	return (
		<F.Page data-role="page-backup">
			<F.PageTitle>{t("nav.backup")}</F.PageTitle>
			<F.PageLead>{t("backup.lead")}</F.PageLead>

			<F.Group>
				<F.GroupTitle>{t("backup.export")}</F.GroupTitle>
				<F.Row>
					<F.RowText>
						<F.Hint>{t("backup.exportHint", { count: ringCount })}</F.Hint>
					</F.RowText>
					<Button data-role="export" disabled={ringCount === 0} onClick={() => void exportAll()}>
						{t("backup.exportButton")}
					</Button>
				</F.Row>
				{exported && (
					<F.Status $tone={exported.tone} data-role="export-result">
						<Icon name={exported.tone === "ok" ? "check" : "triangle-alert"} size={14} />
						<span>{exported.text}</span>
					</F.Status>
				)}
			</F.Group>

			<F.Group>
				<F.GroupTitle>{t("backup.import")}</F.GroupTitle>
				<F.Row>
					<F.RowText>
						<F.Hint>{t("backup.importHint")}</F.Hint>
					</F.RowText>
					<Button data-role="import-choose" onClick={() => void choose()}>
						{t("backup.importButton")}
					</Button>
				</F.Row>
				{pending && (
					<F.Stack data-role="import-preview">
						<p>
							{t("backup.summary.rings", { rings: pending.summary.rings, slots: pending.summary.slots })}
						</p>
						{notes(pending.summary).map((line) => (
							<F.Hint key={line}>{line}</F.Hint>
						))}
						<F.Actions>
							<Button variant="primary" data-role="import-confirm" onClick={() => void confirm()}>
								{t("backup.importConfirm")}
							</Button>
							<Button variant="quiet" onClick={() => setPending(null)}>
								{t("common.cancel")}
							</Button>
						</F.Actions>
					</F.Stack>
				)}
				{imported && (
					<F.Stack>
						<F.Status $tone={imported.tone} data-role="import-result">
							<Icon name={imported.tone === "ok" ? "check" : "triangle-alert"} size={14} />
							<span>{imported.text}</span>
						</F.Status>
						{done && notes(done).map((line) => <F.Hint key={line}>{line}</F.Hint>)}
					</F.Stack>
				)}
			</F.Group>
		</F.Page>
	);
}
