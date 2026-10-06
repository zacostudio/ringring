// 링이 하나도 없을 때의 본문 — 무엇을 하는 앱인지 한 문장과, 시작하는 두 가지 길
import styled from "@emotion/styled";
import { RingDial } from "@/entities/Ring";
import type { RingSlotView } from "@/entities/Ring";
import { useT } from "@/shared/i18n";
import { Button } from "@/shared/ui/Button";
import * as F from "@/shared/ui/form.styles";

interface WelcomeProps {
	onCreateStarter: () => void;
	onCreateEmpty: () => void;
}

/** 그림에 보이는 칸. 이름은 사용자 언어로 채운다. */
const SAMPLE: Array<{ position: number; icon: string; labelKey: string }> = [
	{ position: 0, icon: "folder", labelKey: "welcome.sample.finder" },
	{ position: 1, icon: "compass", labelKey: "welcome.sample.browser" },
	{ position: 2, icon: "square-terminal", labelKey: "welcome.sample.terminal" },
	{ position: 4, icon: "download", labelKey: "welcome.sample.downloads" },
	{ position: 5, icon: "settings", labelKey: "welcome.sample.settings" }
];

export function Welcome({ onCreateStarter, onCreateEmpty }: WelcomeProps) {
	const t = useT();
	const slots: RingSlotView[] = SAMPLE.map((sample) => ({
		position: sample.position,
		icon: sample.icon,
		label: t(sample.labelKey),
		opensRing: false
	}));
	return (
		<Root data-role="welcome">
			<Text>
				<F.PageTitle>{t("welcome.title")}</F.PageTitle>
				<Lead>{t("welcome.lead")}</Lead>
				<Steps>
					<li>{t("welcome.step1")}</li>
					<li>{t("welcome.step2")}</li>
					<li>{t("welcome.step3")}</li>
				</Steps>
				<F.Actions>
					<Button variant="primary" data-role="create-starter" onClick={onCreateStarter}>
						{t("welcome.starter")}
					</Button>
					<Button data-role="create-empty" onClick={onCreateEmpty}>
						{t("welcome.empty")}
					</Button>
				</F.Actions>
				<F.Hint>{t("welcome.starterHint")}</F.Hint>
			</Text>
			<RingDial slotCount={6} slots={slots} hovered={1} size={232} />
		</Root>
	);
}

const Root = styled.div`
    display: flex;
    align-items: center;
    gap: 48px;
`;

const Text = styled.div`
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 420px;
`;

const Lead = styled.p`
    color: var(--text-2);
`;

const Steps = styled.ol`
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding-left: 18px;

    li::marker {
        font-weight: 600;
        color: var(--text-3);
    }
`;
