use paula::llm::score::{changed_lines, ScoreProposal};
use paula::modfile::{period_of, Cell, Module};
use paula::rules::Placed;
use paula::score::kit::demo_kit;
use paula::session::Proposal;
use paula_gui::audio::SimulatedAudio;
use paula_gui::circle::{CircleHit, Ring};
use paula_gui::edit::EditField;
use paula_gui::player::{AuditionMode, PlayerState};
use paula_gui::state::{Action, AppState, LlmEvent};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../paula/target/night")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const ZOUK_SCORE: &str = r#"
[style]
name = "rio-zouk"
key = "A aeolian"
meter = "4/4"
bpm = 86

[progression]
verse = "Am | Dm | Em | Am"

[role.kick]
channel = 1
sample = 1
groove = "x.......x......."

[role.chick]
channel = 2
sample = 2
groove = "....x.o.....x.o."
delay = 2

[role.bass]
channel = 3
sample = 3
groove = "x.......x......."
register = "C-1..G-2"
rule = "root"

[role.pad]
channel = 4
sample = 4
groove = "x..............."
duck = "kick"
register = "C-2..B-3"
rule = "chord-tones"

[song]
form = "verse 4"
"#;

#[test]
fn clear_clip_and_circle_click_actions_update_state() {
    let audio = SimulatedAudio::available();
    let mut player = PlayerState::new(demo_kit(), "zouk", 44100, true).unwrap();
    player.master_meter.update_from_pcm(&[32767, -32768], 0.016);
    assert!(player.master_meter.left.clipped);
    assert!(player.master_meter.right.clipped);

    let mut app = AppState::direct_player(player);
    app.dispatch(Action::ClearClip, &audio);
    let p = app.player.as_ref().unwrap();
    assert!(!p.master_meter.left.clipped);
    assert!(!p.master_meter.right.clipped);

    let before_mod = p.module().clone();
    app.dispatch(
        Action::ClickCircle(CircleHit {
            sector: 11,
            ring: Ring::Minor,
            label: "Dm",
        }),
        &audio,
    );
    let p = app.player.as_ref().unwrap();
    assert_eq!(p.circle.active_sector, 11);
    assert_eq!(p.circle.active_ring, Ring::Minor);
    assert_eq!(p.selected_chord_label.as_deref(), Some("Dm"));
    assert!(p.diatonic_chords.iter().any(|(_, _, s)| s == "Dm"));
    // no score loaded: the click shows the key's chords and leaves the song alone (#96)
    assert_eq!(p.audition_mode, AuditionMode::None);
    assert_eq!(p.module(), &before_mod);
}

#[test]
fn pad_ducking_toggle_updates_score_and_module() {
    let audio = SimulatedAudio::unavailable("headless");
    let kit = demo_kit();
    let (module, _) = paula::score::compile_source(ZOUK_SCORE, &kit, false).unwrap();
    let score = paula::score::lint::analyze(
        ZOUK_SCORE,
        Some(&kit),
        &paula::score::lint::Options { strict: false },
    )
    .score
    .unwrap();

    let mut player = PlayerState::new(module, "zouk", 44100, false).unwrap();
    player.set_score_context(Some(ZOUK_SCORE.to_string()), Some(score), Some(kit), None);
    let mut app = AppState::direct_player(player);

    assert!(app.player.as_ref().unwrap().score.as_ref().unwrap().roles[3].duck.is_some());
    app.dispatch(Action::TogglePadDucking, &audio);
    assert!(app.player.as_ref().unwrap().score.as_ref().unwrap().roles[3].duck.is_none());
    app.dispatch(Action::TogglePadDucking, &audio);
    assert!(app.player.as_ref().unwrap().score.as_ref().unwrap().roles[3].duck.is_some());
}

#[test]
fn tracker_edit_mode_navigation_entry_undo_and_save_overwrite() {
    let dir = scratch("studio-edit-save");
    let orig_mod_path = dir.join("zouk.mod");
    std::fs::write(&orig_mod_path, Module::empty().to_bytes()).unwrap();

    let audio = SimulatedAudio::unavailable("headless");
    let mut player = PlayerState::new(Module::empty(), "zouk.mod", 44100, false).unwrap();
    player.source_path = Some(orig_mod_path.clone());
    let mut app = AppState::direct_player(player);

    app.dispatch(Action::ToggleEditMode, &audio);
    assert!(app.player.as_ref().unwrap().edit_mode);

    app.dispatch(Action::EditRow(4), &audio);
    assert_eq!(app.player.as_ref().unwrap().edit_cursor.row, 4);

    app.dispatch(Action::EditOctave(-1), &audio);
    assert_eq!(app.player.as_ref().unwrap().edit_octave, 1);
    app.dispatch(Action::EditOctave(1), &audio);
    assert_eq!(app.player.as_ref().unwrap().edit_octave, 2);

    // Enter note C-2
    app.dispatch(Action::EditKey('z'), &audio);
    assert_eq!(
        app.player.as_ref().unwrap().module().patterns[0][4][0].period,
        period_of("C-2").unwrap()
    );

    // Tab through fields to SampleHi, SampleLo, FxCmd, FxParamHi, FxParamLo, then wrap to channel 1
    app.dispatch(Action::EditNextField, &audio);
    assert_eq!(app.player.as_ref().unwrap().edit_cursor.field, EditField::SampleHi);
    app.dispatch(Action::EditKey('0'), &audio);
    app.dispatch(Action::EditNextField, &audio);
    app.dispatch(Action::EditKey('2'), &audio);
    assert_eq!(app.player.as_ref().unwrap().module().patterns[0][4][0].sample, 2);

    app.dispatch(Action::EditNextField, &audio); // FxCmd
    app.dispatch(Action::EditNextField, &audio); // FxParamHi
    app.dispatch(Action::EditNextField, &audio); // FxParamLo
    app.dispatch(Action::EditNextField, &audio); // wraps to ch 1 Note
    assert_eq!(app.player.as_ref().unwrap().edit_cursor.channel, 1);
    assert_eq!(app.player.as_ref().unwrap().edit_cursor.field, EditField::Note);

    app.dispatch(Action::EditPrevField, &audio); // wraps back to ch 0 FxParamLo
    assert_eq!(app.player.as_ref().unwrap().edit_cursor.channel, 0);
    assert_eq!(app.player.as_ref().unwrap().edit_cursor.field, EditField::FxParamLo);

    // Delete clears field; Backspace clears cell; u undoes through Session
    app.dispatch(Action::EditBackspace, &audio);
    assert_eq!(app.player.as_ref().unwrap().module().patterns[0][4][0], Cell::default());
    app.dispatch(Action::UndoProposal, &audio);
    assert_eq!(app.player.as_ref().unwrap().module().patterns[0][4][0].sample, 2);

    // First SaveMod asks before overwriting existing file
    let before_bytes = std::fs::read(&orig_mod_path).unwrap();
    app.dispatch(Action::SaveMod, &audio);
    assert!(app.status.contains("w again"));
    assert_eq!(std::fs::read(&orig_mod_path).unwrap(), before_bytes);

    // Second SaveMod overwrites
    app.dispatch(Action::SaveMod, &audio);
    assert!(app.status.contains("saved"));
    let saved = paula::read_module(&orig_mod_path).unwrap();
    assert_eq!(saved.patterns[0][4][0].sample, 2);
}

#[test]
fn pattern_and_score_proposal_audition_accept_and_undo() {
    let audio = SimulatedAudio::available();
    let kit = demo_kit();
    let player = PlayerState::new(kit.clone(), "zouk", 44100, true).unwrap();
    let old_song_len = player.module().song_len;

    let mut app = AppState::direct_player(player);

    // Deliver a pattern proposal via LlmEvent channel
    let (tx, rx) = mpsc::channel();
    app.llm_rx = Some(rx);
    let cell = Cell { period: 428, sample: 1, cmd: 0xE, param: 0xD2 };
    let prop = Proposal::new("laid-back chick", vec![Placed { row: 4, ch: 1, cell }]);
    tx.send(LlmEvent::DonePattern(Ok(prop))).unwrap();
    app.poll_llm();

    // Space auditions proposed pattern without mutating Session::module()
    app.dispatch(Action::TogglePlay, &audio);
    let p = app.player.as_ref().unwrap();
    assert_eq!(p.audition_mode, AuditionMode::PatternProposal);
    assert!(p.is_playing());
    assert_eq!(p.module().song_len, old_song_len);

    // Accept commits at new song position and clears audition mode
    app.dispatch(Action::AcceptProposal, &audio);
    let p = app.player.as_ref().unwrap();
    assert_eq!(p.audition_mode, AuditionMode::None);
    assert_eq!(p.module().song_len, old_song_len + 1);
    assert_eq!(p.view_pos as u8, old_song_len);

    // Deliver a song-level ScoreProposal via LlmEvent channel
    let (tx2, rx2) = mpsc::channel();
    app.llm_rx = Some(rx2);
    let (new_mod, warnings) = paula::score::compile_source(ZOUK_SCORE, &kit, false).unwrap();
    let diff = changed_lines("", ZOUK_SCORE);
    let sp = ScoreProposal {
        request: "write zouk".into(),
        text: ZOUK_SCORE.into(),
        module: new_mod,
        warnings,
    };
    tx2.send(LlmEvent::DoneScore(Ok((sp, diff)))).unwrap();
    app.poll_llm();
    assert!(app.player.as_ref().unwrap().pending_score_proposal.is_some());

    app.dispatch(Action::AcceptProposal, &audio);
    let p = app.player.as_ref().unwrap();
    assert!(p.pending_score_proposal.is_none());
    assert!(p.score.is_some());
    assert!(p.prev_score_snapshot.is_some());
}
