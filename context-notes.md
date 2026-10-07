# context-notes

작업 중에 내린 결정과 그 이유를 쌓는다. 새 항목은 아래에 붙인다.

## 2026-10-07 — 일반 단축키와 빠른 단축키를 나눈다 (설계 승인 대기)

설계 문서: tome HTML 노트 "링링 — 일반 단축키와 빠른 단축키 설계"
(id `49b87b64-a1d3-4658-8695-3f84fd053f34`, 그룹 zacostudio).

### 문제

단축키를 짧게 누르는 순간 마우스가 같이 움직이면, 링이 보이기 전에 칸이 실행됐다.

- 키를 뗄 때 커서가 dead zone(30px, `geometry.rs:8`) 밖이면 누른 시간과 상관없이 실행한다
  (`controller.rs:246-252`). 220ms 안의 flick 도 일부러 실행하게 만들었다 (테스트 `controller.rs:994`).
- 링의 appear animation 은 90ms 다 (`RingPage.styles.ts:31`).

### 결정 (사용자)

- 링마다 단축키를 두 칸으로 나눈다.
  - 일반 단축키: 링만 띄운다 (`Mode::Open`). 키를 떼도 실행하지 않는다.
  - 빠른 단축키: F1~F24 + 수식키(⌃⌥⌘⇧) 선택. 지금의 hold 동작.
- F1~F24 전부 허용한다. 수식키 없는 F1~F12 는 막지 않고 안내만 한다.
- 기존 단축키는 일반 단축키가 된다. hold 실행이 사라져도 괜찮다. 빠른 단축키로 자동으로 옮기지 않는다.
- tome 에는 적용하지 않는다. 사용자가 직접 한다.

### 버린 방법

- 220ms 안에 떼면 실행하지 않기 — flick 이 같이 사라진다.
- 키 종류로 자동 판단(칸 하나) — 사용자가 두 칸(B안)을 원했다.
- dead zone 넓히기 — 움직임이 조금 더 크면 다시 난다.

### 구현할 때 조심할 것

- 일반 단축키를 누르고 있으면 OS 가 누름을 반복해 보낸다 (`controller.rs:401`). Open 에서 같은 단축키는
  닫기이므로 (`controller.rs:408-411`) 그대로 두면 첫 반복에 닫힌다. 뗀 것을 받기 전의 누름은 무시한다.
- macOS 에서 수식키 없는 F1~F12 가 fn 없이 들어오는지는 확인하지 않았다 (추정: fn 이 필요).
- 백업 형식 struct 에 `deny_unknown_fields` 가 없다 — 옛 앱이 새 칸 `quickShortcut` 을 무시하고 읽는다.
  그래서 `FORMAT_VERSION` 은 1 로 둔다.

### 보류

- 하위 링을 만드는 흐름 정리 — 사용자가 "지켜보겠다"고 했다.

## 2026-10-07 — dev-agent 실제 키 입력으로 확인한 결과

방법: `RINGRING_DEV_AGENT=1 RINGRING_DEV_QUIET=1 RINGRING_DEV_GLOBAL_SHORTCUTS=1 bun run tauri:dev -- --features dev-agent`.
이 기기에는 링링 데이터가 없어서 새 dev DB 에서 시작했다 (로그: `schema v1 created` → `schema v2 — quick_shortcut added`).
링 A 의 1번 칸(12시) = 하위 링 B. 칸이 "실행"되면 B 로 들어가므로 `/ring/state` 의 depth 로 판정한다.
키는 `/gate/press` (⌃⌥⇧F18, CGEvent). gate 가 빠른 단축키 칸도 보도록 `gate.rs` 의 `combo_registered` 를 넓혔다.

| # | 단축키 | 입력 | 결과 |
|---|---|---|---|
| T1 | 일반 | 600ms, 누른 채 위로 90px | open, depth 1 — **실행 안 됨** (처음 문제 해결) |
| T2 | 일반 | 2000ms | open, depth 1 |
| T3 | 일반 | T2 뒤 다시 누름 | 닫힘 |
| T4 | 일반 | 수식키 먼저 떼기 → 다시 누름 | open → 닫힘 (polling 이 뗌을 잡음) |
| T5 | 빠른 | 600ms, 위로 90px | depth 2 — 칸 실행 |
| T6 | 빠른 | 80ms, 가운데 | open, depth 1 (tap) |
| T7 | 빠른 | 600ms, 가운데 | 닫힘 |

거절도 실제 command 로 확인했다: 빠른 단축키에 `Alt+Space` → `shortcut_needs_function_key`, 같은 링의 일반과 같은
조합을 빠른 단축키로 → `shortcut_taken`.

**확인하지 못한 것:** CGEvent 로 만든 key-down 이 OS 의 key repeat 를 만드는지 모른다 (추정: 만들지 않는다 —
일반 단축키의 반복 누름은 로그를 남기지 않아 T2 에서 반복이 왔는지 볼 수 없었다). 그래서 T2 는 "누르고 있는 동안
반복 누름이 와도 닫히지 않는다" 를 증명하지 않는다. 그 동작은 단위 테스트(`repeated_presses_of_a_held_key_are_ignored`)
로만 고정돼 있다. 실제 키보드로 눌러 봐야 한다.

## 2026-10-07 — 구현 마무리

커밋: `07c823f` DB·모델 → `79ddbcf` 규칙·등록 → `4d7a449` 누름·뗌 → `a4e2e7f` 백업 → `659f920` dev gate → `e7194c0` 설정 화면.
끝난 뒤: `cargo test --features dev-agent` 182 통과 (시작 전 166), `bun test src/` 37 통과, typecheck·lint 통과.

- 일반 단축키로 연 링을 같은 링의 **빠른** 단축키로 다시 눌러도 닫힌다. 같은 링인지만 보고 종류는 보지 않는다
  (`decide_press`). 일부러 단순하게 두었다.
- 커밋 작성자는 zacostudio 계정이어야 한다. 첫 커밋이 전역 git 설정(회사 계정)으로 들어가 amend 했고,
  이 저장소에 repo-local `user.name`/`user.email` 을 넣었다.
