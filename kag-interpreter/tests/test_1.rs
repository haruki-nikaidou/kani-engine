//! 「1 — 新しい人生を選べ」統合テスト
//!
//! 女神が13の質問を通じてプレイヤーの転生先を決定するシナリオ。
//!
//! # スコアリング（1問あたり）
//!
//! | 選択肢 | index | karma | courage | kindness |
//! |--------|-------|-------|---------|----------|
//! | A      | 0     | +3    | +1      | +0       |
//! | B      | 1     | +1    | +3      | +1       |
//! | C      | 2     | +0    | +1      | +3       |
//!
//! # エンディング判定（Rhaiがスクリプト内で実行、優先順位順）
//!
//! | エンディング            | 条件                                             |
//! |-------------------------|--------------------------------------------------|
//! | 偉大なるプログラマー     | karma ≤ 8 AND courage ≥ 20 AND kindness ≥ 20    |
//! | 怪盗                    | karma ≥ 20 AND courage ≥ 20 AND kindness ≥ 9    |
//! | ならず者                | karma ≥ 20                                       |
//! | エクストリームスポーツ  | courage ≥ 20                                     |
//! | 普通人（デフォルト）    | それ以外                                          |
//!
//! # テスト戦略（13問への回答パターン）
//!
//! - 普通人:           3×A + 3×B + 7×C → karma=12, courage=19, kindness=24
//! - 怪盗:             6×A + 6×B + 1×C → karma=24, courage=25, kindness=9
//! - 偉大なるプログラマー: 7×B + 6×C   → karma=7,  courage=27, kindness=25
//! - ならず者:         13×A             → karma=39, courage=13
//! - エクストリームスポーツ: 13×B       → karma=13, courage=39, kindness=13

use kag_interpreter::{HostEvent, KagEvent, KagInterpreter};
use tokio::task::LocalSet;

static SCRIPT: &str = include_str!("1-choose-your-new-life/game.ks");

// ─── Shared helpers ───────────────────────────────────────────────────────────

/// Drive the interpreter to completion, auto-clicking through all wait points
/// and consuming choices from the provided slice in order.
async fn drive(choices: &[usize]) -> Vec<KagEvent> {
    let (mut handle, _task, _diags) =
        KagInterpreter::spawn_from_source(SCRIPT, "game.ks").unwrap();

    let mut events: Vec<KagEvent> = Vec::new();
    let mut ci = 0usize;

    loop {
        match handle.recv().await {
            Some(KagEvent::End) | None => break,
            Some(KagEvent::WaitForClick { .. }) => {
                handle.send(HostEvent::Clicked).await.unwrap();
            }
            Some(KagEvent::BeginChoices(_)) => {
                let idx = choices[ci];
                ci += 1;
                handle.send(HostEvent::ChoiceSelected(idx)).await.unwrap();
            }
            Some(e) => events.push(e),
        }
    }

    events
}

fn has_text(events: &[KagEvent], needle: &str) -> bool {
    events
        .iter()
        .any(|e| matches!(e, KagEvent::DisplayText { text, .. } if text.contains(needle)))
}

// ─── Ending 1: 普通人 ─────────────────────────────────────────────────────────
// 3×A + 3×B + 7×C → karma=12 (middle), courage=19 (middle) → normie

#[tokio::test(flavor = "current_thread")]
async fn test_ending_normie() {
    LocalSet::new()
        .run_until(async {
            #[rustfmt::skip]
            let choices: &[usize] = &[
                0, 0, 0,             // Q1–Q3:  A
                1, 1, 1,             // Q4–Q6:  B
                2, 2, 2, 2, 2, 2, 2, // Q7–Q13: C
            ];
            let events = drive(choices).await;

            assert!(has_text(&events, "普通人"), "normie ending missing");
            assert!(!has_text(&events, "怪盗"), "master-thief must not appear");
            assert!(!has_text(&events, "偉大なるプログラマー"), "grand-prog must not appear");
        })
        .await;
}

// ─── Ending 2: 怪盗 ───────────────────────────────────────────────────────────
// 6×A + 6×B + 1×C → karma=24 (≥20), courage=25 (≥20), kindness=9 (≥9)

#[tokio::test(flavor = "current_thread")]
async fn test_ending_master_thief() {
    LocalSet::new()
        .run_until(async {
            #[rustfmt::skip]
            let choices: &[usize] = &[
                0, 0, 0, 0, 0, 0, // Q1–Q6:  A
                1, 1, 1, 1, 1, 1, // Q7–Q12: B
                2,                  // Q13:    C
            ];
            let events = drive(choices).await;

            assert!(has_text(&events, "怪盗"), "master-thief ending missing");
            assert!(!has_text(&events, "普通人"), "normie must not appear");
            assert!(!has_text(&events, "偉大なるプログラマー"), "grand-prog must not appear");
        })
        .await;
}

// ─── Ending 3: 偉大なるプログラマー ─────────────────────────────────────────────
// 7×B + 6×C → karma=7 (≤8), courage=27 (≥20), kindness=25 (≥20)

#[tokio::test(flavor = "current_thread")]
async fn test_ending_grand_programmer() {
    LocalSet::new()
        .run_until(async {
            #[rustfmt::skip]
            let choices: &[usize] = &[
                1, 1, 1, 1, 1, 1, 1, // Q1–Q7:  B
                2, 2, 2, 2, 2, 2,     // Q8–Q13: C
            ];
            let events = drive(choices).await;

            assert!(has_text(&events, "偉大なるプログラマー"), "grand-programmer ending missing");
            assert!(!has_text(&events, "怪盗"), "master-thief must not appear");
            assert!(!has_text(&events, "普通人"), "normie must not appear");
        })
        .await;
}

// ─── Ending 4: ならず者 ───────────────────────────────────────────────────────
// 13×A → karma=39 (≥20), courage=13 (<20) → scumbag (not master thief)

#[tokio::test(flavor = "current_thread")]
async fn test_ending_scumbag() {
    LocalSet::new()
        .run_until(async {
            let choices: &[usize] = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            let events = drive(choices).await;

            assert!(has_text(&events, "ならず者"), "scumbag ending missing");
            assert!(!has_text(&events, "怪盗"), "master-thief must not appear");
            assert!(!has_text(&events, "エクストリームスポーツ"), "extreme-sports must not appear");
        })
        .await;
}

// ─── Ending 5: エクストリームスポーツ選手 ────────────────────────────────────────
// 13×B → karma=13 (<20), courage=39 (≥20), kindness=13 (<20)

#[tokio::test(flavor = "current_thread")]
async fn test_ending_extreme_sports() {
    LocalSet::new()
        .run_until(async {
            let choices: &[usize] = &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
            let events = drive(choices).await;

            assert!(has_text(&events, "エクストリームスポーツ"), "extreme-sports ending missing");
            assert!(!has_text(&events, "ならず者"), "scumbag must not appear");
            assert!(!has_text(&events, "普通人"), "normie must not appear");
        })
        .await;
}
