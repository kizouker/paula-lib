//! Tests for `assets/starter-kit/convert.sh` (SoX conversion to ProTracker samples).
//!
//! Inputs are synthetic and made with `sox synth` inside the test: no real
//! recordings, no network. Equation numbers refer to
//! `docs/sample-conversion/index.md`. The reference and the statistics come
//! from this crate (`pt2_dsp`), so the script is checked against the same math
//! as the simulations.
//!
//! Interface under test:
//!
//! ```text
//! convert.sh [--no-dither] <list-file> <out-dir>
//! list line:  source | target | rate | gain      (blank lines and # comments ignored)
//! ```
//!
//! Scratch files go to `CARGO_TARGET_TMPDIR` (inside `target/`), never `/tmp`.

use pt2_dsp::{Dither, Lcg, error_moments, quant, step_size};
use std::f64::consts::PI;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const RATE: f64 = 8363.0;
const SOX_HOMEBREW: &str = "/opt/homebrew/bin/sox";

// ---------------------------------------------------------------- helpers

fn sox_bin() -> &'static str {
    if Path::new(SOX_HOMEBREW).exists() { SOX_HOMEBREW } else { "sox" }
}

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/starter-kit/convert.sh")
}

/// Fresh scratch folder for one test.
fn case(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("convert_sh").join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn sox(args: &[&str]) {
    let out = Command::new(sox_bin()).args(args).output().expect("sox not found");
    assert!(
        out.status.success(),
        "sox {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn soxi(flag: &str, file: &Path) -> String {
    let out = Command::new(format!("{}i", sox_bin()))
        .args([flag, s(file)])
        .output()
        .expect("soxi not found");
    assert!(out.status.success(), "soxi {flag} {file:?} failed");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// 48 kHz 24-bit mono file from one `synth` spec plus optional effects.
fn mono(path: &Path, effects: &[&str]) {
    let mut a = vec!["-R", "-n", "-r", "48000", "-b", "24", "-c", "1", s(path)];
    a.extend_from_slice(effects);
    sox(&a);
}

/// Stereo file from two mono files (`sox -M`).
fn stereo(path: &Path, left: &Path, right: &Path) {
    sox(&["-R", "-M", s(left), s(right), "-b", "24", s(path)]);
}

/// Mono input for one sine, stereo output with the same sine in both channels (in phase).
fn stereo_sine(dir: &Path, name: &str, secs: &str, vol: &str) -> PathBuf {
    let l = dir.join(format!("{name}_l.wav"));
    mono(&l, &["synth", secs, "sine", "440", "vol", vol]);
    let out = dir.join(format!("{name}.wav"));
    stereo(&out, &l, &l);
    out
}

fn list_line(src: &Path, target: &str) -> String {
    format!("{} | {} | 8363 | -0.3", s(src), target)
}

/// Write the list, run the script, return the process output. Output folder is `<dir>/out`.
fn convert(dir: &Path, flags: &[&str], list: &str) -> Output {
    let list_file = dir.join("list.txt");
    fs::write(&list_file, list).unwrap();
    // /bin/bash is 3.2 on macOS: the script must work there. Same sox as the tests use.
    Command::new("/bin/bash")
        .env("SOX", sox_bin())
        .arg(script())
        .args(flags)
        .arg(&list_file)
        .arg(dir.join("out"))
        .output()
        .expect("could not start bash")
}

fn ok(out: &Output) {
    assert!(
        out.status.success(),
        "convert.sh failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn fails_with_message(out: &Output) {
    assert!(!out.status.success(), "expected non-zero exit");
    assert!(
        !String::from_utf8_lossy(&out.stderr).trim().is_empty(),
        "expected an error message on stderr"
    );
}

/// Non-zero exit and a message that names the list line and the reason.
fn fails_with(out: &Output, line: usize, reason: &str) {
    fails_with_message(out);
    let err = String::from_utf8_lossy(&out.stderr).to_lowercase();
    assert!(err.contains(&format!("line {line}")), "no 'line {line}' in: {err}");
    assert!(err.contains(&reason.to_lowercase()), "no '{reason}' in: {err}");
}

/// Decode any audio file to mono f64 in [-1, 1) through SoX (float output: no dither).
/// 8-bit unsigned decodes to (byte - 128) / 128, the same grid as `step_size(8)`.
fn decode(file: &Path) -> Vec<f64> {
    // Keep the decoded copy out of the folders the script writes to.
    let raw_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("convert_sh_raw");
    fs::create_dir_all(&raw_dir).unwrap();
    let raw = raw_dir.join(s(file).replace(['/', ' '], "_") + ".f32");
    sox(&["-R", "-D", s(file), "-t", "raw", "-e", "floating-point", "-b", "32", "-c", "1", "-L", s(&raw)]);
    fs::read(&raw)
        .unwrap()
        .chunks_exact(4)
        .map(|b| f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])))
        .collect()
}

/// The chain without the 8-bit step: eq. (1)-(4) in float. Reference for the dither tests.
fn reference(input: &Path) -> Vec<f64> {
    let raw = input.with_extension("ref.f32");
    sox(&[
        "-R", "-D", s(input), "-t", "raw", "-e", "floating-point", "-b", "32", "-c", "1", "-L",
        s(&raw), "remix", "-", "rate", "-v", "8363", "gain", "-n", "-0.3",
    ]);
    fs::read(&raw)
        .unwrap()
        .chunks_exact(4)
        .map(|b| f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])))
        .collect()
}

fn peak(x: &[f64]) -> f64 {
    x.iter().fold(0.0, |m, v| m.max(v.abs()))
}

/// Amplitude of a sine at `f` Hz (Hann-windowed single-bin DFT).
fn tone_amp(x: &[f64], f: f64) -> f64 {
    let n = x.len() as f64;
    let (mut re, mut im, mut sw) = (0.0, 0.0, 0.0);
    for (i, v) in x.iter().enumerate() {
        let w = 0.5 - 0.5 * (2.0 * PI * i as f64 / (n - 1.0)).cos();
        let ph = 2.0 * PI * f * i as f64 / RATE;
        re += v * w * ph.cos();
        im -= v * w * ph.sin();
        sw += w;
    }
    2.0 * (re * re + im * im).sqrt() / sw
}

fn db(a: f64, b: f64) -> f64 {
    20.0 * (a / b).log10()
}

// ------------------------------------------------------------------ format

#[test]
fn output_is_wav_8bit_unsigned_mono_8363() {
    let d = case("format");
    let src = stereo_sine(&d, "in", "1", "0.5");
    ok(&convert(&d, &[], &list_line(&src, "out.wav")));
    let f = d.join("out/out.wav");
    assert_eq!(soxi("-t", &f), "wav");
    assert_eq!(soxi("-r", &f), "8363");
    assert_eq!(soxi("-c", &f), "1");
    assert_eq!(soxi("-b", &f), "8");
    assert!(soxi("-e", &f).starts_with("Unsigned"), "encoding: {}", soxi("-e", &f));
}

#[test]
fn duration_matches_input_within_one_sample() {
    let d = case("length");
    let src = stereo_sine(&d, "in", "1.2345", "0.5");
    ok(&convert(&d, &[], &list_line(&src, "out.wav")));
    let n_in: f64 = soxi("-s", &src).parse().unwrap();
    let n_out: f64 = soxi("-s", &d.join("out/out.wav")).parse().unwrap();
    let diff = (n_out / RATE - n_in / 48000.0).abs();
    assert!(diff <= 1.0 / RATE, "duration differs by {diff} s");
}

// ----------------------------------------------------------- eq. (1) mono

#[test]
fn out_of_phase_stereo_gives_silence() {
    let d = case("phase");
    let l = d.join("l.wav");
    let r = d.join("r.wav");
    mono(&l, &["synth", "1", "sine", "440", "vol", "0.5"]);
    sox(&["-R", s(&l), "-b", "24", s(&r), "vol", "-1"]);
    let src = d.join("in.wav");
    stereo(&src, &l, &r);
    ok(&convert(&d, &["--no-dither"], &list_line(&src, "out.wav")));
    let y = decode(&d.join("out/out.wav"));
    assert!(peak(&y) <= step_size(8), "peak {} is more than one step", peak(&y));
}

#[test]
fn mono_mix_keeps_both_channels_at_equal_level() {
    // After eq. (4) the absolute level is always -0.3 dBFS, so "keeps its level" is
    // tested as: left 440 Hz and right 1000 Hz come out equally strong (eq. 1 averages;
    // a script that drops one channel would lose a tone).
    let d = case("mix");
    let l = d.join("l.wav");
    let r = d.join("r.wav");
    mono(&l, &["synth", "1", "sine", "440", "vol", "0.5"]);
    mono(&r, &["synth", "1", "sine", "1000", "vol", "0.5"]);
    let src = d.join("in.wav");
    stereo(&src, &l, &r);
    ok(&convert(&d, &[], &list_line(&src, "out.wav")));
    let y = decode(&d.join("out/out.wav"));
    let (a, b) = (tone_amp(&y, 440.0), tone_amp(&y, 1000.0));
    assert!(b > 0.1, "1000 Hz tone is missing (amplitude {b})");
    assert!(db(a, b).abs() < 1.0, "channels differ by {} dB", db(a, b));
}

// -------------------------------------------------- eq. (2)-(3) aliasing

#[test]
fn seven_khz_tone_leaves_no_1363_hz_alias() {
    // 440 Hz + 7000 Hz. A lone 7000 Hz tone would be filtered away and then
    // re-normalised by eq. (4), so the alias is measured against the 440 Hz tone.
    let d = case("alias");
    let a = d.join("a.wav");
    let b = d.join("b.wav");
    mono(&a, &["synth", "4", "sine", "440", "vol", "0.25"]);
    mono(&b, &["synth", "4", "sine", "7000", "vol", "0.25"]);
    let src = d.join("in.wav");
    sox(&["-R", "-m", s(&a), s(&b), "-b", "24", s(&src)]);
    ok(&convert(&d, &[], &list_line(&src, "out.wav")));
    let y = decode(&d.join("out/out.wav"));
    let (low, alias) = (tone_amp(&y, 440.0), tone_amp(&y, 1363.0));
    assert!(low > 0.5, "440 Hz tone too weak: {low}");
    assert!(db(low, alias) >= 60.0, "alias only {} dB down", db(low, alias));
}

// ----------------------------------------------------- eq. (4) normalise

#[test]
fn peak_is_minus_0_3_dbfs_within_one_step() {
    let d = case("peak");
    let src = stereo_sine(&d, "in", "1", "0.2");
    ok(&convert(&d, &["--no-dither"], &list_line(&src, "out.wav")));
    let y = decode(&d.join("out/out.wav"));
    let target = 10f64.powf(-0.3 / 20.0);
    assert!((peak(&y) - target).abs() <= step_size(8), "peak {} vs {target}", peak(&y));
}

// --------------------------------------------------- eq. (5)-(7) dither

/// 440 Hz, 1 s, decaying to silence, 0.01 of full scale on input.
fn quiet_decaying(d: &Path) -> PathBuf {
    let src = d.join("quiet.wav");
    mono(&src, &["synth", "1", "sine", "440", "fade", "l", "0", "1", "1", "vol", "0.01"]);
    src
}

#[test]
fn tpdf_dither_error_has_zero_mean_and_power_q2_over_4() {
    let d = case("dither_on");
    let src = quiet_decaying(&d);
    ok(&convert(&d, &[], &list_line(&src, "out.wav")));
    let y = decode(&d.join("out/out.wav"));
    let z = reference(&src);
    assert_eq!(y.len(), z.len(), "length differs from the float reference");

    let q = step_size(8);
    let n = y.len() as f64;
    let e: Vec<f64> = y.iter().zip(&z).map(|(a, b)| a - b).collect();
    let mean = e.iter().sum::<f64>() / n;
    let power = e.iter().map(|v| v * v).sum::<f64>() / n;

    // Prediction from pt2-dsp (eq. 7), averaged over the actual reference samples.
    let mut rng = Lcg::new(7);
    let predicted = z
        .iter()
        .step_by(8)
        .map(|&v| error_moments(v, q, Dither::Tpdf, 400, &mut rng).msq)
        .sum::<f64>()
        / (n / 8.0).ceil();

    assert!(mean.abs() < 0.05 * q, "mean {mean} is not about 0 (q = {q})");
    let want = q * q / 4.0;
    assert!((predicted / want - 1.0).abs() < 0.05, "pt2-dsp predicts {predicted}, want {want}");
    assert!((power / want - 1.0).abs() < 0.07, "power {power} vs q^2/4 = {want}");
}

#[test]
fn without_dither_output_is_plain_rounding_of_the_reference() {
    // Eq. (5): without dither the error is a fixed function of the signal.
    let d = case("dither_off");
    let src = quiet_decaying(&d);
    ok(&convert(&d, &["--no-dither"], &list_line(&src, "out.wav")));
    let y = decode(&d.join("out/out.wav"));
    let z = reference(&src);
    assert_eq!(y.len(), z.len());

    let q = step_size(8);
    let exact = y.iter().zip(&z).filter(|(a, b)| (**a - quant(**b, q)).abs() < 1e-9).count();
    let worst = y.iter().zip(&z).map(|(a, b)| (a - quant(*b, q)).abs()).fold(0.0, f64::max);
    assert!(worst <= q + 1e-9, "worst deviation from Q(z) is {worst}");
    assert!(exact as f64 >= 0.99 * y.len() as f64, "only {exact} of {} samples equal Q(z)", y.len());
}

#[test]
fn digital_silence_stays_silent() {
    let d = case("silence");
    let src = d.join("silence.wav");
    sox(&["-R", "-n", "-r", "48000", "-b", "24", "-c", "2", s(&src), "trim", "0", "1"]);
    ok(&convert(&d, &["--no-dither"], &list_line(&src, "off.wav")));
    assert_eq!(peak(&decode(&d.join("out/off.wav"))), 0.0, "silence without dither must be exact");
    let d2 = case("silence_dither");
    let src2 = d2.join("silence.wav");
    sox(&["-R", "-n", "-r", "48000", "-b", "24", "-c", "2", s(&src2), "trim", "0", "1"]);
    ok(&convert(&d2, &[], &list_line(&src2, "on.wav")));
    // Q(0 + d) with d in [-q, q) can only be -q, 0 or +q.
    assert!(peak(&decode(&d2.join("out/on.wav"))) <= step_size(8) + 1e-9);
}

// ------------------------------------------------------------ reproducible

#[test]
fn two_runs_give_byte_identical_files() {
    let d = case("repro");
    let src = quiet_decaying(&d);
    let line = list_line(&src, "out.wav");
    let a = d.join("a");
    let b = d.join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();
    ok(&convert(&a, &[], &line));
    ok(&convert(&b, &[], &line));
    assert_eq!(fs::read(a.join("out/out.wav")).unwrap(), fs::read(b.join("out/out.wav")).unwrap());
}

// ------------------------------------------------------------------ safety

#[test]
fn source_file_is_not_modified() {
    let d = case("source_safe");
    let src = quiet_decaying(&d);
    let bytes = fs::read(&src).unwrap();
    let mtime = fs::metadata(&src).unwrap().modified().unwrap();
    ok(&convert(&d, &[], &list_line(&src, "out.wav")));
    ok(&convert(&d, &["--no-dither"], &list_line(&src, "out2.wav")));
    assert_eq!(fs::read(&src).unwrap(), bytes, "source bytes changed");
    assert_eq!(fs::metadata(&src).unwrap().modified().unwrap(), mtime, "source mtime changed");
}

#[test]
fn blank_lines_comments_and_spacing_are_accepted() {
    let d = case("list_syntax");
    let src = quiet_decaying(&d);
    let list = format!("# comment\n\n   \n  {} |out.wav|  8363 |-0.3  \n# end\n", s(&src));
    ok(&convert(&d, &[], &list));
    assert!(d.join("out/out.wav").exists());
}

#[test]
fn line_with_missing_fields_fails_with_message() {
    let d = case("bad_fields");
    let src = quiet_decaying(&d);
    fails_with_message(&convert(&d, &[], &format!("{} | out.wav | 8363\n", s(&src))));
}

#[test]
fn unreadable_source_fails_with_message() {
    let d = case("bad_source");
    let missing = d.join("does_not_exist.wav");
    fails_with_message(&convert(&d, &[], &list_line(&missing, "out.wav")));
}

#[test]
fn target_containing_a_path_fails_with_message() {
    let d = case("bad_target");
    let src = quiet_decaying(&d);
    fails_with_message(&convert(&d, &[], &list_line(&src, "../escape.wav")));
    fails_with_message(&convert(&d, &[], &list_line(&src, "sub/out.wav")));
    assert!(!d.join("escape.wav").exists());
}

// ------------------------------------- review fixes: flags, values, targets

fn dir_is_empty(dir: &Path) -> bool {
    !dir.exists() || fs::read_dir(dir).unwrap().next().is_none()
}

/// The chain run directly with SoX. Pins every flag of the script, including -R.
fn direct_sox(src: &Path, out: &Path, dither: bool, rate: &str, gain: &str) {
    let mut line = vec!["-R".to_string()];
    if !dither {
        line.push("-D".into());
    }
    line.push(s(src).into());
    line.extend(["-b", "8", "-e", "unsigned-integer", "-t", "wav"].map(String::from));
    line.push(s(out).into());
    line.extend(["remix", "-", "rate", "-v", rate, "gain", "-n", gain].map(String::from));
    if dither {
        line.push("dither".into());
    }
    let refs: Vec<&str> = line.iter().map(String::as_str).collect();
    sox(&refs);
}

#[test]
fn script_output_equals_the_direct_sox_chain_byte_for_byte() {
    let d = case("golden");
    let src = stereo_sine(&d, "in", "1", "0.5");
    for (flags, dither, name) in [(vec![], true, "on.wav"), (vec!["--no-dither"], false, "off.wav")] {
        ok(&convert(&d, &flags, &list_line(&src, name)));
        let direct = d.join(format!("direct_{name}"));
        direct_sox(&src, &direct, dither, "8363", "-0.3");
        assert_eq!(
            fs::read(d.join("out").join(name)).unwrap(),
            fs::read(&direct).unwrap(),
            "{name} differs from the direct sox chain"
        );
    }
}

#[test]
fn dither_seed_is_fixed_even_when_runs_are_a_second_apart() {
    // SoX seeds its dither from the clock unless -R is given; a gap of over a second
    // makes a missing -R show up.
    let d = case("repro_gap");
    let src = quiet_decaying(&d);
    let line = list_line(&src, "out.wav");
    let a = d.join("a");
    let b = d.join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();
    ok(&convert(&a, &[], &line));
    std::thread::sleep(std::time::Duration::from_millis(1100));
    ok(&convert(&b, &[], &line));
    assert_eq!(fs::read(a.join("out/out.wav")).unwrap(), fs::read(b.join("out/out.wav")).unwrap());
}

#[test]
fn rate_and_gain_come_from_the_list() {
    let d = case("rate_gain");
    let src = stereo_sine(&d, "in", "1", "0.5");
    let line = format!("{} | out.wav | 11025 | -6", s(&src));
    ok(&convert(&d, &["--no-dither"], &line));
    let f = d.join("out/out.wav");
    assert_eq!(soxi("-r", &f), "11025");
    let y = decode(&f);
    let target = 10f64.powf(-6.0 / 20.0);
    assert!((peak(&y) - target).abs() <= step_size(8), "peak {} vs {target}", peak(&y));
}

#[test]
fn rate_zero_and_positive_gain_are_rejected() {
    let d = case("bad_values");
    let src = quiet_decaying(&d);
    fails_with_message(&convert(&d, &[], &format!("{} | out.wav | 0 | -0.3", s(&src))));
    fails_with_message(&convert(&d, &[], &format!("{} | out.wav | 8363 | 3", s(&src))));
    fails_with_message(&convert(&d, &[], &format!("{} | out.wav | 8363 | 0.5", s(&src))));
}

#[test]
fn failing_sox_run_leaves_no_files_in_the_out_dir() {
    let d = case("atomic");
    let good = quiet_decaying(&d);
    let bad = d.join("x.wav");
    fs::write(&bad, "this is not audio").unwrap();
    let list = format!("{}\n{}\n", list_line(&good, "a.wav"), list_line(&bad, "b.wav"));
    fails_with_message(&convert(&d, &[], &list));
    assert!(dir_is_empty(&d.join("out")), "files left behind: {:?}", fs::read_dir(d.join("out")).unwrap().collect::<Vec<_>>());
}

#[test]
fn failing_run_does_not_touch_an_existing_good_output() {
    let d = case("atomic_keep");
    let first = quiet_decaying(&d);
    ok(&convert(&d, &[], &list_line(&first, "a.wav")));
    let before = fs::read(d.join("out/a.wav")).unwrap();

    let other = stereo_sine(&d, "other", "1", "0.5");
    let bad = d.join("x.wav");
    fs::write(&bad, "this is not audio").unwrap();
    let list = format!("{}\n{}\n", list_line(&other, "a.wav"), list_line(&bad, "b.wav"));
    fails_with_message(&convert(&d, &[], &list));
    assert_eq!(fs::read(d.join("out/a.wav")).unwrap(), before, "good output was changed or removed");
}

#[test]
fn target_must_be_a_wav_file_name() {
    let d = case("target_ext");
    let src = quiet_decaying(&d);
    for t in ["foo", "foo.aif", "foo.txt", "-x.txt"] {
        fails_with_message(&convert(&d, &[], &list_line(&src, t)));
    }
}

#[test]
fn target_starting_with_dash_is_fine_when_it_ends_in_wav() {
    let d = case("target_dash");
    let src = quiet_decaying(&d);
    ok(&convert(&d, &[], &list_line(&src, "-kick.wav")));
    assert!(d.join("out/-kick.wav").exists());
}

#[test]
fn duplicate_targets_are_rejected_even_when_only_the_case_differs() {
    let d = case("dupes");
    let src = quiet_decaying(&d);
    let same = format!("{}\n{}\n", list_line(&src, "a.wav"), list_line(&src, "a.wav"));
    fails_with_message(&convert(&d, &[], &same));
    let case_only = format!("{}\n{}\n", list_line(&src, "a.wav"), list_line(&src, "A.wav"));
    fails_with_message(&convert(&d, &[], &case_only));
    assert!(dir_is_empty(&d.join("out")));
}

#[test]
fn a_target_that_is_the_source_file_is_refused() {
    let d = case("overwrite_source");
    let orig = quiet_decaying(&d);
    fs::create_dir_all(d.join("out")).unwrap();
    let inside = d.join("out/same.wav");
    fs::copy(&orig, &inside).unwrap();
    let bytes = fs::read(&inside).unwrap();
    fails_with_message(&convert(&d, &[], &list_line(&inside, "same.wav")));
    assert_eq!(fs::read(&inside).unwrap(), bytes, "the source was overwritten");
}

// ------------------------------------------- review fixes: remaining gaps

/// Run the script by hand: `convert.sh <args...>`, used where the argument list is the test.
fn run_script(args: &[&str]) -> Output {
    Command::new("/bin/bash")
        .env("SOX", sox_bin())
        .arg(script())
        .args(args)
        .output()
        .expect("could not start bash")
}

/// Parsed 8-bit WAV: (format tag, channels, rate, bits, data bytes).
fn wav_parts(bytes: &[u8]) -> (u16, u16, u32, u16, Vec<u8>) {
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let (mut fmt, mut data) = ((0, 0, 0, 0), Vec::new());
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        let body = &bytes[at + 8..(at + 8 + len).min(bytes.len())];
        if id == b"fmt " {
            fmt = (
                u16::from_le_bytes([body[0], body[1]]),
                u16::from_le_bytes([body[2], body[3]]),
                u32::from_le_bytes(body[4..8].try_into().unwrap()),
                u16::from_le_bytes([body[14], body[15]]),
            );
        } else if id == b"data" {
            data = body.to_vec();
        }
        at += 8 + len + (len & 1);
    }
    (fmt.0, fmt.1, fmt.2, fmt.3, data)
}

#[test]
fn wav_header_is_plain_pcm_8bit_mono_8363_and_silence_is_0x80() {
    // Checked without SoX, so the file is not judged by the tool that wrote it.
    let d = case("wav_header");
    let src = d.join("silence.wav");
    sox(&["-R", "-n", "-r", "48000", "-b", "24", "-c", "2", s(&src), "trim", "0", "1"]);
    ok(&convert(&d, &["--no-dither"], &list_line(&src, "s.wav")));
    let (tag, ch, rate, bits, data) = wav_parts(&fs::read(d.join("out/s.wav")).unwrap());
    assert_eq!((tag, ch, rate, bits), (1, 1, 8363, 8));
    assert_eq!(data.len(), 8363, "one second of 8363 Hz");
    assert!(data.iter().all(|&b| b == 0x80), "silence must be all 0x80");
}

#[test]
fn tpdf_error_power_is_q2_over_4_for_every_position_between_steps() {
    // Eq. (7): E[nu^2 | z] = q^2/4 for every z. Group the samples by the position of z
    // inside a step. One uniform value (RPDF) would give a different power per group.
    let d = case("dither_bins");
    let src = d.join("quiet.wav");
    mono(&src, &["synth", "4", "sine", "440", "fade", "l", "0", "4", "4", "vol", "0.01"]);
    ok(&convert(&d, &[], &list_line(&src, "out.wav")));
    let y = decode(&d.join("out/out.wav"));
    let z = reference(&src);
    assert_eq!(y.len(), z.len());
    let q = step_size(8);
    let (mut sum, mut cnt) = ([0.0f64; 4], [0usize; 4]);
    for (a, b) in y.iter().zip(&z) {
        let t = b / q;
        let frac = t - t.floor();
        let bin = ((frac * 4.0) as usize).min(3);
        sum[bin] += (a - b) * (a - b);
        cnt[bin] += 1;
    }
    let want = q * q / 4.0;
    for k in 0..4 {
        assert!(cnt[k] > 2000, "bin {k} has only {} samples", cnt[k]);
        let p = sum[k] / cnt[k] as f64;
        assert!((p / want - 1.0).abs() < 0.10, "bin {k}: power {p} vs q^2/4 = {want}");
    }
}

#[test]
fn other_tones_above_nyquist_alias_nowhere() {
    // Eq. (3): 4400 -> 3963, 5000 -> 3363, 12000 -> 3637, 20000 -> 3274 Hz.
    for hi in [4400.0, 5000.0, 12000.0, 20000.0] {
        let d = case(&format!("alias_{hi}"));
        let a = d.join("a.wav");
        let b = d.join("b.wav");
        mono(&a, &["synth", "4", "sine", "440", "vol", "0.25"]);
        mono(&b, &["synth", "4", "sine", &hi.to_string(), "vol", "0.25"]);
        let src = d.join("in.wav");
        sox(&["-R", "-m", s(&a), s(&b), "-b", "24", s(&src)]);
        ok(&convert(&d, &[], &list_line(&src, "out.wav")));
        let y = decode(&d.join("out/out.wav"));
        let fa = pt2_dsp::alias_of(hi, RATE).fa;
        let (low, alias) = (tone_amp(&y, 440.0), tone_amp(&y, fa));
        assert!(db(low, alias) >= 60.0, "{hi} Hz -> {fa} Hz only {} dB down", db(low, alias));
    }
}

#[test]
fn full_scale_stereo_input_does_not_clip() {
    let d = case("hot");
    let l = d.join("l.wav");
    mono(&l, &["synth", "1", "sine", "440", "vol", "0.999"]);
    let src = d.join("in.wav");
    stereo(&src, &l, &l);
    let out = convert(&d, &["--no-dither"], &list_line(&src, "out.wav"));
    ok(&out);
    let err = String::from_utf8_lossy(&out.stderr).to_lowercase();
    assert!(!err.contains("clip"), "sox reported clipping: {err}");
    let y = decode(&d.join("out/out.wav"));
    assert!((peak(&y) - 10f64.powf(-0.3 / 20.0)).abs() <= step_size(8));
}

#[test]
fn mono_input_is_accepted() {
    let d = case("mono_in");
    let src = d.join("in.wav");
    mono(&src, &["synth", "1", "sine", "440", "vol", "0.5"]);
    ok(&convert(&d, &["--no-dither"], &list_line(&src, "out.wav")));
    let y = decode(&d.join("out/out.wav"));
    assert!((peak(&y) - 10f64.powf(-0.3 / 20.0)).abs() <= step_size(8));
}

#[test]
fn paths_with_spaces_and_relative_sources_work() {
    let d = case("spaces");
    let dir = d.join("my dir");
    fs::create_dir_all(&dir).unwrap();
    let src = dir.join("in put.wav");
    mono(&src, &["synth", "1", "sine", "440", "vol", "0.5"]);
    let list = dir.join("my list.txt");
    fs::write(&list, "in put.wav | out.wav | 8363 | -0.3\n").unwrap(); // relative to the list
    let outdir = d.join("out dir");
    ok(&run_script(&[s(&list), s(&outdir)]));
    assert!(outdir.join("out.wav").exists());
}

#[test]
fn pipe_in_a_source_path_is_reported_as_too_many_fields() {
    let d = case("pipe_path");
    let src = d.join("a|b.wav");
    mono(&src, &["synth", "1", "sine", "440"]);
    fails_with(&convert(&d, &[], &list_line(&src, "out.wav")), 1, "too many fields");
}

#[test]
fn empty_out_dir_and_misplaced_flag_are_usage_errors() {
    let d = case("usage");
    let src = quiet_decaying(&d);
    let list = d.join("list.txt");
    fs::write(&list, list_line(&src, "out.wav")).unwrap();
    let out = run_script(&[s(&list), ""]);
    fails_with_message(&out);
    assert!(String::from_utf8_lossy(&out.stderr).contains("out-dir"));
    let out = run_script(&[s(&list), s(&d.join("out")), "--no-dither"]);
    fails_with_message(&out);
    assert!(String::from_utf8_lossy(&out.stderr).to_lowercase().contains("usage"));
}

#[test]
fn crlf_list_and_utf8_bom_are_accepted() {
    let d = case("crlf_bom");
    let src = quiet_decaying(&d);
    let line = list_line(&src, "out.wav");
    ok(&convert(&d, &[], &format!("# c\r\n{line}\r\n")));
    assert!(d.join("out/out.wav").exists());
    let d2 = case("bom");
    let src2 = quiet_decaying(&d2);
    ok(&convert(&d2, &[], &format!("\u{feff}{}\n", list_line(&src2, "out.wav"))));
    assert!(d2.join("out/out.wav").exists());
}

#[test]
fn error_messages_name_the_line_and_the_reason() {
    let d = case("messages");
    let src = quiet_decaying(&d);
    let p = s(&src);
    let cases: [(String, &str); 7] = [
        (format!("{p} | o.wav | abc | -0.3"), "rate"),
        (format!("{p} | o.wav | 8363.5 | -0.3"), "rate"),
        (format!("{p} | o.wav | 8363 | loud"), "gain"),
        (format!("{p} | o.wav | 8363 | -0.3 | extra"), "too many fields"),
        (format!("{p} | o.wav | 8363 | -0.3 |"), "too many fields"),
        (format!("{p} | o.wav | 8363"), "missing field"),
        (format!("{} | o.wav | 8363 | -0.3", s(&d)), "cannot read source"),
    ];
    for (line, reason) in cases {
        let out = convert(&d, &[], &format!("# first line is a comment\n{line}\n"));
        fails_with(&out, 2, reason);
        assert!(dir_is_empty(&d.join("out")), "files left after: {line}");
    }
}

#[test]
fn empty_source_file_fails_and_leaves_nothing() {
    let d = case("empty_source");
    let empty = d.join("empty.wav");
    fs::write(&empty, "").unwrap();
    fails_with_message(&convert(&d, &[], &list_line(&empty, "out.wav")));
    assert!(dir_is_empty(&d.join("out")));
}
