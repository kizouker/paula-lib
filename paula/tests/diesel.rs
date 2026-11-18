//! Tests for the Diesel mascot (issue #29). No audio device; every MOD is generated here.

use paula::diesel::{self, ansi_lines, frame, half_blocks, still, Choreography, Dancer, Image, Move};
use paula::modfile::{Cell, Module, NUM_ROWS};
use paula::{Engine, EventKind};

const RATE: u32 = 44_100;

#[test]
fn svg_parses_to_28x28_with_537_coloured_pixels() {
    let img = Image::diesel();
    assert_eq!((img.w, img.h), (28, 28));
    assert_eq!(img.coloured(), 537);
    // known colours straight from assets/diesel.svg
    assert_eq!(img.get(5, 0), Some([0x2A, 0x2A, 0x33]));
    assert_eq!(img.get(5, 2), Some([0x74, 0x70, 0x7A]));
    assert_eq!(img.get(20, 1), Some([0x2A, 0x2A, 0x33]));
    assert_eq!(img.get(0, 0), None, "no rect there: transparent");
    // the art uses 13 distinct colours
    let mut distinct = img.colour_multiset();
    distinct.dedup();
    assert_eq!(distinct.len(), 13);
}

#[test]
fn svg_parser_rejects_bad_input() {
    assert!(Image::from_svg("<svg></svg>").is_err());
    let bad = r##"<svg viewBox="0 0 2 2"><rect x="5" y="0" width="1" height="1" fill="#000000"/></svg>"##;
    assert!(Image::from_svg(bad).is_err());
    let big = r##"<svg viewBox="0 0 2 2"><rect x="0" y="0" width="2" height="1" fill="#000000"/></svg>"##;
    assert!(Image::from_svg(big).is_err());
}

fn all_pose_combinations() -> Vec<Vec<Move>> {
    (0u32..1 << Move::ALL.len())
        .map(|bits| Move::ALL.iter().enumerate().filter(|(i, _)| bits >> i & 1 == 1).map(|(_, m)| *m).collect())
        .collect()
}

#[test]
fn every_frame_has_the_colours_of_the_source() {
    let src = Image::diesel();
    let want = src.colour_multiset();
    let poses = all_pose_combinations();
    assert_eq!(poses.len(), 64);
    for moves in &poses {
        let f = frame(&src, moves);
        assert_eq!(f.colour_multiset(), want, "pixels invented or lost by {moves:?}");
    }
    assert_eq!(still(&src).colour_multiset(), want);
}

#[test]
fn moves_actually_move_pixels() {
    let src = Image::diesel();
    let rest = still(&src);
    for m in Move::ALL {
        assert_ne!(frame(&src, &[m]), rest, "{m:?} changes nothing");
    }
    // the still frame is the source in the middle of the margin
    assert_eq!(rest.get(5 + diesel::MARGIN, diesel::MARGIN), src.get(5, 0));
}

#[test]
fn half_block_render_of_a_2x2_image() {
    let (a, b, c) = ([1, 2, 3], [4, 5, 6], [7, 8, 9]);
    let mut img = Image::new(2, 2);
    img.set(0, 0, Some(a));
    img.set(0, 1, Some(b));
    img.set(1, 1, Some(c)); // (1,0) stays transparent
    let rows = half_blocks(&img);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], diesel::Cell { ch: '▀', fg: Some(a), bg: Some(b) });
    assert_eq!(rows[0][1], diesel::Cell { ch: '▄', fg: Some(c), bg: None });
    let lines = ansi_lines(&img);
    assert_eq!(lines[0], "\x1b[0m\x1b[38;2;1;2;3m\x1b[48;2;4;5;6m▀\x1b[0m\x1b[38;2;7;8;9m▄\x1b[0m");
    // an empty cell is a plain space
    assert_eq!(half_blocks(&Image::new(1, 2))[0][0], diesel::Cell { ch: ' ', fg: None, bg: None });
}

#[test]
fn half_block_render_of_diesel_is_the_canvas_size_over_two() {
    let f = still(&Image::diesel());
    let rows = half_blocks(&f);
    assert_eq!((rows[0].len(), rows.len()), (28 + 2 * diesel::MARGIN, (28 + 2 * diesel::MARGIN) / 2));
}

#[test]
fn choreography_parses_and_reports_errors() {
    let c = Choreography::standard();
    assert_eq!(c.beats_per_bar, 4);
    assert_eq!(c.steps[0].as_ref().unwrap().moves, vec![Move::Bob, Move::LeanLeft]);
    assert_eq!(c.steps[3].as_ref().unwrap().hold, 3);
    assert_eq!(c.kick, vec![Move::Squash]);
    assert!(c.mirror_alternate_bars);
    assert!(Choreography::parse("beat 0 moonwalk").is_err());
    assert!(Choreography::parse("beats-per-bar 2\nbeat 2 bob").is_err());
    assert!(Choreography::parse("beat 0 bob hold=9").is_err());
    assert!(Choreography::parse("dance now").is_err());
    let custom = Choreography::parse("beats-per-bar 2\nbeat 1 hop # comment\n").unwrap();
    assert_eq!(custom.steps, vec![None, Some(diesel::Step { moves: vec![Move::Hop], hold: 2 })]);
}

/// One pattern, kick on channel 1 every 4 rows (like tests/events.rs).
fn kick_module(with_notes: bool) -> Module {
    let mut m = Module::empty();
    m.samples[0].data = (0..2000).map(|i| ((i as f64 * 0.05).sin() * 100.0) as i8).collect();
    m.samples[0].volume = 64;
    m.samples[0].loop_len = 2;
    if with_notes {
        for row in (0..NUM_ROWS).step_by(4) {
            m.patterns[0][row][0] = Cell { period: 428, sample: 1, cmd: 0, param: 0 };
        }
    }
    m
}

/// Pose at the end of every row of the first pattern, driven by the real event stream.
fn poses_per_row(m: &Module) -> Vec<(u8, Vec<Move>)> {
    let mut e = Engine::new(RATE).unwrap();
    e.load(m).unwrap();
    e.play();
    let mut events = Vec::new();
    let mut buf = vec![0i16; 4096 * 2];
    let mut dancer = Dancer::new(Choreography::standard());
    let mut out: Vec<(u8, Vec<Move>)> = Vec::new();
    let mut cur: Option<u8> = None;
    'run: for _ in 0..2000 {
        events.clear();
        e.render_events(&mut buf, &mut events).unwrap();
        for ev in &events {
            if let EventKind::RowStart { row, .. } = ev.kind {
                if let Some(prev) = cur {
                    out.push((prev, dancer.pose()));
                }
                if out.len() >= 32 {
                    break 'run;
                }
                cur = Some(row);
            }
            dancer.on_event(ev);
        }
    }
    out
}

#[test]
fn kick_every_4_rows_gives_a_bob_frame_on_those_rows() {
    let poses = poses_per_row(&kick_module(true));
    assert!(poses.len() >= 16, "got {} rows", poses.len());
    for (row, pose) in &poses {
        let beat = (*row / 4) as usize % 4;
        match (row % 4, beat) {
            (0 | 1, 0..=2) => {
                assert!(pose.contains(&Move::Bob), "row {row}: {pose:?}");
                assert!(pose.contains(&Move::Squash), "row {row}: kick squash, got {pose:?}");
            }
            (0..=2, 3) if *row % 4 < 3 => assert!(pose.contains(&Move::Hop), "row {row}: {pose:?}"),
            _ => {}
        }
        if row % 4 == 3 || (*row % 4 == 2 && beat != 3) {
            assert!(!pose.iter().any(|m| matches!(m, Move::Bob | Move::Hop | Move::Squash)), "row {row} should rest: {pose:?}");
        }
    }
    // beat 0 leans left, beat 1 leans right
    let at = |r: u8| &poses.iter().find(|(x, _)| *x == r).unwrap().1;
    assert!(at(0).contains(&Move::LeanLeft));
    assert!(at(4).contains(&Move::LeanRight));
}

#[test]
fn no_notes_means_no_kick_squash() {
    let poses = poses_per_row(&kick_module(false));
    assert!(poses.iter().any(|(_, p)| p.contains(&Move::Bob)), "the beat still bobs");
    assert!(poses.iter().all(|(_, p)| !p.contains(&Move::Squash)));
}

#[test]
fn stopped_or_silent_diesel_stands_still() {
    let mut d = Dancer::new(Choreography::standard());
    assert!(d.pose().is_empty(), "never started");
    d.on_row(0, 0);
    d.on_note();
    assert!(!d.pose().is_empty());
    d.set_levels(&[0.0; 4]);
    assert!(d.pose().is_empty(), "silent");
    d.set_levels(&[0.0, 0.5, 0.0, 0.0]);
    assert!(!d.pose().is_empty());
    d.stop();
    assert!(d.pose().is_empty(), "stopped");
}

#[test]
fn mirror_on_every_second_bar_and_position_driven_kick() {
    let m = kick_module(true);
    let mut d = Dancer::new(Choreography::standard());
    d.on_position(&m, 0, 2);
    assert!(!d.pose().contains(&Move::Mirror), "bar 0");
    d.on_position(&m, 0, 16);
    assert!(d.pose().contains(&Move::Mirror), "bar 1");
    assert!(d.pose().contains(&Move::Squash), "note read from the module cell");
    d.on_position(&m, 0, 32);
    assert!(!d.pose().contains(&Move::Mirror), "bar 2");
}
