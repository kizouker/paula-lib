use paula::llm::{Message, Model, ModelError};
use paula::score::kit::demo_kit;
use paula_gui::player::PlayerState;

struct FakeModel {
    answer: String,
    calls: usize,
}

impl Model for FakeModel {
    fn complete(&mut self, _system: &str, _messages: &[Message]) -> Result<String, ModelError> {
        self.calls += 1;
        Ok(self.answer.clone())
    }
}

#[test]
fn fake_model_propose_accept_reject_and_undo() {
    let module = demo_kit();
    let mut player = PlayerState::new(module, "test", 44100, false).unwrap();
    let before = player.module().clone();
    let mut fake = FakeModel {
        answer: r#"{"cells":[{"row":0,"ch":0,"note":"C-2","sample":1,"fx":"..."}]}"#.to_string(),
        calls: 0,
    };

    player.submit_prompt_with(&mut fake, "add a kick");
    assert_eq!(fake.calls, 1);
    assert!(player.session.proposal().is_some());
    assert_eq!(player.module(), &before);

    player.accept_proposal();
    assert_eq!(player.session.module().song_len, before.song_len + 1);
    assert_eq!(player.session.module().patterns.len(), before.patterns.len() + 1);
    assert!(player.session.proposal().is_none());
    assert!(player.session.can_undo());

    player.undo_proposal();
    assert_eq!(player.session.module().song_len, before.song_len);
    assert_eq!(player.session.module().patterns.len(), before.patterns.len());
    assert!(!player.session.can_undo());

    player.submit_prompt_with(&mut fake, "another pattern");
    assert!(player.session.proposal().is_some());
    player.reject_proposal();
    assert!(player.session.proposal().is_none());
}

#[test]
fn empty_request_and_empty_module_do_not_call_model() {
    let mut player = PlayerState::new(demo_kit(), "test", 44100, false).unwrap();
    let mut fake = FakeModel {
        answer: r#"{"cells":[]}"#.to_string(),
        calls: 0,
    };
    player.submit_prompt_with(&mut fake, "  ");
    assert_eq!(fake.calls, 0);
    assert!(player.status.contains("request is empty"));

    let mut empty = PlayerState::new(paula::modfile::Module::empty(), "empty", 44100, false).unwrap();
    empty.submit_prompt_with(&mut fake, "add something");
    assert_eq!(fake.calls, 0);
    assert!(empty.status.contains("no samples"));
}

#[test]
fn open_ask_guards_in_flight_and_pending_proposal() {
    let mut player = PlayerState::new(demo_kit(), "test", 44100, false).unwrap();
    player.llm_in_flight = true;
    player.open_ask();
    assert!(!player.ask_open);
    assert_eq!(player.status, "still waiting for the model");

    player.llm_in_flight = false;
    let mut fake = FakeModel {
        answer: r#"{"cells":[{"row":0,"ch":0,"note":"C-2","sample":1,"fx":"..."}]}"#.to_string(),
        calls: 0,
    };
    player.submit_prompt_with(&mut fake, "add a kick");
    player.open_ask();
    assert!(!player.ask_open);
    assert!(player.status.contains("proposal is waiting"));
}
