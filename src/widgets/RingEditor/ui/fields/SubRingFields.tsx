// "하위 링" 동작의 입력 — 열 링을 고르거나 그 자리에서 새로 만든다. 고를 수 있는지는 Rust 가 정한다
import { useEffect, useState } from "react";
import { ringRefusalText, ringRepository } from "@/entities/Ring";
import type { RingAction, RingLimits, RingLinkChoices } from "@/entities/Ring";
import { useT } from "@/shared/i18n";
import { Button } from "@/shared/ui/Button";
import { Icon } from "@/shared/ui/Icon";
import * as F from "@/shared/ui/form.styles";
import * as S from "../RingEditor.styles";
import type { ActionChange } from "./actionChange";

interface SubRingFieldsProps {
	ringId: string;
	position: number;
	action: Extract<RingAction, { kind: "open_ring" }>;
	/** 저장된 칸의 이름. 이름을 고치면 링의 이름도 바뀌므로 목록을 다시 읽는다. */
	savedLabel: string;
	limits: RingLimits;
	onChange: ActionChange;
	/** 빈 하위 링을 만들어 이 칸에 잇는다. */
	onCreate: () => void;
	/** 그 링을 편집하러 간다. 돌아올 자리는 부른 쪽이 기억한다. */
	onEditRing: (ringId: string) => void;
}

export function SubRingFields({
	ringId,
	position,
	action,
	savedLabel,
	limits,
	onChange,
	onCreate,
	onEditRing
}: SubRingFieldsProps) {
	const t = useT();
	const [choices, setChoices] = useState<RingLinkChoices | null>(null);

	useEffect(() => {
		let alive = true;
		void ringRepository.linkCandidates(ringId, position).then(
			(found) => {
				if (alive) setChoices(found);
			},
			() => {
				if (alive) setChoices({ candidates: [], createBlocked: null });
			}
		);
		return () => {
			alive = false;
		};
	}, [ringId, position, savedLabel]);

	if (choices === null) return null;

	const linked = choices.candidates.find((candidate) => candidate.id === action.ring_id) ?? null;

	return (
		<F.Stack>
			<F.Hint>{t(choices.candidates.length === 0 ? "subRing.none" : "subRing.lead")}</F.Hint>
			{choices.candidates.length > 0 && (
				<S.ChoiceList>
					{choices.candidates.map((candidate) => {
						const selected = candidate.id === action.ring_id;
						const blocked = candidate.blocked !== null;
						return (
							<div key={candidate.id}>
								<S.Choice
									type="button"
									aria-pressed={selected}
									data-ring={candidate.id}
									$selected={selected}
									disabled={blocked}
									onClick={() =>
										onChange({ kind: "open_ring", ring_id: candidate.id }, true, {
											label: candidate.name
										})
									}
								>
									<Icon name={selected ? "circle-check" : "circle"} size={16} />
									<S.ChoiceName>{candidate.name}</S.ChoiceName>
									<S.ChoiceMeta>
										{candidate.filledSlots === 0
											? t("subRing.empty")
											: t("subRing.slots", { count: candidate.filledSlots })}
									</S.ChoiceMeta>
								</S.Choice>
								{blocked && (
									<S.ChoiceReason>{ringRefusalText(candidate.blocked, limits, t)}</S.ChoiceReason>
								)}
							</div>
						);
					})}
				</S.ChoiceList>
			)}
			<F.Actions>
				<Button data-role="sub-ring-create" disabled={choices.createBlocked !== null} onClick={onCreate}>
					<Icon name="plus" size={14} />
					{t("subRing.create")}
				</Button>
				{linked && (
					<Button variant="quiet" data-role="sub-ring-edit" onClick={() => onEditRing(linked.id)}>
						<Icon name="pencil" size={14} />
						{t("subRing.edit")}
					</Button>
				)}
			</F.Actions>
			{choices.createBlocked !== null && <F.Hint>{ringRefusalText(choices.createBlocked, limits, t)}</F.Hint>}
			{linked && linked.filledSlots === 0 && <F.Hint>{t("subRing.emptyHint")}</F.Hint>}
		</F.Stack>
	);
}
