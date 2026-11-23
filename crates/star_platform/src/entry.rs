//! The entry point of games on systems that do not call `main`.
//!
//! # Why this is the one place with `unsafe`
//!
//! `unsafe` is forbidden in ProjectStar. This macro is the single exception
//! the project owner agreed to (see `CLAUDE.md`, section 3.1), and only for
//! this reason:
//!
//! - Android does not start a program by calling `main`. Its NativeActivity
//!   glue loads the game as a shared library and looks up, by name, a C
//!   function called exactly `android_main`.
//! - Rust renames ("mangles") every function in the library unless it is
//!   marked `#[no_mangle]`, so without it the system finds nothing and the
//!   game never starts.
//! - Edition 2024 writes it `#[unsafe(no_mangle)]`: two exported symbols
//!   with the same name would be undefined behavior, which the compiler
//!   cannot check, so the attribute counts as `unsafe` code.
//! - The symbol has to be in the game's own library, not in the engine, so
//!   the macro expands there; the `allow` covers that one function only.
//!
//! The function does nothing unsafe itself: it only passes the app to
//! [`crate::android::start`]. No other unsafe code may be added here.

/// Defines the function Android starts a game with, `android_main`, which
/// runs `$run` (a `fn() -> Result<(), E>` with a displayable error) in the
/// app the system gives. Elsewhere it defines nothing, so a game writes it
/// once in its library and keeps its `main` for the other systems:
///
/// ```ignore
/// // lib.rs, a `cdylib` and `rlib` library
/// pub fn run() -> Result<(), Box<dyn std::error::Error>> { /* … */ }
/// star_engine::android_main!(run);
/// ```
///
/// It is the only use of `unsafe` the project allows; the module
/// documentation explains why.
#[macro_export]
macro_rules! android_main {
    ($run:path) => {
        #[cfg(target_os = "android")]
        #[allow(unsafe_code)]
        // SAFETY: `#[unsafe(no_mangle)]` exports this function as the C
        // symbol `android_main`, the one name the NativeActivity glue
        // looks for. It is sound because the macro is meant to be used
        // once per game, so no other symbol in the library has that name,
        // and the signature is the one the glue calls (an `AndroidApp`,
        // no result). The body is safe code.
        #[unsafe(no_mangle)]
        fn android_main(app: $crate::android::AndroidApp) {
            $crate::android::start(app, $run);
        }
    };
}
