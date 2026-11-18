//! Backend tests: prompt builder, answer parser, repair loop, proposals, key lookup.
//! No network, no API key, no audio device.

use paula::llm::key::{dotenv_value, find_key, pick_model, DEFAULT_MODEL};
use paula::llm::parse::{parse_answer, ParseError};
use paula::llm::prompt::{build_request, Message, Role, Tempo, SYSTEM_PROMPT};
use paula::llm::{compose, ComposeError, Model, ModelError, MAX_RETRIES};
use paula::modfile::{Cell, Module, NUM_CHANNELS, NUM_ROWS};
use paula::rules::Problem;
use paula::session::{Proposal, Session, SessionError};

/// Module with sample 1 "kick" and sample 2 "snare" loaded, one empty pattern.
fn module() -> Module {
    let mut m = Module::empty();
    m.samples[0].data = vec![1, 2, 3, 4];
    m.samples[0].name[..4].copy_from_slice(b"kick");
    m.samples[1].data = vec![5, 6, 7, 8];
    m.samples[1].name[..5].copy_from_slice(b"snare");
    m
}

const GOOD: &str = r#"[
  {"row": 0, "ch": 0, "note": "C-2", "sample": 1, "fx": "..."},
  {"row": 4, "ch": 1, "note": "D-2", "sample": 2, "fx": "C20"},
  {"row": 8, "ch": 0, "note": "---", "sample": 0, "fx": "..."}
]"#;

// ---- prompt builder ----

#[test]
fn request_prompt_matches_golden_text() {
    let got = build_request(&module(), Tempo { bpm: 90, speed: 6 }, "90 BPM 4/4 kick snare hihat");
    let want = "Loaded samples (number name):\n1 kick\n2 snare\n\nTempo: 90 BPM, speed 6 (ticks per row)\n\nRequest: 90 BPM 4/4 kick snare hihat\n";
    assert_eq!(got, want);
}

#[test]
fn request_prompt_without_samples_says_none_and_cuts_long_requests() {
    let got = build_request(&Module::empty(), Tempo::default(), &"x".repeat(2000));
    assert!(got.starts_with("Loaded samples (number name):\n(none)\n"));
    assert!(got.contains("Tempo: 125 BPM, speed 6"));
    let request_line = got.lines().last().unwrap();
    assert_eq!(request_line.len(), "Request: ".len() + 500);
}

#[test]
fn system_prompt_states_the_mod_rules_and_the_cell_format() {
    for part in ["64 rows (row 0..63)", "4 channels (ch 0..3)", "C-1 to B-3", "1..31", "three hex digits", "JSON only", "\"fx\""] {
        assert!(SYSTEM_PROMPT.contains(part), "missing {part:?}");
    }
}

// ---- parser ----

#[test]
fn valid_json_gives_the_expected_cells() {
    let cells = parse_answer(GOOD, &module()).unwrap();
    // the all-empty third cell is dropped
    assert_eq!(cells.len(), 2);
    assert_eq!((cells[0].row, cells[0].ch), (0, 0));
    assert_eq!(cells[0].cell, Cell { period: 428, sample: 1, cmd: 0, param: 0 });
    assert_eq!((cells[1].row, cells[1].ch), (4, 1));
    assert_eq!(cells[1].cell, Cell { period: 381, sample: 2, cmd: 0xC, param: 0x20 });
}

#[test]
fn json_in_a_code_fence_or_a_cells_object_is_accepted() {
    let m = module();
    let fenced = format!("Here you go:\n```json\n{GOOD}\n```");
    assert_eq!(parse_answer(&fenced, &m).unwrap().len(), 2);
    let obj = format!("{{\"cells\": {GOOD}}}");
    assert_eq!(parse_answer(&obj, &m).unwrap().len(), 2);
}

fn bad_cells(answer: &str) -> Vec<(usize, Problem)> {
    match parse_answer(answer, &module()) {
        Err(ParseError::Invalid(b)) => b.into_iter().map(|b| (b.index, b.problem)).collect(),
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[test]
fn malformed_json_is_a_typed_error() {
    assert!(matches!(parse_answer("not json at all", &module()), Err(ParseError::Malformed(_))));
    assert!(matches!(parse_answer("[{\"row\": 0,", &module()), Err(ParseError::Malformed(_))));
    assert!(matches!(parse_answer("42", &module()), Err(ParseError::Malformed(_))));
    assert_eq!(parse_answer("[]", &module()), Err(ParseError::Empty));
}

#[test]
fn unknown_note_is_listed() {
    let b = bad_cells(r#"[{"row":0,"ch":0,"note":"H-9","sample":1},{"row":1,"ch":0,"note":"C-4","sample":1},{"row":2,"ch":0,"note":"é","sample":1}]"#);
    assert_eq!(b, vec![(0, Problem::UnknownNote("H-9".into())), (1, Problem::UnknownNote("C-4".into())), (2, Problem::UnknownNote("é".into()))]);
}

#[test]
fn sample_32_and_unloaded_samples_are_listed() {
    let b = bad_cells(r#"[{"row":0,"ch":0,"note":"C-2","sample":32},{"row":1,"ch":0,"note":"C-2","sample":3},{"row":2,"ch":0,"note":"C-2","sample":-1}]"#);
    assert_eq!(b, vec![(0, Problem::SampleOutOfRange(32)), (1, Problem::SampleNotLoaded(3)), (2, Problem::SampleOutOfRange(-1))]);
}

#[test]
fn channel_4_and_row_64_are_listed_with_their_position() {
    let answer = r#"[{"row":0,"ch":4,"note":"C-2","sample":1},{"row":64,"ch":0,"note":"C-2","sample":1},{"row":3,"ch":0,"note":"C-2","sample":1}]"#;
    let b = bad_cells(answer);
    assert_eq!(b, vec![(0, Problem::ChannelOutOfRange(4)), (1, Problem::RowOutOfRange(64))]);
    let err = parse_answer(answer, &module()).unwrap_err();
    let text = err.to_string();
    assert!(text.contains("cell #0 (row 0, ch 4)"), "{text}");
    assert!(text.contains("cell #1 (row 64, ch 0)"), "{text}");
}

#[test]
fn bad_effects_missing_fields_and_duplicates_are_listed() {
    let b = bad_cells(r#"[{"row":0,"ch":0,"note":"C-2","sample":1,"fx":"G00"},{"row":1,"ch":0,"note":"C-2","sample":1,"fx":"C2"},{"ch":0},{"row":5,"ch":2,"note":"C-2","sample":1},{"row":5,"ch":2,"note":"D-2","sample":1}]"#);
    assert_eq!(b[0], (0, Problem::BadEffect("G00".into())));
    assert_eq!(b[1], (1, Problem::BadEffect("C2".into())));
    assert!(matches!(b[2], (2, Problem::BadField(_))));
    assert_eq!(b[3], (4, Problem::DuplicateCell));
}

// ---- repair loop ----

struct Fake {
    answers: Vec<Result<String, ModelError>>,
    seen: Vec<Vec<Message>>,
}

impl Fake {
    fn new(answers: &[&str]) -> Self {
        Fake { answers: answers.iter().map(|a| Ok(a.to_string())).collect(), seen: vec![] }
    }
}

impl Model for Fake {
    fn complete(&mut self, system: &str, messages: &[Message]) -> Result<String, ModelError> {
        assert_eq!(system, SYSTEM_PROMPT);
        self.seen.push(messages.to_vec());
        self.answers.remove(0)
    }
}

const BAD: &str = r#"[{"row":64,"ch":0,"note":"C-2","sample":1}]"#;

#[test]
fn first_answer_invalid_second_valid_gives_a_proposal_after_one_retry() {
    let mut fake = Fake::new(&[BAD, GOOD]);
    let mut status = vec![];
    let p = compose(&mut fake, &module(), Tempo::default(), "a beat", &mut |s| status.push(s.to_string())).unwrap();
    assert_eq!(p.cells.len(), 2);
    assert_eq!(p.prompt, "a beat");
    assert_eq!(fake.seen.len(), 2);
    // the retry carries the question, the bad answer and the listed bad cell
    let retry = &fake.seen[1];
    assert_eq!(retry.len(), 3);
    assert_eq!(retry[1], Message { role: Role::Assistant, content: BAD.into() });
    assert!(retry[2].content.contains("cell #0 (row 64, ch 0)"), "{}", retry[2].content);
    assert_eq!(status.len(), 2);
}

#[test]
fn three_invalid_answers_give_an_error_and_no_proposal() {
    let mut fake = Fake::new(&[BAD, BAD, BAD, GOOD]);
    let err = compose(&mut fake, &module(), Tempo::default(), "a beat", &mut |_| {}).unwrap_err();
    assert_eq!(fake.seen.len(), 1 + MAX_RETRIES, "the fourth answer must not be asked for");
    match err {
        ComposeError::Invalid { attempts, last } => {
            assert_eq!(attempts, 3);
            assert!(matches!(last, ParseError::Invalid(_)));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn model_errors_stop_the_loop_and_map_to_req9_categories() {
    let mut fake = Fake { answers: vec![Err(ModelError::Network("down".into()))], seen: vec![] };
    let err = compose(&mut fake, &module(), Tempo::default(), "a beat", &mut |_| {}).unwrap_err();
    assert_eq!(err, ComposeError::Model(ModelError::Network("down".into())));
    assert!(matches!(paula::Error::from(err), paula::Error::Io(_)));
    assert!(matches!(paula::Error::from(ComposeError::Model(ModelError::NoKey)), paula::Error::InvalidArgument(_)));
    let invalid = ComposeError::Invalid { attempts: 3, last: ParseError::Empty };
    assert!(matches!(paula::Error::from(invalid), paula::Error::CorruptData(_)));
}

#[test]
fn empty_request_never_reaches_the_model() {
    let mut fake = Fake::new(&[]);
    let err = compose(&mut fake, &module(), Tempo::default(), "   ", &mut |_| {}).unwrap_err();
    assert_eq!(err, ComposeError::EmptyRequest);
}

#[test]
fn a_module_without_samples_never_reaches_the_model() {
    let mut fake = Fake::new(&[]);
    let err = compose(&mut fake, &Module::empty(), Tempo::default(), "a beat", &mut |_| {}).unwrap_err();
    assert_eq!(err, ComposeError::NoSamples);
}

// ---- proposals in a session ----

fn proposal() -> Proposal {
    Proposal::new("a beat", parse_answer(GOOD, &module()).unwrap())
}

#[test]
fn accept_writes_exactly_the_proposed_cells_into_a_new_pattern() {
    let before = module();
    let mut s = Session::new(before.clone());
    s.propose(proposal());
    assert_eq!(s.module(), &before, "a pending proposal changes nothing");
    let pos = s.accept().unwrap();
    let m = s.module();
    assert_eq!(pos, 1);
    assert_eq!(m.patterns.len(), 2);
    assert_eq!((m.song_len, m.order[1]), (2, 1));
    assert_eq!(m.patterns[0], before.patterns[0], "the old pattern is untouched");
    let mut non_empty = vec![];
    for r in 0..NUM_ROWS {
        for c in 0..NUM_CHANNELS {
            if !m.patterns[1][r][c].is_empty() {
                non_empty.push((r, c, m.patterns[1][r][c]));
            }
        }
    }
    assert_eq!(non_empty, vec![(0, 0, Cell { period: 428, sample: 1, cmd: 0, param: 0 }), (4, 1, Cell { period: 381, sample: 2, cmd: 0xC, param: 0x20 })]);
    assert!(s.proposal().is_none());
    // the result survives a save and load
    assert_eq!(Module::parse(&m.to_bytes()).unwrap(), *m);
}

#[test]
fn accept_is_one_undo_step() {
    let before = module();
    let mut s = Session::new(before.clone());
    s.propose(proposal());
    s.accept().unwrap();
    assert!(s.can_undo());
    assert!(s.undo());
    assert_eq!(s.module().to_bytes(), before.to_bytes());
    assert!(!s.undo(), "only one step was recorded");
}

#[test]
fn reject_leaves_the_module_byte_identical() {
    let before = module();
    let mut s = Session::new(before.clone());
    s.propose(proposal());
    s.reject();
    assert!(s.proposal().is_none());
    assert_eq!(s.module().to_bytes(), before.to_bytes());
    assert!(!s.can_undo());
    assert_eq!(s.accept(), Err(SessionError::NoProposal));
}

#[test]
fn a_full_song_cannot_accept_and_keeps_the_proposal() {
    let mut m = module();
    m.song_len = 128;
    let before = m.to_bytes();
    let mut s = Session::new(m);
    s.propose(proposal());
    assert_eq!(s.accept(), Err(SessionError::SongFull));
    assert!(s.proposal().is_some());
    assert_eq!(s.module().to_bytes(), before);
}

// ---- key and model lookup ----

fn scratch_dir(name: &str) -> std::path::PathBuf {
    let d = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn dotenv_lines_are_read_with_quotes_comments_and_export() {
    let env = "# comment\nOTHER=1\nexport ANTHROPIC_API_KEY=\"sk-test-1\"\n";
    assert_eq!(dotenv_value(env, "ANTHROPIC_API_KEY").as_deref(), Some("sk-test-1"));
    assert_eq!(dotenv_value("ANTHROPIC_API_KEY='k2'", "ANTHROPIC_API_KEY").as_deref(), Some("k2"));
    assert_eq!(dotenv_value("# ANTHROPIC_API_KEY=k3", "ANTHROPIC_API_KEY"), None);
    assert_eq!(dotenv_value("NOPE=1", "ANTHROPIC_API_KEY"), None);
}

#[test]
fn key_comes_from_the_environment_first_then_env_files_in_order() {
    let a = scratch_dir("key-a");
    let b = scratch_dir("key-b");
    let none = scratch_dir("key-none");
    std::fs::write(a.join(".env"), "ANTHROPIC_API_KEY=from-a\n").unwrap();
    std::fs::write(b.join(".env"), "ANTHROPIC_API_KEY=from-b\n").unwrap();
    let dirs = [none.clone(), a.clone(), b.clone()];
    assert_eq!(find_key(Some("from-env".into()), &dirs).unwrap().expose(), "from-env");
    assert_eq!(find_key(None, &dirs).unwrap().expose(), "from-a");
    assert_eq!(find_key(Some("  ".into()), &dirs).unwrap().expose(), "from-a");
    assert!(find_key(None, &[none]).is_none());
}

#[test]
fn the_key_is_hidden_in_debug_and_display() {
    let k = find_key(Some("sk-secret-value".into()), &[]).unwrap();
    assert!(!format!("{k:?} {k}").contains("secret"));
}

#[test]
fn model_name_has_a_default() {
    assert_eq!(pick_model(None), DEFAULT_MODEL);
    assert_eq!(pick_model(Some(" ".into())), DEFAULT_MODEL);
    assert_eq!(pick_model(Some("my-model".into())), "my-model");
}

// ---- client wire format (feature llm, still no network) ----

#[cfg(feature = "llm")]
mod wire {
    use paula::llm::client::{error_message, request_body, response_text};
    use paula::llm::{Message, ModelError};

    #[test]
    fn request_body_has_the_messages_api_shape() {
        let b = request_body("m", "sys", &[Message::user("hi"), Message::assistant("yo")]);
        assert_eq!(b["model"], "m");
        assert_eq!(b["system"], "sys");
        assert!(b["max_tokens"].as_u64().unwrap() > 0);
        assert_eq!(b["messages"][0]["role"], "user");
        assert_eq!(b["messages"][0]["content"], "hi");
        assert_eq!(b["messages"][1]["role"], "assistant");
    }

    #[test]
    fn response_text_joins_text_blocks() {
        let body = r#"{"id":"x","type":"message","role":"assistant","content":[{"type":"text","text":"[1,"},{"type":"text","text":"2]"}],"stop_reason":"end_turn"}"#;
        assert_eq!(response_text(body).unwrap(), "[1,2]");
        assert_eq!(response_text(r#"{"content":[],"stop_reason":"end_turn"}"#), Err(ModelError::BadResponse("no text in the answer".into())));
        assert_eq!(response_text(r#"{"content":[{"type":"text","text":"["}],"stop_reason":"max_tokens"}"#), Err(ModelError::Truncated));
        assert!(matches!(response_text("<html>"), Err(ModelError::BadResponse(_))));
    }

    #[test]
    fn error_body_message_is_extracted() {
        let body = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        assert_eq!(error_message(body), "invalid x-api-key");
        assert_eq!(error_message("gateway down"), "gateway down");
    }
}
