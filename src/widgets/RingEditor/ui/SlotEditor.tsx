// 칸 하나의 편집기 — 동작, 이름, 아이콘, 실행 전에 확인. 고칠 때마다 Rust 에 저장하고, 거절되면 그 이유를 보인다
import { useEffect, useRef, useState } from "react";
import { ringNotice, ringRepository } from "@/entities/Ring";
import type { Ring, RingAction, RingActionKind, RingLimits, RingNotice } from "@/entities/Ring";

type OpenTarget = Extract<RingAction, { kind: "open" }>["target"];
import { useT } from "@/shared/i18n";
import { registerLeaveGuard } from "@/shared/lib/leaveGuard";
import { Button } from "@/shared/ui/Button";
import { Icon } from "@/shared/ui/Icon";
import { IconButton } from "@/shared/ui/IconButton";
import { Segmented } from "@/shared/ui/Segmented";
import { TextField } from "@/shared/ui/TextField";
import { Toggle } from "@/shared/ui/Toggle";
import * as F from "@/shared/ui/form.styles";
import { ACTION_KINDS, OPEN_TARGET_ICON, blankAction, isAutoIcon, kindIcon, swapOrder } from "../lib/kinds";
import { type SlotDraft, draftOf, savedSlot, slotSnapshot } from "../lib/slotDraft";
import { IconPicker } from "./IconPicker";
import * as S from "./RingEditor.styles";
import { KeystrokeFields } from "./fields/KeystrokeFields";
import { OpenFields } from "./fields/OpenFields";
import { ShellFields } from "./fields/ShellFields";
import { SubRingFields } from "./fields/SubRingFields";
import type { SlotDefaults } from "./fields/actionChange";

interface SlotEditorProps {
	ring: Ring;
	position: number;
	limits: RingLimits;
	onRingChanged: (ring: Ring) => void;
	/** 링 목록을 다시 읽는다. 하위 링을 자리에서 만들면 목록에 링이 하나 는다. */
	onReload: () => Promise<void>;
	/** 다른 자리의 칸을 고른다 (칸을 옮긴 뒤 그 칸을 따라간다). */
	onSelectPosition: (position: number) => void;
	/** 이 칸이 여는 링을 편집하러 간다. */
	onEditRing: (ringId: string) => void;
	onOpenPermissions: () => void;
}

export function SlotEditor({
	ring,
	position,
	limits,
	onRingChanged,
	onReload,
	onSelectPosition,
	onEditRing,
	onOpenPermissions
}: SlotEditorProps) {
	const t = useT();
	const saved = savedSlot(ring, position);
	const [draft, setDraft] = useState<SlotDraft>(() => draftOf(saved));
	// 초안을 만들 때 이 자리에 저장돼 있던 칸.
	const [base, setBase] = useState(() => slotSnapshot(saved));
	// 초안을 버리고 다시 만든 횟수. 동작 입력 칸의 key 다 — 그 칸들이 쥔 화면 상태도 새로 시작한다.
	const [generation, setGeneration] = useState(0);
	// Rust 가 저장을 거절한 이유. 검증은 Rust 에만 있다 — 여기서는 받은 코드의 문장을 보이기만 한다.
	// 아직 채우지 않은 값 때문이면 오류가 아니다. 무엇이 남았는지를 보통 글로 알린다.
	const [notice, setNotice] = useState<RingNotice | null>(null);
	const [pickingIcon, setPickingIcon] = useState(false);
	// 사용자가 고쳤는데 아직 저장되지 않은 것이 있는가. 종류만 고른 빈 초안은 고친 것이 아니다.
	const touched = useRef(false);
	// 가장 새 초안. blur 와 "떠나기 전 저장" 은 그리기를 기다리지 않고 이 값을 저장한다.
	const latest = useRef(draft);
	// 돌고 있는 저장. 같은 초안을 두 번 저장하지 않게 그 끝을 기다린다.
	const saving = useRef<Promise<boolean> | null>(null);
	const root = useRef<HTMLDivElement>(null);

	// 이 자리의 칸이 이 편집기의 저장이 아닌 일로 바뀌었다 — 순서 바꾸기, 가져오기, 칸 비우기.
	// 초안은 예전 칸의 것이므로 버리고 지금 칸으로 다시 만든다. 저장하지 않은 입력도 함께 버린다.
	// 예전 초안을 지금 칸에 쓰는 것보다 낫다.
	const current = slotSnapshot(saved);
	if (current !== base) {
		touched.current = false;
		latest.current = draftOf(saved);
		setBase(current);
		setDraft(latest.current);
		setGeneration(generation + 1);
		setNotice(null);
		setPickingIcon(false);
	}

	/** 실패를 한 줄로 옮긴다. 링의 거절이 아니면 "저장하지 못했습니다" 다. */
	const noticeOf = (reason: unknown): RingNotice =>
		ringNotice(reason, limits, t) ?? { text: t("common.saveFailed"), incomplete: false };

	/** 초안을 저장한다. 저장됐으면 true. 거절되면 까닭을 보이고 false. */
	const commit = (next: SlotDraft): Promise<boolean> => {
		const action = next.action;
		if (!action) return Promise.resolve(true);
		const run = (async () => {
			try {
				const changed = await ringRepository.saveSlot(ring.id, {
					position,
					label: next.label,
					icon: next.icon,
					action,
					confirm: next.confirm
				});
				// 이 편집기의 저장이다. 초안은 그대로 두고 기준만 옮긴다 — 저장이 끝나기 전에 친 글자를 잃지 않는다.
				setBase(slotSnapshot(savedSlot(changed, position)));
				onRingChanged(changed);
				setNotice(null);
				// 저장하는 동안 더 고쳤으면 그것은 아직 저장되지 않았다.
				if (latest.current === next) touched.current = false;
				return true;
			} catch (reason) {
				setNotice(noticeOf(reason));
				return false;
			}
		})();
		saving.current = run;
		void run.finally(() => {
			if (saving.current === run) saving.current = null;
		});
		return run;
	};

	/** 고쳤는데 저장하지 않은 것이 있으면 저장한다. 남은 것이 없으면 true. */
	const flush = async (): Promise<boolean> => {
		if (saving.current) await saving.current;
		if (!touched.current) return true;
		return commit(latest.current);
	};
	const flushLatest = useRef(flush);
	flushLatest.current = flush;

	// 다른 칸·링·페이지로 가거나 창을 닫기 전에 불린다. 저장할 수 없으면 떠나지 않고 까닭이 보이게 한다.
	useEffect(
		() =>
			registerLeaveGuard(async () => {
				const saved = await flushLatest.current();
				if (!saved) root.current?.scrollIntoView({ block: "nearest" });
				return saved;
			}),
		[]
	);

	/** 저장하지 않은 입력을 버리고 저장된 칸으로 돌아간다. */
	const discard = () => {
		touched.current = false;
		latest.current = draftOf(saved);
		setDraft(latest.current);
		setGeneration(generation + 1);
		setNotice(null);
		setPickingIcon(false);
	};

	// 빈 하위 링을 만들어 이 칸에 잇는다. 만들기와 잇기는 Rust 가 한 번에 한다.
	// 저장된 칸이 바뀌므로 위의 `current !== base` 가 초안을 새 칸으로 다시 만든다.
	const createSubRing = async () => {
		try {
			const created = await ringRepository.createSubRing(ring.id, position, draft.label, draft.icon);
			onRingChanged(created.ring);
			await onReload();
		} catch (reason) {
			setNotice(noticeOf(reason));
		}
	};

	/** 초안을 바꾼다. `save` 면 바로 저장한다 (고르기·토글). 글 입력은 blur 에서 저장한다. */
	const update = (patch: Partial<SlotDraft>, save: boolean) => {
		const next = { ...latest.current, ...patch };
		touched.current = true;
		latest.current = next;
		setDraft(next);
		if (save) void commit(next);
	};

	const chooseKind = (kind: RingActionKind) => {
		if (draft.action?.kind === kind) return;
		setNotice(null);
		// 종류만 골랐을 때는 저장하지 않는다. 아직 채울 값이 남아 있다.
		// 편집기가 채운 아이콘이면 새 종류의 것으로 바꾼다. 사용자가 고른 아이콘은 그대로 둔다.
		latest.current = {
			...draft,
			action: blankAction(kind, limits),
			icon: isAutoIcon(draft.icon) ? kindIcon(kind) : draft.icon,
			confirm: limits.confirmByDefaultKinds.includes(kind)
		};
		setDraft(latest.current);
	};

	// 무엇을 열지만 골랐다. 종류를 고른 것과 같다 — 값은 비우고, 저장하지 않고, 고친 것으로 치지 않는다.
	const chooseOpenTarget = (target: OpenTarget) => {
		setNotice(null);
		latest.current = {
			...latest.current,
			action: { kind: "open", target, value: "" },
			icon: isAutoIcon(latest.current.icon) ? OPEN_TARGET_ICON[target] : latest.current.icon
		};
		setDraft(latest.current);
	};

	/**
	 * 동작이 바뀌었다. 이름과 아이콘은 사용자가 아직 정하지 않았을 때만 그 동작의 것으로 채운다.
	 * 하위 링은 다르다 — 칸의 이름이 곧 그 링의 이름이므로 고른 링의 이름을 그대로 쓴다.
	 */
	const changeAction = (action: RingAction, save: boolean, defaults?: SlotDefaults) => {
		const label =
			action.kind === "open_ring" && defaults?.label
				? defaults.label
				: latest.current.label || defaults?.label || "";
		update(
			{
				action,
				label,
				icon: defaults?.icon && isAutoIcon(latest.current.icon) ? defaults.icon : latest.current.icon
			},
			save
		);
	};

	const clear = async () => {
		try {
			onRingChanged(await ringRepository.clearSlot(ring.id, position));
			touched.current = false;
			latest.current = draftOf(null);
			setDraft(latest.current);
			setBase(slotSnapshot(null));
			setNotice(null);
		} catch (reason) {
			setNotice(noticeOf(reason));
		}
	};

	const move = async (clockwise: boolean) => {
		const { order, target } = swapOrder(ring.slotCount, position, clockwise);
		// 칸이 자리를 옮기면 초안은 버려진다. 저장하지 않은 입력을 먼저 저장한다.
		if (!(await flush())) return;
		try {
			onRingChanged(await ringRepository.reorderSlots(ring.id, order));
			onSelectPosition(target);
		} catch (reason) {
			setNotice(noticeOf(reason));
		}
	};

	const action = draft.action;
	const kind = action?.kind ?? null;
	// 하위 링 열기는 실행이 아니라 다음 링을 보이는 것이다. 확인을 묻지 않으므로 토글도 없다.
	const asksConfirm = kind !== null && !limits.kindsWithoutConfirm.includes(kind);

	return (
		<S.Slot ref={root} data-role="slot-editor" data-position={position}>
			<S.SlotHead>
				<S.SlotTitle>{t("slot.title", { n: position + 1 })}</S.SlotTitle>
				<IconButton
					icon="rotate-ccw"
					label={t("slot.moveCounterClockwise")}
					data-role="move-ccw"
					disabled={!saved}
					onClick={() => void move(false)}
				/>
				<IconButton
					icon="rotate-cw"
					label={t("slot.moveClockwise")}
					data-role="move-cw"
					disabled={!saved}
					onClick={() => void move(true)}
				/>
			</S.SlotHead>

			<Segmented
				label={t("slot.action")}
				options={ACTION_KINDS.map((entry) => ({
					value: entry.kind,
					label: t(entry.labelKey),
					icon: entry.icon
				}))}
				value={kind}
				onChange={chooseKind}
			/>

			{action === null && <F.Hint>{t("slot.chooseAction")}</F.Hint>}

			{action !== null && (
				<S.Fields key={generation}>
					{action.kind === "open" && (
						<OpenFields
							action={action}
							onChange={changeAction}
							onChooseTarget={chooseOpenTarget}
							onCommit={() => void flush()}
						/>
					)}
					{action.kind === "shell" && (
						<ShellFields
							action={action}
							limits={limits}
							onChange={changeAction}
							onCommit={() => void flush()}
						/>
					)}
					{action.kind === "keystroke" && (
						<KeystrokeFields
							action={action}
							limits={limits}
							onChange={changeAction}
							onOpenPermissions={onOpenPermissions}
						/>
					)}
					{action.kind === "open_ring" && (
						<SubRingFields
							ringId={ring.id}
							position={position}
							action={action}
							limits={limits}
							onChange={changeAction}
							savedLabel={saved?.label ?? ""}
							onCreate={() => void createSubRing()}
							onEditRing={onEditRing}
						/>
					)}
				</S.Fields>
			)}

			{action !== null && (
				<F.Stack>
					<F.Label>{t("slot.name")}</F.Label>
					<S.NameRow>
						<S.IconSwatch
							type="button"
							data-role="icon-swatch"
							aria-label={t("slot.icon")}
							title={t("slot.icon")}
							$open={pickingIcon}
							onClick={() => setPickingIcon(!pickingIcon)}
						>
							<Icon name={draft.icon || kindIcon(action.kind)} size={16} />
						</S.IconSwatch>
						<TextField
							data-field="slot-name"
							value={draft.label}
							maxLength={limits.nameMaxChars}
							placeholder={t("slot.namePlaceholder")}
							onChange={(event) => update({ label: event.target.value }, false)}
							onBlur={() => void flush()}
						/>
					</S.NameRow>
					{action.kind === "open_ring" && <F.Hint>{t("slot.subRingNameHint")}</F.Hint>}
					{pickingIcon && (
						<IconPicker
							value={draft.icon}
							onPick={(icon) => {
								setPickingIcon(false);
								update({ icon }, true);
							}}
						/>
					)}
				</F.Stack>
			)}

			{asksConfirm && (
				<F.Row>
					<F.RowText>
						<F.Label>{t("slot.confirm")}</F.Label>
						<F.Hint>{t("slot.confirmHint")}</F.Hint>
					</F.RowText>
					<Toggle
						checked={draft.confirm}
						label={t("slot.confirm")}
						onChange={(confirm) => update({ confirm }, true)}
					/>
				</F.Row>
			)}

			{(kind === "shell" || kind === "keystroke") && <F.Hint>{t("slot.secretWarning")}</F.Hint>}

			{notice && (
				<F.Status
					$tone={notice.incomplete ? "plain" : "error"}
					role={notice.incomplete ? "status" : "alert"}
					data-role="slot-notice"
					data-incomplete={notice.incomplete}
				>
					{!notice.incomplete && <Icon name="triangle-alert" size={14} />}
					<span>{notice.text}</span>
				</F.Status>
			)}

			{notice && touched.current && (
				<F.QuietActions>
					<Button variant="quiet" data-role="slot-discard" onClick={discard}>
						{t("common.discardEdit")}
					</Button>
				</F.QuietActions>
			)}

			{saved && (
				<F.QuietActions>
					<Button variant="quiet" danger data-role="slot-clear" onClick={() => void clear()}>
						{t("slot.clear")}
					</Button>
				</F.QuietActions>
			)}
		</S.Slot>
	);
}
