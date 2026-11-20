pub const CYAN: (u8, u8, u8) = (0x00, 0xe5, 0xff);
pub const PINK: (u8, u8, u8) = (0xff, 0x3d, 0xc8);
pub const GREEN: (u8, u8, u8) = (0x39, 0xff, 0x88);
pub const DIM: (u8, u8, u8) = (0x60, 0x6a, 0x80);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_matches_documented_rgb_values() {
        assert_eq!(CYAN, (0, 229, 255));
        assert_eq!(PINK, (255, 61, 200));
        assert_eq!(GREEN, (57, 255, 136));
        assert_eq!(DIM, (96, 106, 128));
    }
}
