// 링 하나의 편집기 — 링 그림이 곧 칸 목록이다. 그림에서 칸을 고르고 오른쪽에서 그 칸을 고친다
import { useState } from "react";
import type { AppState } from "@/entities/AppState";
import { RingDial, ringFailureText, ringRepository } from "@/entities/Ring";
import type { Ring, RingLimits } from "@/entities/Ring";
import { useT } from "@/shared/i18n";
import { showToast } from "@/shared/lib/toast";
import { Button } from "@/shared/ui/Button";
import { Icon } from "@/shared/ui/Icon";
import { Segmented } from "@/shared/ui/Segmented";
import * as F from "@/shared/ui/form.styles";
import * as S from "./RingEditor.styles";
import { RingHeader } from "./RingHeader";
import { SlotEditor } from "./SlotEditor";

interface RingEditorProps {
	ring: Ring;
	limits: RingLimits;
	appState: AppState;
	/** 고른 칸의 자리. */
	position: number;
	onSelectPosition: (position: number) => void;
	/** Rust 가 돌려준 새 링. */
	onRingChanged: (ring: Ring) => void;
	onReload: () => Promise<void>;
	/** 칸이 여는 하위 링을 편집하러 간다. */
	onEditRing: (ringId: string) => void;
	onOpenPermissions: () => void;
	/** 하위 링을 편집하러 들어왔을 때, 돌아갈 곳의 이름과 돌아가는 일. */
	back: { label: string; go: () => void } | null;
}

export function RingEditor({
	ring,
	limits,
	appState,
	position,
	onSelectPosition,
	onRingChanged,
	onReload,
	onEditRing,
	onOpenPermissions,
	back
}: RingEditorProps) {
	const t = useT();
	// 칸 수를 줄이면 칸이 지워질 때, 줄이려는 칸 수. 사용자의 답을 기다린다.
	const [shrinkTo, setShrinkTo] = useState<number | null>(null);
	const selected = Math.min(position, ring.slotCount - 1);

	const saveCount = async (slotCount: number) => {
		setShrinkTo(null);
		try {
			onRingChanged(await ringRepository.save(ring.id, ring.name, slotCount));
		} catch (error) {
			showToast(ringFailureText(error, limits, t, "common.saveFailed"));
		}
	};

	const changeCount = (slotCount: number) => {
		if (slotCount === ring.slotCount) {
			setShrinkTo(null);
			return;
		}
		// 넘치는 칸은 지워진다. 지워질 칸이 있으면 먼저 묻는다.
		if (ring.slots.some((slot) => slot.position >= slotCount)) setShrinkTo(slotCount);
		else void saveCount(slotCount);
	};

	const counts = Array.from({ length: limits.maxSlots - limits.minSlots + 1 }, (_, index) => limits.minSlots + index);
	const dropped = shrinkTo === null ? 0 : ring.slots.filter((slot) => slot.position >= shrinkTo).length;

	return (
		<S.Root data-role="ring-editor" data-ring={ring.id}>
			{back && (
				<S.BackRow>
					<Button variant="quiet" data-role="ring-back" onClick={back.go}>
						<Icon name="arrow-left" size={14} />
						{back.label}
					</Button>
				</S.BackRow>
			)}
			<RingHeader
				ring={ring}
				limits={limits}
				appState={appState}
				onRingChanged={onRingChanged}
				onReload={onReload}
			/>
			<S.Body>
				<S.DialColumn>
					<RingDial
						slotCount={ring.slotCount}
						slots={ring.slots.map((slot) => ({
							position: slot.position,
							label: slot.label,
							icon: slot.icon,
							opensRing: slot.action.kind === "open_ring"
						}))}
						hovered={selected}
						showEmpty
						onSlotClick={onSelectPosition}
					/>
					<S.CountRow>
						<F.Label>{t("ring.slotCount")}</F.Label>
						<Segmented
							label={t("ring.slotCount")}
							options={counts.map((count) => ({ value: count, label: String(count) }))}
							value={shrinkTo ?? ring.slotCount}
							onChange={changeCount}
						/>
					</S.CountRow>
					{shrinkTo !== null && (
						<S.Confirming data-role="shrink-confirm">
							<span>{t("ring.shrinkQuestion", { count: shrinkTo, dropped })}</span>
							<F.Actions>
								<Button data-role="shrink-yes" onClick={() => void saveCount(shrinkTo)}>
									{t("ring.shrink")}
								</Button>
								<Button variant="quiet" onClick={() => setShrinkTo(null)}>
									{t("common.cancel")}
								</Button>
							</F.Actions>
						</S.Confirming>
					)}
				</S.DialColumn>
				<SlotEditor
					key={`${ring.id}:${selected}`}
					ring={ring}
					position={selected}
					limits={limits}
					onRingChanged={onRingChanged}
					onReload={onReload}
					onSelectPosition={onSelectPosition}
					onEditRing={onEditRing}
					onOpenPermissions={onOpenPermissions}
				/>
			</S.Body>
		</S.Root>
	);
}
