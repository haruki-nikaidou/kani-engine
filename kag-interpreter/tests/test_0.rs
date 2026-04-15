//! Integration tests for "0 — Introduction to Sorting Algorithms".
//!
//! Two characters: Me and Senpai.
//!
//! # Script branches
//!
//! 1. Senpai asks whether the player knows computational complexity.
//!    - Yes (index 0) → skip straight to insertion sort (step 4).
//!    - No  (index 1) → Big O slide intro (step 3), then insertion sort.
//! 4. Senpai introduces insertion sort; Me asks if computers actually sort that way.
//! 5. Senpai asks which kind of algorithm interests the player.
//!    - Stable one   (index 0) → step 6: merge sort → step 9.
//!    - Slightly fast (index 1) → step 7: quicksort  → step 9.
//!    - Crazy one    (index 2) → step 8: bogo sort joke, loops back to step 5.
//! 9. Me asks which algorithm Rust uses; Senpai answers "Voultapher/driftsort".
//!
//! All six possible routes are covered:
//!   (yes | no) × (merge | quick | bogo → merge)

use kag_interpreter::{HostEvent, KagEvent, KagInterpreter};
use tokio::task::LocalSet;

static SCRIPT: &str = include_str!("0-intorudction-to-sorting-algorithm/game.ks");

// ─── Shared helpers ───────────────────────────────────────────────────────────

/// Drive the interpreter to completion, auto-clicking through all wait points
/// and consuming choices from the provided slice in order.
async fn drive(choices: &[usize]) -> Vec<KagEvent> {
    let (mut handle, _task, _diags) = KagInterpreter::spawn_from_source(SCRIPT, "game.ks").unwrap();

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

// ─── Route 1: yes → merge sort ───────────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn test_yes_complexity_merge_sort() {
    LocalSet::new()
        .run_until(async {
            // choice 0 = "Yes, I am!" — choice 0 = "A stable one."
            let events = drive(&[0, 0]).await;

            assert!(
                !has_text(&events, "Big O"),
                "Big O intro should be skipped when player knows complexity"
            );
            assert!(
                has_text(&events, "insertion sort"),
                "insertion sort must appear"
            );
            assert!(
                has_text(&events, "divide-and-conquer"),
                "merge sort missing"
            );
            assert!(has_text(&events, "stable"), "merge sort stability missing");
            assert!(!has_text(&events, "chaos"), "bogo sort must not appear");
            assert!(has_text(&events, "driftsort"), "Rust answer missing");
        })
        .await;
}

// ─── Route 2: yes → quicksort ────────────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn test_yes_complexity_quicksort() {
    LocalSet::new()
        .run_until(async {
            let events = drive(&[0, 1]).await;

            assert!(!has_text(&events, "Big O"), "Big O intro should be skipped");
            assert!(
                has_text(&events, "insertion sort"),
                "insertion sort must appear"
            );
            assert!(has_text(&events, "pivot"), "quicksort pivot missing");
            assert!(
                has_text(&events, "not stable"),
                "quicksort stability caveat missing"
            );
            assert!(
                !has_text(&events, "divide-and-conquer"),
                "merge sort must not appear"
            );
            assert!(has_text(&events, "driftsort"), "Rust answer missing");
        })
        .await;
}

// ─── Route 3: yes → bogo sort (loop), then merge sort ────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn test_yes_complexity_bogo_then_merge_sort() {
    LocalSet::new()
        .run_until(async {
            // choice 0 = yes; choice 2 = bogo (loops back); choice 0 = merge
            let events = drive(&[0, 2, 0]).await;

            assert!(!has_text(&events, "Big O"), "Big O intro should be skipped");
            assert!(
                has_text(&events, "insertion sort"),
                "insertion sort must appear"
            );
            assert!(has_text(&events, "chaos"), "bogo sort (chaos) missing");
            assert!(
                has_text(&events, "The joke ends here"),
                "bogo loop-back phrase missing"
            );
            assert!(
                has_text(&events, "divide-and-conquer"),
                "merge sort after bogo missing"
            );
            assert!(has_text(&events, "driftsort"), "Rust answer missing");
        })
        .await;
}

// ─── Route 4: no → merge sort ────────────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn test_no_complexity_merge_sort() {
    LocalSet::new()
        .run_until(async {
            // choice 1 = "No, not really." — choice 0 = "A stable one."
            let events = drive(&[1, 0]).await;

            assert!(
                has_text(&events, "Big O"),
                "Big O intro must appear when player does not know it"
            );
            assert!(
                has_text(&events, "linearithmic"),
                "Big O complexity types missing"
            );
            assert!(
                has_text(&events, "insertion sort"),
                "insertion sort must appear"
            );
            assert!(
                has_text(&events, "divide-and-conquer"),
                "merge sort missing"
            );
            assert!(has_text(&events, "driftsort"), "Rust answer missing");
        })
        .await;
}

// ─── Route 5: no → quicksort ─────────────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn test_no_complexity_quicksort() {
    LocalSet::new()
        .run_until(async {
            let events = drive(&[1, 1]).await;

            assert!(has_text(&events, "Big O"), "Big O intro must appear");
            assert!(
                has_text(&events, "insertion sort"),
                "insertion sort must appear"
            );
            assert!(has_text(&events, "pivot"), "quicksort pivot missing");
            assert!(
                !has_text(&events, "divide-and-conquer"),
                "merge sort must not appear"
            );
            assert!(has_text(&events, "driftsort"), "Rust answer missing");
        })
        .await;
}

// ─── Route 6: no → bogo sort (loop), then merge sort ─────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn test_no_complexity_bogo_then_merge_sort() {
    LocalSet::new()
        .run_until(async {
            // choice 1 = no; choice 2 = bogo (loops back); choice 0 = merge
            let events = drive(&[1, 2, 0]).await;

            assert!(has_text(&events, "Big O"), "Big O intro must appear");
            assert!(
                has_text(&events, "insertion sort"),
                "insertion sort must appear"
            );
            assert!(has_text(&events, "chaos"), "bogo sort (chaos) missing");
            assert!(
                has_text(&events, "The joke ends here"),
                "bogo loop-back phrase missing"
            );
            assert!(
                has_text(&events, "divide-and-conquer"),
                "merge sort after bogo missing"
            );
            assert!(has_text(&events, "driftsort"), "Rust answer missing");
        })
        .await;
}
