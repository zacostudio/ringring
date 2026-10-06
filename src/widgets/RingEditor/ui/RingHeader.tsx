// 링 편집기의 머리줄 — 이름, 전역 단축키, 띄워 보기, 삭제. 그 아래 한 줄이 이 링이 어떻게 열리는지 말한다
import { useEffect, useRef, useState } from "react";
import { appRepository } from "@/entities/AppState";
import type { AppState } from "@/entities/AppState";
import { ringFailureText, ringRepository } from "@/entities/Ring";
import type { Ring, RingLimits } from "@/entities/Ring";
import { useT } from "@/shared/i18n";
import { registerLeaveGuard } from "@/shared/lib/leaveGuard";
import { showToast } from "@/shared/lib/toast";
import { Button } from "@/shared/ui/Button";
import { IconButton } from "@/shared/ui/IconButton";
import { ShortcutField } from "@/shared/ui/ShortcutField";
import * as S from "./RingEditor.styles";

interface RingHeaderProps {
	ring: Ring;
	limits: RingLimits;
	appState: AppState;
	onRingChanged: (ring: Ring) => void;
	onReload: () => Promise<void>;
}

export function RingHeader({ ring, limits, appState, onRingChanged, onReload }: RingHeaderProps) {
	const t = useT();
	const [name, setName] = useState(ring.name);
	// 입력 칸을 채울 때의 저장된 이름. 다른 곳이 이름을 바꾸면 입력 칸도 그 이름으로 바꾼다 —
	// 그러지 않으면 다음 blur 가 예전 이름을 도로 저장한다. 치던 글자는 버린다.
	const [savedName, setSavedName] = useState(ring.name);
	if (ring.name !== savedName) {
		setSavedName(ring.name);
		setName(ring.name);
	}
	// 이름이 거절된 이유. 친 이름은 입력 칸에 그대로 둔다 — 고치거나 되돌릴 때까지 보인다.
	const [nameError, setNameError] = useState<string | null>(null);
	// 단축키가 거절된 이유. 다음 입력까지 보인다.
	const [shortcutError, setShortcutError] = useState<string | null>(null);
	const [deleting, setDeleting] = useState(false);

	/** 친 이름을 저장한다. 저장됐거나 바뀐 것이 없으면 true. 거절되면 까닭을 보이고 false. */
	const saveName = async (): Promise<boolean> => {
		if (name === ring.name) {
			setNameError(null);
			return true;
		}
		try {
			onRingChanged(await ringRepository.save(ring.id, name, ring.slotCount));
			setNameError(null);
			return true;
		} catch (error) {
			setNameError(ringFailureText(error, limits, t, "common.saveFailed"));
			return false;
		}
	};
	const saveNameLatest = useRef(saveName);
	saveNameLatest.current = saveName;
	// 다른 링·페이지로 가거나 창을 닫기 전에 불린다. 이름을 저장할 수 없으면 떠나지 않는다.
	useEffect(() => registerLeaveGuard(() => saveNameLatest.current()), []);

	const revertName = () => {
		setName(ring.name);
		setNameError(null);
	};

	// **등록이 먼저다.** Rust 가 OS 에 등록한 뒤에만 저장한다. 거절되면 화면의 값은 그대로다.
	// 입력받기를 끝내는 것도 이 command 가 Rust 에서 한다 — 화면이 보내는 "끝" 과 순서를 다투지 않는다.
	const setShortcut = async (shortcut: string | null) => {
		try {
			await ringRepository.setShortcut(ring.id, shortcut);
			setShortcutError(null);
			await onReload();
		} catch (error) {
			setShortcutError(ringFailureText(error, limits, t, "common.saveFailed"));
		}
	};

	const deleteRing = async () => {
		try {
			const refs = await ringRepository.delete(ring.id);
			if (refs.length > 0) {
				setDeleting(false);
				showToast(t("ring.deleteBlocked"), {
					detail: refs
						.map((ref) => t("ring.deleteBlockedBy", { ring: ref.ringName, n: ref.position + 1 }))
						.join("\n")
				});
				return;
			}
			await onReload();
		} catch (error) {
			showToast(ringFailureText(error, limits, t, "common.saveFailed"));
		}
	};

	const tryIt = () =>
		void ringRepository.show(ring.id).catch((error: unknown) => {
			showToast(ringFailureText(error, limits, t, "ring.showFailed"));
		});

	const note = (): { text: string; tone?: "error" | "warn" } => {
		if (nameError) return { text: nameError, tone: "error" };
		if (shortcutError) return { text: shortcutError, tone: "error" };
		if (!ring.shortcut) return { text: t("ring.noShortcutNote") };
		if (!appState.shortcutsRegister) return { text: t("ring.devNote"), tone: "warn" };
		if (appState.paused) return { text: t("ring.pausedNote"), tone: "warn" };
		if (appState.refusedShortcuts.includes(ring.id)) return { text: t("ring.refusedNote"), tone: "error" };
		if (ring.slots.length === 0) return { text: t("ring.emptyNote"), tone: "warn" };
		return { text: t("ring.shortcutNote") };
	};
	const shown = note();

	return (
		<div>
			<S.Header>
				<S.NameInput
					data-field="ring-name"
					aria-label={t("ring.name")}
					value={name}
					maxLength={limits.nameMaxChars}
					autoCapitalize="off"
					autoCorrect="off"
					spellCheck={false}
					onChange={(event) => setName(event.target.value)}
					onBlur={() => void saveName()}
					onKeyDown={(event) => {
						if (event.key === "Enter") event.currentTarget.blur();
						if (event.key === "Escape") revertName();
					}}
				/>
				{deleting ? (
					<>
						<Button danger data-role="ring-delete-confirm" onClick={() => void deleteRing()}>
							{t("ring.deleteConfirm")}
						</Button>
						<Button variant="quiet" onClick={() => setDeleting(false)}>
							{t("common.cancel")}
						</Button>
					</>
				) : (
					<>
						<ShortcutField
							mode="global"
							value={ring.shortcut}
							texts={{
								empty: t("shortcut.none"),
								capturing: t("shortcut.capturing"),
								clear: t("common.clear")
							}}
							onCapture={(active, id) => void appRepository.shortcutCapture(active, id).catch(() => {})}
							onChange={(shortcut) => void setShortcut(shortcut)}
							onClear={() => void setShortcut(null)}
						/>
						<Button data-role="ring-try" disabled={ring.slots.length === 0} onClick={tryIt}>
							{t("ring.tryIt")}
						</Button>
						<IconButton
							icon="trash"
							label={t("ring.delete")}
							data-role="ring-delete"
							onClick={() => setDeleting(true)}
						/>
					</>
				)}
			</S.Header>
			<S.HeaderNote $tone={shown.tone} data-role="ring-note">
				{deleting ? t("ring.deleteQuestion", { name: ring.name }) : shown.text}
				{nameError && !deleting && (
					<S.NoteAction type="button" data-role="ring-name-revert" onClick={revertName}>
						{t("common.discardEdit")}
					</S.NoteAction>
				)}
			</S.HeaderNote>
		</div>
	);
}
