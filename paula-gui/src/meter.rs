use paula::modfile::Cell;
use paula::Spectra;

pub const METER_FLOOR_DB: f32 = -48.0;
pub const METER_CEIL_DB: f32 = 0.0;
pub const PEAK_HOLD_SECS: f32 = 1.5;
const PEAK_DECAY_DB_PER_SEC: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelMeter {
    pub rms_db: f32,
    pub peak_db: f32,
    pub hold_remaining: f32,
    pub clipped: bool,
}

impl Default for ChannelMeter {
    fn default() -> Self {
        Self {
            rms_db: METER_FLOOR_DB,
            peak_db: METER_FLOOR_DB,
            hold_remaining: 0.0,
            clipped: false,
        }
    }
}

impl ChannelMeter {
    fn update(&mut self, rms_lin: f32, peak_lin: f32, dt_secs: f32, sample_clipped: bool) {
        let dt = dt_secs.max(0.0);
        self.rms_db = linear_to_dbfs(rms_lin);
        if sample_clipped || peak_lin >= 1.0 {
            self.clipped = true;
        }
        let inst_peak_db = linear_to_dbfs(peak_lin);
        if inst_peak_db >= self.peak_db {
            self.peak_db = inst_peak_db;
            self.hold_remaining = PEAK_HOLD_SECS;
        } else if self.hold_remaining > dt {
            self.hold_remaining -= dt;
        } else {
            let decay_dt = dt - self.hold_remaining;
            self.hold_remaining = 0.0;
            self.peak_db = (self.peak_db - PEAK_DECAY_DB_PER_SEC * decay_dt)
                .max(inst_peak_db)
                .max(METER_FLOOR_DB);
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MasterMeter {
    pub left: ChannelMeter,
    pub right: ChannelMeter,
}

pub fn linear_to_dbfs(amp: f32) -> f32 {
    if !amp.is_finite() || amp <= 0.0 {
        return METER_FLOOR_DB;
    }
    (20.0 * amp.log10()).clamp(METER_FLOOR_DB, METER_CEIL_DB)
}

pub fn dbfs_to_norm(db: f32) -> f32 {
    ((db - METER_FLOOR_DB) / (METER_CEIL_DB - METER_FLOOR_DB)).clamp(0.0, 1.0)
}

impl MasterMeter {
    /// Update L/R RMS and peak from rendered interleaved i16 stereo samples and frame delta time.
    pub fn update_from_pcm(&mut self, stereo_i16: &[i16], dt_secs: f32) {
        let frames = stereo_i16.len() / 2;
        if frames == 0 {
            self.left.update(0.0, 0.0, dt_secs, false);
            self.right.update(0.0, 0.0, dt_secs, false);
            return;
        }

        let mut sum_sq_l = 0.0_f64;
        let mut sum_sq_r = 0.0_f64;
        let mut peak_l = 0.0_f32;
        let mut peak_r = 0.0_f32;
        let mut clip_l = false;
        let mut clip_r = false;

        for frame in stereo_i16.chunks_exact(2) {
            let l_raw = frame[0];
            let r_raw = frame[1];
            if l_raw.unsigned_abs() >= 32767 {
                clip_l = true;
            }
            if r_raw.unsigned_abs() >= 32767 {
                clip_r = true;
            }
            let l = l_raw as f32 / 32768.0;
            let r = r_raw as f32 / 32768.0;
            sum_sq_l += (l * l) as f64;
            sum_sq_r += (r * r) as f64;
            peak_l = peak_l.max(l.abs());
            peak_r = peak_r.max(r.abs());
        }

        let rms_l = (sum_sq_l / frames as f64).sqrt() as f32;
        let rms_r = (sum_sq_r / frames as f64).sqrt() as f32;
        self.left.update(rms_l, peak_l, dt_secs, clip_l);
        self.right.update(rms_r, peak_r, dt_secs, clip_r);
    }

    /// Update L/R meters from per-channel Spectra levels when raw PCM is unavailable.
    /// ProTracker Paula panning: channels 0 and 3 are left, channels 1 and 2 are right.
    pub fn update_from_spectra(&mut self, spectra: &Spectra, muted: [bool; 4], dt_secs: f32) {
        let ch_lvl = |ch: usize| if muted[ch] { 0.0 } else { spectra.levels[ch].clamp(0.0, 1.0) };
        let l0 = ch_lvl(0);
        let l3 = ch_lvl(3);
        let r1 = ch_lvl(1);
        let r2 = ch_lvl(2);

        let rms_l = (l0 * l0 + l3 * l3).sqrt().min(1.0);
        let rms_r = (r1 * r1 + r2 * r2).sqrt().min(1.0);
        let peak_l = l0.max(l3);
        let peak_r = r1.max(r2);

        self.left.update(rms_l, peak_l, dt_secs, false);
        self.right.update(rms_r, peak_r, dt_secs, false);
    }

    /// Clear sticky clip lights (called only when the user clicks the clip indicator).
    pub fn clear_clip(&mut self) {
        self.left.clipped = false;
        self.right.clipped = false;
    }
}

/// Compute sidechain gain reduction (0.0 = no dip, 1.0 = full dip) for a channel
/// from its current Cxx volume effect relative to nominal volume (64).
pub fn sidechain_dip(cell: &Cell, nominal_vol: u8) -> f32 {
    if cell.cmd != 0xC {
        return 0.0;
    }
    let nom = nominal_vol.clamp(1, 64) as f32;
    let vol = cell.param.min(64) as f32;
    ((nom - vol) / nom).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_to_dbfs_clamps_to_floor_and_ceiling() {
        assert_eq!(linear_to_dbfs(0.0), -48.0);
        assert_eq!(linear_to_dbfs(-0.5), -48.0);
        assert_eq!(linear_to_dbfs(1.0), 0.0);
        assert_eq!(linear_to_dbfs(2.0), 0.0);
        assert!((linear_to_dbfs(0.5) - (-6.0206)).abs() < 0.01);
        assert_eq!(dbfs_to_norm(-48.0), 0.0);
        assert_eq!(dbfs_to_norm(0.0), 1.0);
    }

    #[test]
    fn peak_hold_lasts_1_5_seconds_then_decays() {
        let mut meter = MasterMeter::default();
        meter.update_from_pcm(&[16384, 16384], 0.016);
        let held_peak = meter.left.peak_db;
        assert!(held_peak > -7.0 && held_peak < -5.5);

        meter.update_from_pcm(&[0, 0], 1.49);
        assert_eq!(meter.left.peak_db, held_peak);

        meter.update_from_pcm(&[0, 0], 0.05);
        assert!(meter.left.peak_db < held_peak);
    }

    #[test]
    fn clip_light_stays_lit_until_clicked() {
        let mut meter = MasterMeter::default();
        meter.update_from_pcm(&[32767, 1000], 0.016);
        assert!(meter.left.clipped);
        assert!(!meter.right.clipped);

        meter.update_from_pcm(&[0, 0], 2.0);
        assert!(meter.left.clipped);

        meter.clear_clip();
        assert!(!meter.left.clipped);
        assert!(!meter.right.clipped);
    }

    #[test]
    fn sidechain_dip_tracks_volume_drop() {
        let half = Cell { period: 0, sample: 0, cmd: 0xC, param: 0x20 };
        let full = Cell { period: 0, sample: 0, cmd: 0xC, param: 0x40 };
        let none = Cell::default();
        assert!((sidechain_dip(&half, 64) - 0.5).abs() < 1e-6);
        assert_eq!(sidechain_dip(&full, 64), 0.0);
        assert_eq!(sidechain_dip(&none, 64), 0.0);
    }
}
