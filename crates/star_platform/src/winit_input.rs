//! Translation of `winit` input events into [`InputEvent`]s.

use winit::event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::input_event::{ButtonState, InputEvent, Key, MouseButton};

/// Pixels of a touchpad scroll that count as one line of a mouse wheel, so
/// both devices scroll at a similar speed.
const PIXELS_PER_WHEEL_LINE: f32 = 20.0;

/// The input event behind a window event, if it is an input event the engine
/// knows about.
pub(crate) fn translate_window_event(event: &WindowEvent) -> Option<InputEvent> {
    match event {
        WindowEvent::KeyboardInput {
            event,
            is_synthetic: false,
            ..
        } => translate_key_event(event),
        WindowEvent::MouseInput { state, button, .. } => {
            translate_mouse_button(*button).map(|button| InputEvent::MouseButton {
                button,
                state: translate_state(*state),
            })
        }
        WindowEvent::CursorMoved { position, .. } => Some(InputEvent::CursorMoved {
            x: pixels_to_f32(position.x),
            y: pixels_to_f32(position.y),
        }),
        WindowEvent::CursorLeft { .. } => Some(InputEvent::CursorLeft),
        WindowEvent::MouseWheel { delta, .. } => Some(translate_scroll(*delta)),
        WindowEvent::Focused(false) => Some(InputEvent::FocusLost),
        _ => None,
    }
}

/// Synthetic events (keys held while the window gains or loses focus) are
/// skipped: losing focus is reported as [`InputEvent::FocusLost`] instead.
fn translate_key_event(event: &KeyEvent) -> Option<InputEvent> {
    if event.repeat {
        return None;
    }
    let PhysicalKey::Code(code) = event.physical_key else {
        return None;
    };
    translate_key_code(code).map(|key| InputEvent::Key {
        key,
        state: translate_state(event.state),
    })
}

fn translate_state(state: ElementState) -> ButtonState {
    match state {
        ElementState::Pressed => ButtonState::Pressed,
        ElementState::Released => ButtonState::Released,
    }
}

fn translate_mouse_button(button: winit::event::MouseButton) -> Option<MouseButton> {
    match button {
        winit::event::MouseButton::Left => Some(MouseButton::Left),
        winit::event::MouseButton::Right => Some(MouseButton::Right),
        winit::event::MouseButton::Middle => Some(MouseButton::Middle),
        winit::event::MouseButton::Back => Some(MouseButton::Back),
        winit::event::MouseButton::Forward => Some(MouseButton::Forward),
        winit::event::MouseButton::Other(_) => None,
    }
}

fn translate_scroll(delta: MouseScrollDelta) -> InputEvent {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => InputEvent::MouseWheel { x, y },
        MouseScrollDelta::PixelDelta(pixels) => InputEvent::MouseWheel {
            x: pixels_to_f32(pixels.x) / PIXELS_PER_WHEEL_LINE,
            y: pixels_to_f32(pixels.y) / PIXELS_PER_WHEEL_LINE,
        },
    }
}

// Window coordinates are far below `f32`'s exact integer range, so nothing
// that matters is lost.
#[allow(clippy::cast_possible_truncation)]
fn pixels_to_f32(pixels: f64) -> f32 {
    pixels as f32
}

/// Maps `winit` key codes to [`Key`]s; codes the engine does not know are ignored.
macro_rules! translate_key_codes {
    ($code:expr, { $($winit:ident => $ours:ident),+ $(,)? }) => {
        match $code {
            $(KeyCode::$winit => Some(Key::$ours),)+
            _ => None,
        }
    };
}

fn translate_key_code(code: KeyCode) -> Option<Key> {
    translate_key_codes!(code, {
        KeyA => A, KeyB => B, KeyC => C, KeyD => D, KeyE => E, KeyF => F,
        KeyG => G, KeyH => H, KeyI => I, KeyJ => J, KeyK => K, KeyL => L,
        KeyM => M, KeyN => N, KeyO => O, KeyP => P, KeyQ => Q, KeyR => R,
        KeyS => S, KeyT => T, KeyU => U, KeyV => V, KeyW => W, KeyX => X,
        KeyY => Y, KeyZ => Z,
        Digit0 => Digit0, Digit1 => Digit1, Digit2 => Digit2, Digit3 => Digit3,
        Digit4 => Digit4, Digit5 => Digit5, Digit6 => Digit6, Digit7 => Digit7,
        Digit8 => Digit8, Digit9 => Digit9,
        F1 => F1, F2 => F2, F3 => F3, F4 => F4, F5 => F5, F6 => F6,
        F7 => F7, F8 => F8, F9 => F9, F10 => F10, F11 => F11, F12 => F12,
        ArrowUp => ArrowUp, ArrowDown => ArrowDown,
        ArrowLeft => ArrowLeft, ArrowRight => ArrowRight,
        Space => Space, Enter => Enter, Escape => Escape, Tab => Tab,
        Backspace => Backspace, Delete => Delete, Insert => Insert,
        Home => Home, End => End, PageUp => PageUp, PageDown => PageDown,
        CapsLock => CapsLock,
        ShiftLeft => ShiftLeft, ShiftRight => ShiftRight,
        ControlLeft => ControlLeft, ControlRight => ControlRight,
        AltLeft => AltLeft, AltRight => AltRight,
        SuperLeft => SuperLeft, SuperRight => SuperRight,
        Minus => Minus, Equal => Equal,
        BracketLeft => BracketLeft, BracketRight => BracketRight,
        Backslash => Backslash, Semicolon => Semicolon, Quote => Quote,
        Backquote => Backquote, Comma => Comma, Period => Period, Slash => Slash,
        Numpad0 => Numpad0, Numpad1 => Numpad1, Numpad2 => Numpad2,
        Numpad3 => Numpad3, Numpad4 => Numpad4, Numpad5 => Numpad5,
        Numpad6 => Numpad6, Numpad7 => Numpad7, Numpad8 => Numpad8,
        Numpad9 => Numpad9,
        NumpadAdd => NumpadAdd, NumpadSubtract => NumpadSubtract,
        NumpadMultiply => NumpadMultiply, NumpadDivide => NumpadDivide,
        NumpadDecimal => NumpadDecimal, NumpadEnter => NumpadEnter,
    })
}

#[cfg(test)]
mod tests {
    use winit::dpi::PhysicalPosition;

    use super::*;

    #[test]
    fn letter_keys_drop_the_key_prefix() {
        assert_eq!(translate_key_code(KeyCode::KeyW), Some(Key::W));
    }

    #[test]
    fn other_keys_keep_their_name() {
        assert_eq!(translate_key_code(KeyCode::Space), Some(Key::Space));
        assert_eq!(
            translate_key_code(KeyCode::NumpadEnter),
            Some(Key::NumpadEnter)
        );
    }

    #[test]
    fn unknown_keys_are_ignored() {
        assert_eq!(translate_key_code(KeyCode::MediaPlayPause), None);
    }

    #[test]
    fn extra_mouse_buttons_are_ignored() {
        assert_eq!(
            translate_mouse_button(winit::event::MouseButton::Left),
            Some(MouseButton::Left)
        );
        assert_eq!(
            translate_mouse_button(winit::event::MouseButton::Other(9)),
            None
        );
    }

    #[test]
    fn wheel_lines_are_kept() {
        assert_eq!(
            translate_scroll(MouseScrollDelta::LineDelta(0.0, 1.0)),
            InputEvent::MouseWheel { x: 0.0, y: 1.0 }
        );
    }

    #[test]
    fn touchpad_pixels_become_lines() {
        let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(
            f64::from(PIXELS_PER_WHEEL_LINE),
            -2.0 * f64::from(PIXELS_PER_WHEEL_LINE),
        ));

        assert_eq!(
            translate_scroll(delta),
            InputEvent::MouseWheel { x: 1.0, y: -2.0 }
        );
    }
}
