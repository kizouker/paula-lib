//! Builds the unmodified pt2-clone engine files plus our bridge into a static library.
//! SDL2 is needed for its headers only (types used by pt2_audio.c); libSDL2 is never linked.
//! Header search order: SDL2_INCLUDE_DIR, then `pkg-config --cflags sdl2`.

use std::process::Command;

fn sdl2_include_dirs() -> Vec<String> {
    if let Ok(dir) = std::env::var("SDL2_INCLUDE_DIR") {
        // the bridge includes <SDL2/SDL.h>, so pass the parent of the SDL2 folder
        return vec![dir];
    }
    let out = Command::new("pkg-config").args(["--cflags-only-I", "sdl2"]).output();
    match out {
        Ok(o) if o.status.success() => {
            let flags = String::from_utf8_lossy(&o.stdout).to_string();
            let mut dirs: Vec<String> = flags
                .split_whitespace()
                .filter_map(|f| f.strip_prefix("-I").map(str::to_string))
                .collect();
            // pkg-config usually gives .../include/SDL2; <SDL2/SDL.h> needs .../include as well
            let parents: Vec<String> = dirs
                .iter()
                .filter_map(|d| d.strip_suffix("/SDL2").map(str::to_string))
                .collect();
            dirs.extend(parents);
            dirs
        }
        _ => panic!(
            "SDL2 headers not found. Install them (macOS: brew install sdl2 pkg-config; \
             Debian/Ubuntu: sudo apt install libsdl2-dev pkg-config) or set SDL2_INCLUDE_DIR."
        ),
    }
}

fn main() {
    let src = "../src";
    let mut b = cc::Build::new();
    for dir in sdl2_include_dirs() {
        b.include(dir);
    }
    b.include(src)
        .opt_level(2)
        .warnings(false)
        .file("c_src/bridge.c"); // includes ../src/pt2_audio.c
    for f in ["pt2_replayer", "pt2_paula", "pt2_blep", "pt2_rcfilters", "pt2_downsample2x", "pt2_tables", "pt2_structs"] {
        b.file(format!("{src}/{f}.c"));
    }
    b.compile("paulacore");
    println!("cargo:rerun-if-changed=c_src/bridge.c");
    println!("cargo:rerun-if-env-changed=SDL2_INCLUDE_DIR");
    println!("cargo:rustc-link-lib=m");
}
