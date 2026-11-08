//! Keyboard input translation: egui key events -> shortcut actions.
//!
//! This module is pure with respect to app state. It reads the frame input,
//! applies the tested `shortcut_context_ok` guard, and returns the actions plus
//! the events that must be consumed so a focused `TextEdit` does not also see
//! them (SRP, spec-017).

use crate::shortcuts::{self, Action, Combo};

/// Translate an `egui::Key` into the small names used by `shortcuts::lookup`.
fn key_name(key: egui::Key) -> &'static str {
    use egui::Key::*;
    match key {
        A => "a",
        B => "b",
        C => "c",
        D => "d",
        E => "e",
        F => "f",
        G => "g",
        H => "h",
        I => "i",
        J => "j",
        K => "k",
        L => "l",
        M => "m",
        N => "n",
        O => "o",
        P => "p",
        Q => "q",
        R => "r",
        S => "s",
        T => "t",
        U => "u",
        V => "v",
        W => "w",
        X => "x",
        Y => "y",
        Z => "z",
        Num0 => "0",
        Num1 => "1",
        Num2 => "2",
        Num3 => "3",
        Num4 => "4",
        Num5 => "5",
        Num6 => "6",
        Num7 => "7",
        Num8 => "8",
        Num9 => "9",
        Plus => "+",
        Minus => "-",
        Equals => "=",
        Backslash => "\\",
        Slash => "/",
        Comma => ",",
        Period => ".",
        Colon => ":",
        Semicolon => ";",
        Quote => "'",
        Backtick => "`",
        Pipe => "|",
        Questionmark => "?",
        Exclamationmark => "!",
        OpenBracket => "[",
        CloseBracket => "]",
        OpenCurlyBracket => "{",
        CloseCurlyBracket => "}",
        Tab => "tab",
        Escape => "escape",
        Enter => "enter",
        Space => "space",
        ArrowUp => "arrowup",
        ArrowDown => "arrowdown",
        ArrowLeft => "arrowleft",
        ArrowRight => "arrowright",
        F1 => "f1",
        F11 => "f11",
        _ => "",
    }
}

/// Read the pressed keys, resolve exact shortcuts, and return the events to
/// consume so they do not also reach a focused `TextEdit`.
pub(super) fn collect_shortcuts(
    ctx: &egui::Context,
    popup_open: bool,
) -> (Vec<Action>, Vec<(egui::Key, egui::Modifiers)>) {
    // Read the frame's trusted input context together with the events. A key
    // is only allowed to become a shortcut when the pointer is over the
    // window, the window owns the keyboard focus, and the event's modifiers
    // agree with the held modifiers (spec-013). This stops key events that
    // arrive after the pointer leaves the window — or with a stale Ctrl/Alt
    // left over from a focus change — from hijacking ordinary typing.
    let (events, context) = ctx.input(|i| {
        let events: Vec<(egui::Key, egui::Modifiers)> = i
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } => Some((*key, *modifiers)),
                _ => None,
            })
            .collect();
        (
            events,
            (
                i.focused,
                i.pointer.has_pointer(),
                (i.modifiers.ctrl, i.modifiers.shift, i.modifiers.alt),
            ),
        )
    });
    let (window_focused, pointer_over_window, frame_mods) = context;
    let mut actions = Vec::new();
    let mut consumed = Vec::new();
    for (key, modifiers) in events {
        let event_mods = (modifiers.ctrl, modifiers.shift, modifiers.alt);
        if !shortcuts::shortcut_context_ok(
            window_focused,
            pointer_over_window,
            event_mods,
            frame_mods,
        ) {
            continue;
        }
        let name = key_name(key);
        // Enter / arrow keys are navigation only while a popover is open;
        // otherwise they must remain available to text editors.
        let popup_only = matches!(name, "enter" | "arrowup" | "arrowdown")
            && !modifiers.ctrl
            && !modifiers.shift
            && !modifiers.alt;
        if popup_only && !popup_open {
            continue;
        }
        let combo = Combo::new(modifiers.ctrl, modifiers.shift, modifiers.alt, name);
        if let Some(action) = shortcuts::lookup(combo) {
            actions.push(action);
            consumed.push((key, modifiers));
        }
    }
    (actions, consumed)
}
