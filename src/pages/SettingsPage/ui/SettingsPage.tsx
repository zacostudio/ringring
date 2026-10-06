// 설정 창의 페이지 — 왼쪽 줄에서 고른 링이나 페이지를 본문에 보인다. 무엇을 저장할지는 Rust 가 정한다
import { useCallback, useEffect, useRef, useState } from "react";
import { appRepository, useAppState } from "@/entities/AppState";
import { ringFailureText, ringRepository, useRings } from "@/entities/Ring";
import type { Ring } from "@/entities/Ring";
import { useT } from "@/shared/i18n";
import { canLeave } from "@/shared/lib/leaveGuard";
import { showToast } from "@/shared/lib/toast";
import { EVENTS } from "@/shared/tauri/events";
import { subscribe } from "@/shared/tauri/ipc";
import { ToastHost } from "@/shared/ui/ToastHost";
import { AboutPage } from "@/widgets/AboutPage";
import { BackupPage } from "@/widgets/BackupPage";
import { GeneralPage } from "@/widgets/GeneralPage";
import { PermissionsPage } from "@/widgets/PermissionsPage";
import { RingEditor } from "@/widgets/RingEditor";
import { RunsPage } from "@/widgets/RunsPage";
import { PAGE_NAMES, SettingsSidebar } from "@/widgets/SettingsSidebar";
import type { Selection, SettingsPageName } from "@/widgets/SettingsSidebar";
import { Welcome } from "@/widgets/Welcome";
import * as S from "./SettingsPage.styles";

function asPageName(value: string | null): SettingsPageName | null {
	return value !== null && PAGE_NAMES.includes(value) ? (value as SettingsPageName) : null;
}

/** 창을 열 때 Rust 가 주소에 실어 준 페이지 이름. */
function initialSelection(): Selection | null {
	const page = asPageName(new URLSearchParams(window.location.search).get("page"));
	return page ? { kind: "page", page } : null;
}

export function SettingsPage() {
	const t = useT();
	const appState = useAppState();
	const { rings, limits, reload, replace } = useRings();
	const [selection, setSelection] = useState<Selection | null>(initialSelection);
	const [position, setPosition] = useState(0);
	// 칸에서 "이 링 편집" 으로 하위 링에 들어왔을 때 돌아갈 자리. 다른 것을 고르면 버린다.
	const [returnTo, setReturnTo] = useState<{ ringId: string; position: number; subRingId: string } | null>(null);

	const loaded = appState !== null && rings !== null && limits !== null;

	// 첫 화면을 그린 뒤에 창을 보인다. 그 전에 보이면 빈 창이 한 번 보인다.
	const announced = useRef(false);
	useEffect(() => {
		if (!loaded || announced.current) return;
		announced.current = true;
		void appRepository.settingsReady().catch(() => {});
	}, [loaded]);

	// 지금 편집기를 떠나는 일은 모두 여기를 거친다. 저장하지 않은 입력을 먼저 저장하고, 저장할 수 없으면
	// 떠나지 않는다. 까닭은 그 편집기가 보이고, 여기서는 떠나지 못했다는 것만 알린다.
	const blockedText = t("unsaved.blocked");
	const leaveThen = useCallback(
		async (go: () => void) => {
			if (await canLeave()) go();
			else showToast(blockedText);
		},
		[blockedText]
	);

	// 트레이의 "정보" 처럼, 떠 있는 창에 페이지를 보이라는 요청.
	useEffect(
		() =>
			subscribe<string>(EVENTS.settingsNavigate, (name) => {
				const page = asPageName(name);
				if (page) {
					void leaveThen(() => {
						setSelection({ kind: "page", page });
						setReturnTo(null);
					});
				}
			}),
		[leaveThen]
	);

	// 창을 닫거나 앱을 끝내려 한다 (Rust 의 `settings_window::request_leave`). 저장하고 답한다.
	// 저장할 수 없는 입력이 남았으면 창은 그대로 있다.
	useEffect(
		() =>
			subscribe<number>(EVENTS.settingsLeaveRequest, (id) => {
				void canLeave().then((mayLeave) => {
					if (!mayLeave) showToast(blockedText);
					return appRepository.leaveAnswer(id, mayLeave).catch(() => {});
				});
			}),
		[blockedText]
	);

	const select = useCallback(
		(next: Selection) =>
			void leaveThen(() => {
				setSelection(next);
				setPosition(0);
				setReturnTo(null);
			}),
		[leaveThen]
	);

	const createRing = async (starter: boolean) => {
		if (!limits) return;
		if (!(await canLeave())) {
			showToast(blockedText);
			return;
		}
		try {
			const created = await (starter ? ringRepository.createStarter() : ringRepository.create());
			await reload();
			setSelection({ kind: "ring", ringId: created.id });
			setPosition(0);
			setReturnTo(null);
		} catch (error) {
			showToast(ringFailureText(error, limits, t, "common.saveFailed"));
		}
	};

	if (!loaded) return null;

	// 고른 것이 없거나 고른 링이 지워졌으면 첫 링을 보인다.
	const ring: Ring | null =
		selection?.kind === "page"
			? null
			: ((selection && rings.find((candidate) => candidate.id === selection.ringId)) ?? rings[0] ?? null);
	const shown: Selection | null =
		selection?.kind === "page" ? selection : ring ? { kind: "ring", ringId: ring.id } : null;

	// 돌아갈 링이 그사이 지워졌으면 돌아갈 곳이 없다.
	const parent = returnTo && ring?.id === returnTo.subRingId ? rings.find((r) => r.id === returnTo.ringId) : null;

	const openPermissions = () => select({ kind: "page", page: "permissions" });

	const content = () => {
		if (selection?.kind === "page") {
			switch (selection.page) {
				case "general":
					return <GeneralPage appState={appState} />;
				case "permissions":
					return <PermissionsPage appState={appState} />;
				case "runs":
					return <RunsPage />;
				case "backup":
					return <BackupPage limits={limits} ringCount={rings.length} />;
				case "about":
					return <AboutPage appState={appState} />;
			}
		}
		if (!ring) {
			return (
				<Welcome onCreateStarter={() => void createRing(true)} onCreateEmpty={() => void createRing(false)} />
			);
		}
		return (
			<RingEditor
				key={ring.id}
				ring={ring}
				limits={limits}
				appState={appState}
				position={position}
				onSelectPosition={(next) => void leaveThen(() => setPosition(next))}
				onRingChanged={replace}
				onReload={reload}
				onEditRing={(subRingId) =>
					void leaveThen(() => {
						setReturnTo({ ringId: ring.id, position: Math.min(position, ring.slotCount - 1), subRingId });
						setSelection({ kind: "ring", ringId: subRingId });
						setPosition(0);
					})
				}
				onOpenPermissions={openPermissions}
				back={
					returnTo && parent
						? {
								label: t("ring.backTo", { name: parent.name, n: returnTo.position + 1 }),
								go: () =>
									void leaveThen(() => {
										setSelection({ kind: "ring", ringId: returnTo.ringId });
										setPosition(returnTo.position);
										setReturnTo(null);
									})
							}
						: null
				}
			/>
		);
	};

	return (
		<S.Shell>
			<SettingsSidebar
				rings={rings}
				selection={shown}
				onSelect={select}
				onCreateRing={() => void createRing(false)}
				permissionsNeedAttention={appState.usesKeystrokes && !appState.accessibilityTrusted}
				runsNeedAttention={appState.unseenFailures > 0}
			/>
			<S.Main>
				<S.DragStrip data-tauri-drag-region />
				{content()}
			</S.Main>
			<ToastHost />
		</S.Shell>
	);
}
