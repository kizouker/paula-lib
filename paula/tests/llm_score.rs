//! The model writes or revises a score (issue #63). Fake model only: no key, no network.

use paula::llm::score::{changed_lines, extract_toml, write_score, ScoreError};
use paula::llm::{Message, Model, ModelError, MAX_RETRIES};
use paula::score::kit::demo_kit;

/// Answers in turn, and remembers what it was sent.
struct Fake {
    answers: Vec<String>,
    seen: Vec<Vec<Message>>,
}

impl Model for Fake {
    fn complete(&mut self, _system: &str, messages: &[Message]) -> Result<String, ModelError> {
        self.seen.push(messages.to_vec());
        if self.answers.is_empty() {
            return Err(ModelError::Network("no more answers".into()));
        }
        Ok(self.answers.remove(0))
    }
}

fn fake(answers: &[&str]) -> Fake {
    Fake { answers: answers.iter().map(|a| a.to_string()).collect(), seen: vec![] }
}

const GOOD: &str = "[style]\nname = \"zouk\"\nkey = \"A aeolian\"\nbpm = 86\n[progression]\nv = \"Am | Dm | Em | Am\"\n[role.kick]\nchannel = 1\nsample = 1\ngroove = \"x.......x.......\"\n[role.chick]\nchannel = 2\nsample = 2\ngroove = \"....x.x.....x.x.\"\ndelay = 2\n[song]\nform = \"v 4\"\n";

#[test]
fn a_clean_answer_becomes_a_compiled_proposal() {
    let mut m = fake(&[&format!("Here you go:\n```toml\n{GOOD}```\n")]);
    let p = write_score(&mut m, "Brazilian zouk, 86 BPM", None, &demo_kit(), false, &mut |_| {}).unwrap();
    assert_eq!(p.text, GOOD, "the fence and the prose are removed");
    assert!(p.module.song_len >= 1);
    let first = &m.seen[0][0].content;
    assert!(first.contains("Kit samples") && first.contains("1 kick") && first.contains("No score yet"), "{first}");
}

#[test]
fn lint_errors_go_back_to_the_model_and_the_fix_is_used() {
    let bad = GOOD.replace("delay = 2", "delay = 9");
    let mut m = fake(&[&bad, GOOD]);
    let p = write_score(&mut m, "zouk", None, &demo_kit(), false, &mut |_| {}).unwrap();
    assert_eq!(p.text, GOOD);
    let repair = &m.seen[1].last().unwrap().content;
    assert!(repair.contains("delay") && repair.contains("corrected score"), "{repair}");
}

#[test]
fn gives_up_after_the_retries_with_the_last_problems() {
    let bad = GOOD.replace("channel = 2", "channel = 1");
    let answers: Vec<&str> = vec![bad.as_str(); MAX_RETRIES + 1];
    let mut m = fake(&answers);
    match write_score(&mut m, "zouk", None, &demo_kit(), false, &mut |_| {}) {
        Err(ScoreError::Invalid { attempts, last }) => {
            assert_eq!(attempts, MAX_RETRIES + 1);
            assert!(last.contains("channel"), "{last}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn revising_sends_the_current_score_and_shows_what_changed() {
    let new = GOOD.replace("bpm = 86", "bpm = 90");
    let mut m = fake(&[&new]);
    let p = write_score(&mut m, "a bit faster", Some(GOOD), &demo_kit(), false, &mut |_| {}).unwrap();
    assert!(m.seen[0][0].content.contains("Current score") && m.seen[0][0].content.contains("bpm = 86"));
    assert_eq!(changed_lines(GOOD, &p.text), vec!["-bpm = 86".to_string(), "+bpm = 90".to_string()]);
}

#[test]
fn empty_requests_do_not_call_the_model() {
    let mut m = fake(&[]);
    assert_eq!(write_score(&mut m, "  ", None, &demo_kit(), false, &mut |_| {}).unwrap_err(), ScoreError::EmptyRequest);
    assert!(m.seen.is_empty());
}

#[test]
fn extract_takes_the_first_fence_or_the_whole_text() {
    assert_eq!(extract_toml("```\na = 1\n```"), "a = 1\n");
    assert_eq!(extract_toml("a = 1"), "a = 1\n");
}

#[test]
fn the_system_prompt_carries_the_style_rules() {
    let s = paula::llm::score::SYSTEM_PROMPT;
    assert!(s.contains("never Antillean zouk") && s.contains("no 808") && s.contains("delay"));
}
