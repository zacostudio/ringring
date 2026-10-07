# checklist — 일반 단축키와 빠른 단축키

설계: tome HTML 노트 "링링 — 일반 단축키와 빠른 단축키 설계" (id `49b87b64-a1d3-4658-8695-3f84fd053f34`).
결정과 이유: `context-notes.md`.

기준 (2026-10-07, 시작 전): `cargo test` 166 통과, `bun test src/` 37 통과, typecheck·lint 통과.

## 공통 규칙

- 단축키 종류는 `ShortcutKind { Normal, Quick }` 하나로 부른다. serde 는 `"normal"` / `"quick"`.
- 빠른 단축키의 키는 F1~F24 만. 수식키는 아무것이나, 없어도 된다. 아니면 `Refusal::ShortcutNeedsFunctionKey` (`shortcut_needs_function_key`).
- 중복 검사는 모든 링의 두 칸을 함께 본다. 자기 자신(같은 링, 같은 종류)만 뺀다.
- 일반 단축키로 연 링은 `Mode::Open`. 키를 떼기 전의 반복 누름은 무시한다.

## 1. 모델과 DB (Rust)

- [x] `model.rs` — `ShortcutKind`, `Ring.quick_shortcut: Option<String>` (`#[serde(default)]`), `Refusal::ShortcutNeedsFunctionKey`
- [x] `store/mod.rs` — schema v2: `ALTER TABLE rings ADD COLUMN quick_shortcut TEXT` + `CREATE UNIQUE INDEX`
      (SQLite 는 `ADD COLUMN ... UNIQUE` 를 받지 않는다). `SCHEMA_VERSION = 2`
- [x] `store/rings.rs` — SELECT 에 칸 추가, `set_shortcut(conn, id, kind, combo)`, `import` 가 두 칸을 쓴다
- [x] 테스트: v1 DB → v2 로 올라가고 기존 단축키 유지 · 빠른 단축키 저장/지우기 · 같은 빠른 단축키 두 링 거절
- [x] `cargo test` → 커밋

## 2. 등록과 검사 (shortcuts.rs, commands)

- [x] `parse_for_quick(combo) -> Result<Shortcut, Refusal>`
- [x] `conflict(ring_id, kind, wanted, rings)` — 두 칸 모두 본다
- [x] `plan` / `run_pass` / `Registrar::register` / `REFUSED` / `change_shortcut` 에 kind 를 싣는다
- [x] `ring_set_shortcut(ring_id, kind, shortcut)` command, `AppStateView.refused_quick_shortcuts`
- [x] 테스트: `F1`·`Ctrl+F5`·`Shift+F13` 받음, `Ctrl+KeyA`·`Space` 거절 · 다른 링의 일반↔빠른 충돌 · 같은 링의 두 칸 충돌 · plan 이 두 조합을 건다 · 거절이 kind 별로 남는다
- [x] `cargo test` → 커밋

## 3. 누름과 뗌 (controller.rs)

- [x] `pressed(app, ring_id, kind, shortcut)` — 판단은 순수 함수 `decide_press` 로 빼서 테스트한다
- [x] Session 에 `trigger_down: bool`. 일반으로 열면 true, Released 또는 polling 의 키 상태로 false
- [x] `released(app, ring_id, kind)` — Normal 이면 `trigger_down = false` 만, Quick 이면 지금처럼 `finish_hold`
- [x] 테스트: 일반으로 연 링에 반복 누름 → 무시 · 뗀 뒤 누름 → 닫기 · 빠른 단축키의 반복 누름 → 지금처럼 무시
- [x] `cargo test` → 커밋

## 4. 백업 (transfer.rs)

- [x] `ImportRing.quick_shortcut` (`#[serde(default)]`), 가져올 때 `parse_for_quick` + 두 칸 공통 `taken`
- [x] 테스트: 옛 파일(칸 없음) 가져오기 · 중복 빠른 단축키는 버리고 `dropped_shortcuts` 에 센다 · 내보내기에 `quickShortcut` 이 들어간다
- [x] `cargo test` → 커밋

## 5. 설정 화면 (프런트)

- [x] `Ring.quickShortcut`, `AppState.refusedQuickShortcuts`, `setShortcut(ringId, kind, shortcut)`
- [x] `RingHeader` — 두 번째 `ShortcutField`, 칸마다 안내·거절 문구
- [x] i18n ko/en/ja (+ `other:` Windows 문구) — 새 키와 `ring.shortcutNote` 수정, `refusal.shortcut_needs_function_key`
- [x] `bun test src/`, typecheck, lint, `cargo test` → 커밋

## 6. 문서

- [x] README "쓰는 법" 표, CHANGELOG [Unreleased] (기존 단축키의 hold 실행이 사라짐을 적는다)
- [x] context-notes 갱신 → 커밋

## 7. 실제로 눌러 보기 (사용자 확인 필요)

전역 단축키는 개발 빌드에서 등록되지 않는다 (`RINGRING_DEV_GLOBAL_SHORTCUTS=1` 이면 등록, 설치된 앱과 조합을 다툰다).

- [x] 일반 단축키 + 마우스 크게 움직이며 떼기 → 실행 없이 링이 남는다 (처음 문제) — dev-agent 실제 키 입력으로 확인 (context-notes T1)
- [ ] 일반 단축키 2초 누르기 → 링이 닫히지 않는다 — dev-agent 로는 key repeat 를 만들 수 없어 실제 키보드 확인 필요
- [x] 빠른 단축키 flick → 실행된다 — dev-agent 로 확인 (T5)
- [ ] macOS 내장 키보드에서 F5 와 fn+F5
- [ ] 일반 단축키로 연 링에서 키 입력 칸 실행
