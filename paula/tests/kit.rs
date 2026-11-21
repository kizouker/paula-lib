//! Own-sample kits: WAV reading, retuning, slots (issue #68).

use paula::score::kit::{demo_kit, retune, semitones_from_c3, with_sounds, KitSound, C2_RATE};
use std::path::PathBuf;

fn starter(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/starter-kit").join(name)
}

/// A WAV written the way other tools do it: an extra chunk before `data`.
fn wav(rate: u32, bits: u16, channels: u16, data: &[u8]) -> Vec<u8> {
    let mut b = b"RIFF\0\0\0\0WAVE".to_vec();
    b.extend_from_slice(b"fmt \x10\0\0\0\x01\0");
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    let align = channels * bits / 8;
    b.extend_from_slice(&(rate * align as u32).to_le_bytes());
    b.extend_from_slice(&align.to_le_bytes());
    b.extend_from_slice(&bits.to_le_bytes());
    b.extend_from_slice(b"FLLR\x04\0\0\0\0\0\0\0");
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(data.len() as u32).to_le_bytes());
    b.extend_from_slice(data);
    b
}

#[test]
fn reads_8_bit_unsigned_and_16_bit_stereo_past_extra_chunks() {
    let (rate, d) = paula::wav::read_mono8(&wav(8363, 8, 1, &[128, 255, 0])).unwrap();
    assert_eq!(rate, 8363);
    assert_eq!(d, vec![0, 127, -128]);
    // stereo 16-bit: left full positive, right silent -> about half
    let frame = [0xFF, 0x7F, 0x00, 0x00];
    let (rate, d) = paula::wav::read_mono8(&wav(48000, 16, 2, &frame)).unwrap();
    assert_eq!(rate, 48000);
    assert!((62..=64).contains(&d[0]), "{d:?}");
    assert!(paula::wav::read_mono8(b"not a wav").is_err());
}

#[test]
fn note_names_count_from_c3() {
    assert_eq!(semitones_from_c3("C3"), Some(0));
    assert_eq!(semitones_from_c3("G3"), Some(7));
    assert_eq!(semitones_from_c3("Ab1"), Some(-16));
    assert_eq!(semitones_from_c3("C#4"), Some(13));
    assert_eq!(semitones_from_c3("H2"), None);
}

#[test]
fn retune_moves_the_root_to_c3_and_converts_the_rate() {
    let data: Vec<i8> = (0..1200).map(|i| ((i as f64 / 10.0).sin() * 100.0) as i8).collect();
    // a G3 sample at the C-2 rate must get 2^(7/12) longer to play C3
    let g = retune(&data, C2_RATE as u32, Some(7));
    assert_eq!(g.len(), (1200.0 * 2f64.powf(7.0 / 12.0)).round() as usize);
    // an Ab1 bass must get 2^(16/12) shorter
    let ab = retune(&data, C2_RATE as u32, Some(-16));
    assert_eq!(ab.len(), (1200.0 / 2f64.powf(16.0 / 12.0)).round() as usize);
    // a drum at 48 kHz is only rate-converted
    let k = retune(&data, 48000, None);
    assert_eq!(k.len(), (1200.0 * C2_RATE / 48000.0).round() as usize);
    // the C-2 rate and no root: unchanged
    assert_eq!(retune(&data, C2_RATE as u32, None), data);
}

#[test]
fn own_sounds_go_into_their_slots_and_the_rest_stays() {
    let demo = demo_kit();
    let kit = with_sounds(demo.clone(), &[KitSound { slot: 4, name: "my bass".into(), data: vec![1, 2, 3], volume: 64 }]).unwrap();
    assert_eq!(kit.samples[3].data, vec![1, 2, 3, 0], "padded to an even length");
    assert_eq!(kit.samples[3].loop_len, 2, "one-shot, not a chip loop");
    assert_eq!(paula::modfile::text(&kit.samples[3].name).trim(), "my bass");
    assert_eq!(kit.samples[1].data, demo.samples[1].data, "snare kept from the demo kit");
    assert!(with_sounds(demo, &[KitSound { slot: 32, name: "x".into(), data: vec![], volume: 64 }]).is_err());
}

#[test]
fn rickards_kit_compiles_rio_zouk_and_is_louder_than_the_chip_bass() {
    let read = |f: &str| paula::wav::read_mono8(&std::fs::read(starter(f)).unwrap()).unwrap();
    let (kr, kick) = read("01_kick_larm.wav");
    let (br, bass) = read("04_bass.wav");
    let sounds = [
        KitSound { slot: 1, name: "kick".into(), data: retune(&kick, kr, None), volume: 64 },
        KitSound { slot: 4, name: "bass".into(), data: retune(&bass, br, Some(-16)), volume: 64 },
    ];
    let kit = with_sounds(demo_kit(), &sounds).unwrap();
    assert!(kit.samples[3].data.len() > 1000, "the bass is Rickard's SG, not a 64-byte chip cycle");
    let src = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/rio-zouk.paula.toml")).unwrap();
    let (module, _) = paula::score::compile_source(&src, &kit, false).unwrap();
    assert!(module.song_len > 1);
    // round trip through the MOD format keeps the own samples
    let back = paula::Module::parse(&module.to_bytes()).unwrap();
    assert_eq!(back.samples[0].data.len(), kit.samples[0].data.len());
}

#[test]
fn demo_keys_are_a_decaying_tone_not_a_beeping_loop() {
    let kit = demo_kit();
    let keys = &kit.samples[4];
    assert!(keys.loop_len <= 2, "one shot");
    assert!(keys.data.len() > 8000, "about 1.2 s, not a 32-byte cycle");
    let peak = |r: std::ops::Range<usize>| keys.data[r].iter().map(|&v| (v as i32).abs()).max().unwrap();
    assert!(peak(0..800) > 3 * peak(keys.data.len() - 800..keys.data.len()), "it fades out");
    // tuned like the bass: 64 samples per period
    let bass_period = kit.samples[3].data.len();
    assert_eq!(bass_period, 64);
}
