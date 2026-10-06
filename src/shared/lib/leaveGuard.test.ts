// leaveGuard 의 테스트 — 모든 편집기가 저장을 끝내야 떠날 수 있다
import { expect, test } from "bun:test";
import { canLeave, registerLeaveGuard } from "./leaveGuard";

test("떠날 수 있는지는 걸린 편집기가 모두 답한다", async () => {
	expect(await canLeave()).toBe(true);

	const calls: string[] = [];
	const offSaved = registerLeaveGuard(async () => {
		calls.push("saved");
		return true;
	});
	const offRefused = registerLeaveGuard(async () => {
		calls.push("refused");
		return false;
	});
	expect(await canLeave()).toBe(false);
	// 하나가 막아도 나머지는 저장해 본다.
	expect(calls.sort()).toEqual(["refused", "saved"]);

	offRefused();
	expect(await canLeave()).toBe(true);
	offSaved();
});

test("저장하다 실패한 편집기는 떠나지 못하게 한다", async () => {
	const off = registerLeaveGuard(() => Promise.reject(new Error("ipc")));
	expect(await canLeave()).toBe(false);
	off();
});
