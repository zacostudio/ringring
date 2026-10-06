// 지금 떠 있는 toast 하나를 그린다 — 설정 창의 맨 아래 가운데
import styled from "@emotion/styled";
import { useSyncExternalStore } from "react";
import { dismissToast, toastStore } from "../lib/toast";
import { Icon } from "./Icon";

export function ToastHost() {
	const toast = useSyncExternalStore(toastStore.subscribe, toastStore.get);
	if (!toast) return null;
	return (
		<Root
			role={toast.tone === "error" ? "alert" : "status"}
			data-role="toast"
			onClick={() => dismissToast(toast.id)}
		>
			<Mark $tone={toast.tone}>
				<Icon name={toast.tone === "error" ? "triangle-alert" : "check"} size={14} />
			</Mark>
			<div>
				<Text>{toast.text}</Text>
				{toast.detail && <Detail>{toast.detail}</Detail>}
			</div>
		</Root>
	);
}

/* 떠 있는 면이다. 선 없이 더 밝은 면과 그림자로만 뜬다. */
const Root = styled.div`
    position: fixed;
    left: 50%;
    bottom: 20px;
    z-index: 10;
    display: flex;
    align-items: flex-start;
    gap: 8px;
    max-width: 440px;
    padding: 9px 14px 9px 11px;
    border-radius: 10px;
    background: var(--bg-float);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2), 0 1px 3px rgba(0, 0, 0, 0.14);
    transform: translateX(-50%);
`;

const Mark = styled.span<{ $tone: "error" | "info" }>`
    display: inline-flex;
    padding-top: 2px;
    color: ${(p) => (p.$tone === "error" ? "var(--danger)" : "var(--ok)")};

    & > svg {
        flex-shrink: 0;
    }
`;

const Text = styled.p`
    font-size: var(--text-body);
    font-weight: 500;
`;

const Detail = styled.p`
    margin-top: 2px;
    font-size: var(--text-meta);
    color: var(--text-2);
    white-space: pre-line;
`;
