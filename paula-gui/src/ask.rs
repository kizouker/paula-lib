use paula::llm::score::{changed_lines, write_score, ScoreError, ScoreProposal};
use paula::llm::Model;
use paula::modfile::Module;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AskScope {
    Pattern,
    Song,
}

const SONG_KEYWORDS: &[&str] = &[
    "key",
    "mode",
    "scale",
    "bpm",
    "tempo",
    "progression",
    "chord",
    "chords",
    "form",
    "structure",
    "verse",
    "chorus",
    "intro",
    "outro",
    "bridge",
    "song",
    "style",
    "zouk",
    "whole",
    "arrangement",
];

const PATTERN_KEYWORDS: &[&str] = &[
    "channel",
    "ch1",
    "ch2",
    "ch3",
    "ch4",
    "pattern",
    "row",
    "kick",
    "chick",
    "bass",
    "pad",
    "fill",
    "ghost",
    "delay",
    "variation",
];

/// Classify whether a prompt targets the whole song (key, form, tempo, progression, style)
/// or a single pattern/channel variation.
pub fn classify_ask(request: &str, _has_score: bool) -> AskScope {
    let lower = request.to_ascii_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();

    let song_hits = tokens.iter().filter(|t| SONG_KEYWORDS.contains(t)).count();
    let pattern_hits = tokens.iter().filter(|t| PATTERN_KEYWORDS.contains(t)).count();

    if song_hits > pattern_hits {
        AskScope::Song
    } else {
        AskScope::Pattern
    }
}

/// Run a song-level ask via `paula::llm::score::write_score` and compute line diffs.
pub fn run_song_ask(
    model: &mut dyn Model,
    request: &str,
    current_toml: Option<&str>,
    kit: &Module,
    strict: bool,
    progress: &mut dyn FnMut(&str),
) -> Result<(ScoreProposal, Vec<String>), ScoreError> {
    let proposal = write_score(model, request, current_toml, kit, strict, progress)?;
    let diff = changed_lines(current_toml.unwrap_or(""), &proposal.text);
    Ok((proposal, diff))
}

#[cfg(test)]
mod tests {
    use super::*;
    use paula::llm::{Message, ModelError};
    use paula::modfile::Sample;

    struct FakeModel {
        answers: Vec<String>,
        idx: usize,
    }

    impl Model for FakeModel {
        fn complete(&mut self, _system: &str, _messages: &[Message]) -> Result<String, ModelError> {
            let out = self.answers.get(self.idx).cloned().unwrap_or_default();
            self.idx += 1;
            Ok(out)
        }
    }

    fn kit_with_three_samples() -> Module {
        let mut m = Module::empty();
        for i in 0..3 {
            m.samples[i] = Sample {
                data: vec![0, 10, 20, 10],
                volume: 64,
                ..Sample::default()
            };
        }
        m
    }

    #[test]
    fn classify_ask_routes_song_vs_pattern() {
        assert_eq!(classify_ask("change key to D dorian at 86 BPM", true), AskScope::Song);
        assert_eq!(classify_ask("write a new zouk progression for the verse", false), AskScope::Song);
        assert_eq!(classify_ask("add a ghost chick variation on channel 2", true), AskScope::Pattern);
        assert_eq!(classify_ask("make a pattern fill on ch3", true), AskScope::Pattern);
    }

    #[test]
    fn run_song_ask_calls_write_score_with_mock_model() {
        let valid_zouk = r#"```toml
[style]
name = "caboverde-zouk"
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

[role.pad]
channel = 4
sample = 3
groove = "x..............."
duck = "kick"
register = "C-2..B-3"
rule = "chord-tones"

[song]
form = "verse 4"
```"#;
        let mut model = FakeModel { answers: vec![valid_zouk.to_string()], idx: 0 };
        let mut logs = Vec::new();
        let (proposal, diff) = run_song_ask(
            &mut model,
            "write a Brazilian zouk in A aeolian at 86 BPM",
            None,
            &kit_with_three_samples(),
            false,
            &mut |msg| logs.push(msg.to_string()),
        )
        .unwrap();
        assert!(!logs.is_empty());
        assert!(!diff.is_empty());
        assert_eq!(proposal.module.song_len, 1);

        let mut bad_model = FakeModel {
            answers: vec!["not valid toml".into(), "still bad".into(), "nope".into()],
            idx: 0,
        };
        let err = run_song_ask(
            &mut bad_model,
            "write a Brazilian zouk",
            None,
            &kit_with_three_samples(),
            false,
            &mut |_| {},
        )
        .unwrap_err();
        assert!(matches!(err, ScoreError::Invalid { attempts: 3, .. }));
    }
}
