//! The vendored SWTOS system image.
//!
//! The image is a copy, not a build product. `sw-tos/build/` and its PL/SW
//! toolchain are both gitignored, so CI can never produce this artifact; see
//! `assets/PROVENANCE.md` for where it came from and
//! `scripts/refresh-image.sh` for how to update it.

/// The preemptive-multitasking image, loaded at address 0.
pub const PROGRAM: &[u8] = include_bytes!("../../../assets/program.bin");

/// Identity of the image, taken from the debug map it was built alongside.
/// The SWTOS debugger's identity opcode returns a CRC of the image's
/// immutable range, and this is the value it must agree with.
pub const BUILD_ID: &str = "crc24:d10c7a";

/// Byte length the debug map records for the image.
pub const IMAGE_SIZE: usize = 28308;

/// SHA-256 the debug map records for the image.
pub const IMAGE_SHA256: &str = "acc21b3f6dc57843b07b3a4682a9792b4143a680c43f3f039fe9dedc9e55105f";

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    /// The debug map is not compiled into the library -- at 1.6 MB it would
    /// dwarf the WASM bundle for a feature the debugger pane does not need
    /// until later. It is embedded here, in the test binary only, so the
    /// vendored pair can still be proven consistent.
    const DEBUG_MAP: &str = include_str!("../../../assets/program.debug.json");

    fn map() -> serde_json::Value {
        serde_json::from_str(DEBUG_MAP).expect("program.debug.json parses")
    }

    #[test]
    fn image_matches_the_debug_map_it_was_built_with() {
        let map = map();
        assert_eq!(map["format"], "swtos-debug-v1");
        assert_eq!(map["build_id"], BUILD_ID);
        assert_eq!(map["image_size"], IMAGE_SIZE);
        assert_eq!(map["image_sha256"], IMAGE_SHA256);
    }

    #[test]
    fn embedded_image_has_the_recorded_size_and_digest() {
        assert_eq!(PROGRAM.len(), IMAGE_SIZE);
        assert_eq!(format!("{:x}", Sha256::digest(PROGRAM)), IMAGE_SHA256);
    }
}
