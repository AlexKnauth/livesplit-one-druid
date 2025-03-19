use std::fmt;

use druid::{
    theme,
    widget::{Button, Flex},
    BoxConstraints, Code, Color, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, Size, UpdateCtx, Widget, WidgetExt,
};
use livesplit_core::hotkey::{KeyCode, Modifiers};

use crate::{consts::GRID_BORDER, HOTKEY_SYSTEM};

#[derive(Clone, Copy)]
pub struct Hotkey(pub Option<livesplit_core::hotkey::Hotkey>);

impl Hotkey {
    fn new(key: &druid::KeyEvent) -> Self {
        let key_code = match key.code {
            Code::Backquote => KeyCode::Backquote,
            Code::Backslash => KeyCode::Backslash,
            Code::BracketLeft => KeyCode::BracketLeft,
            Code::BracketRight => KeyCode::BracketRight,
            Code::Comma => KeyCode::Comma,
            Code::Digit0 => KeyCode::Digit0,
            Code::Digit1 => KeyCode::Digit1,
            Code::Digit2 => KeyCode::Digit2,
            Code::Digit3 => KeyCode::Digit3,
            Code::Digit4 => KeyCode::Digit4,
            Code::Digit5 => KeyCode::Digit5,
            Code::Digit6 => KeyCode::Digit6,
            Code::Digit7 => KeyCode::Digit7,
            Code::Digit8 => KeyCode::Digit8,
            Code::Digit9 => KeyCode::Digit9,
            Code::Equal => KeyCode::Equal,
            Code::IntlBackslash => KeyCode::IntlBackslash,
            Code::IntlRo => KeyCode::IntlRo,
            Code::IntlYen => KeyCode::IntlYen,
            Code::KeyA => KeyCode::KeyA,
            Code::KeyB => KeyCode::KeyB,
            Code::KeyC => KeyCode::KeyC,
            Code::KeyD => KeyCode::KeyD,
            Code::KeyE => KeyCode::KeyE,
            Code::KeyF => KeyCode::KeyF,
            Code::KeyG => KeyCode::KeyG,
            Code::KeyH => KeyCode::KeyH,
            Code::KeyI => KeyCode::KeyI,
            Code::KeyJ => KeyCode::KeyJ,
            Code::KeyK => KeyCode::KeyK,
            Code::KeyL => KeyCode::KeyL,
            Code::KeyM => KeyCode::KeyM,
            Code::KeyN => KeyCode::KeyN,
            Code::KeyO => KeyCode::KeyO,
            Code::KeyP => KeyCode::KeyP,
            Code::KeyQ => KeyCode::KeyQ,
            Code::KeyR => KeyCode::KeyR,
            Code::KeyS => KeyCode::KeyS,
            Code::KeyT => KeyCode::KeyT,
            Code::KeyU => KeyCode::KeyU,
            Code::KeyV => KeyCode::KeyV,
            Code::KeyW => KeyCode::KeyW,
            Code::KeyX => KeyCode::KeyX,
            Code::KeyY => KeyCode::KeyY,
            Code::KeyZ => KeyCode::KeyZ,
            Code::Minus => KeyCode::Minus,
            Code::Period => KeyCode::Period,
            Code::Quote => KeyCode::Quote,
            Code::Semicolon => KeyCode::Semicolon,
            Code::Slash => KeyCode::Slash,
            Code::AltLeft => KeyCode::AltLeft,
            Code::AltRight => KeyCode::AltRight,
            Code::Backspace => KeyCode::Backspace,
            Code::CapsLock => KeyCode::CapsLock,
            Code::ContextMenu => KeyCode::ContextMenu,
            Code::ControlLeft => KeyCode::ControlLeft,
            Code::ControlRight => KeyCode::ControlRight,
            Code::Enter => KeyCode::Enter,
            Code::MetaLeft => KeyCode::MetaLeft,
            Code::MetaRight => KeyCode::MetaRight,
            Code::ShiftLeft => KeyCode::ShiftLeft,
            Code::ShiftRight => KeyCode::ShiftRight,
            Code::Space => KeyCode::Space,
            Code::Tab => KeyCode::Tab,
            Code::Convert => KeyCode::Convert,
            Code::KanaMode => KeyCode::KanaMode,
            Code::Lang1 => KeyCode::Lang1,
            Code::Lang2 => KeyCode::Lang2,
            Code::Lang3 | Code::Hiragana => KeyCode::Lang3,
            Code::Lang4 | Code::Katakana => KeyCode::Lang4,
            Code::Lang5 => KeyCode::Lang5,
            Code::NonConvert => KeyCode::NonConvert,
            Code::Delete => KeyCode::Delete,
            Code::End => KeyCode::End,
            Code::Help => KeyCode::Help,
            Code::Home => KeyCode::Home,
            Code::Insert => KeyCode::Insert,
            Code::PageDown => KeyCode::PageDown,
            Code::PageUp => KeyCode::PageUp,
            Code::ArrowDown => KeyCode::ArrowDown,
            Code::ArrowLeft => KeyCode::ArrowLeft,
            Code::ArrowRight => KeyCode::ArrowRight,
            Code::ArrowUp => KeyCode::ArrowUp,
            Code::NumLock => KeyCode::NumLock,
            Code::Numpad0 => KeyCode::Numpad0,
            Code::Numpad1 => KeyCode::Numpad1,
            Code::Numpad2 => KeyCode::Numpad2,
            Code::Numpad3 => KeyCode::Numpad3,
            Code::Numpad4 => KeyCode::Numpad4,
            Code::Numpad5 => KeyCode::Numpad5,
            Code::Numpad6 => KeyCode::Numpad6,
            Code::Numpad7 => KeyCode::Numpad7,
            Code::Numpad8 => KeyCode::Numpad8,
            Code::Numpad9 => KeyCode::Numpad9,
            Code::NumpadAdd => KeyCode::NumpadAdd,
            Code::NumpadBackspace => KeyCode::NumpadBackspace,
            Code::NumpadClear => KeyCode::NumpadClear,
            Code::NumpadClearEntry => KeyCode::NumpadClearEntry,
            Code::NumpadComma => KeyCode::NumpadComma,
            Code::NumpadDecimal => KeyCode::NumpadDecimal,
            Code::NumpadDivide => KeyCode::NumpadDivide,
            Code::NumpadEnter => KeyCode::NumpadEnter,
            Code::NumpadEqual => KeyCode::NumpadEqual,
            Code::NumpadHash => KeyCode::NumpadHash,
            Code::NumpadMemoryAdd => KeyCode::NumpadMemoryAdd,
            Code::NumpadMemoryClear => KeyCode::NumpadMemoryClear,
            Code::NumpadMemoryRecall => KeyCode::NumpadMemoryRecall,
            Code::NumpadMemoryStore => KeyCode::NumpadMemoryStore,
            Code::NumpadMemorySubtract => KeyCode::NumpadMemorySubtract,
            Code::NumpadMultiply => KeyCode::NumpadMultiply,
            Code::NumpadParenLeft => KeyCode::NumpadParenLeft,
            Code::NumpadParenRight => KeyCode::NumpadParenRight,
            Code::NumpadStar => KeyCode::NumpadStar,
            Code::NumpadSubtract => KeyCode::NumpadSubtract,
            Code::Escape => KeyCode::Escape,
            Code::F1 => KeyCode::F1,
            Code::F2 => KeyCode::F2,
            Code::F3 => KeyCode::F3,
            Code::F4 => KeyCode::F4,
            Code::F5 => KeyCode::F5,
            Code::F6 => KeyCode::F6,
            Code::F7 => KeyCode::F7,
            Code::F8 => KeyCode::F8,
            Code::F9 => KeyCode::F9,
            Code::F10 => KeyCode::F10,
            Code::F11 => KeyCode::F11,
            Code::F12 => KeyCode::F12,
            Code::Fn => KeyCode::Fn,
            Code::FnLock => KeyCode::FnLock,
            Code::PrintScreen => KeyCode::PrintScreen,
            Code::ScrollLock => KeyCode::ScrollLock,
            Code::Pause => KeyCode::Pause,
            Code::BrowserBack => KeyCode::BrowserBack,
            Code::BrowserFavorites => KeyCode::BrowserFavorites,
            Code::BrowserForward => KeyCode::BrowserForward,
            Code::BrowserHome => KeyCode::BrowserHome,
            Code::BrowserRefresh => KeyCode::BrowserRefresh,
            Code::BrowserSearch => KeyCode::BrowserSearch,
            Code::BrowserStop => KeyCode::BrowserStop,
            Code::Eject => KeyCode::Eject,
            Code::LaunchApp1 => KeyCode::LaunchApp1,
            Code::LaunchApp2 => KeyCode::LaunchApp2,
            Code::LaunchMail => KeyCode::LaunchMail,
            Code::MediaPlayPause => KeyCode::MediaPlayPause,
            Code::MediaSelect => KeyCode::MediaSelect,
            Code::MediaStop => KeyCode::MediaStop,
            Code::MediaTrackNext => KeyCode::MediaTrackNext,
            Code::MediaTrackPrevious => KeyCode::MediaTrackPrevious,
            Code::Power => KeyCode::Power,
            Code::Sleep => KeyCode::Sleep,
            Code::AudioVolumeDown => KeyCode::AudioVolumeDown,
            Code::AudioVolumeMute => KeyCode::AudioVolumeMute,
            Code::AudioVolumeUp => KeyCode::AudioVolumeUp,
            Code::WakeUp => KeyCode::WakeUp,
            // Code::Hyper => KeyCode::Hyper,
            // Code::Super => KeyCode::Super,
            // Code::Turbo => KeyCode::Turbo,
            // Code::Abort => KeyCode::Abort,
            // Code::Resume => KeyCode::Resume,
            // Code::Suspend => KeyCode::Suspend,
            Code::Again => KeyCode::Again,
            Code::Copy => KeyCode::Copy,
            Code::Cut => KeyCode::Cut,
            Code::Find => KeyCode::Find,
            Code::Open => KeyCode::Open,
            Code::Paste => KeyCode::Paste,
            Code::Props => KeyCode::Props,
            Code::Select => KeyCode::Select,
            Code::Undo => KeyCode::Undo,
            // Code::Unidentified => KeyCode::Unidentified,
            Code::F13 => KeyCode::F13,
            Code::F14 => KeyCode::F14,
            Code::F15 => KeyCode::F15,
            Code::F16 => KeyCode::F16,
            Code::F17 => KeyCode::F17,
            Code::F18 => KeyCode::F18,
            Code::F19 => KeyCode::F19,
            Code::F20 => KeyCode::F20,
            Code::F21 => KeyCode::F21,
            Code::F22 => KeyCode::F22,
            Code::F23 => KeyCode::F23,
            Code::F24 => KeyCode::F24,
            Code::BrightnessDown => KeyCode::BrightnessDown,
            Code::BrightnessUp => KeyCode::BrightnessUp,
            Code::DisplayToggleIntExt => KeyCode::DisplayToggleIntExt,
            Code::KeyboardLayoutSelect => KeyCode::KeyboardLayoutSelect,
            Code::LaunchAssistant => KeyCode::LaunchAssistant,
            Code::LaunchControlPanel => KeyCode::LaunchControlPanel,
            Code::LaunchScreenSaver => KeyCode::LaunchScreenSaver,
            Code::MailForward => KeyCode::MailForward,
            Code::MailReply => KeyCode::MailReply,
            Code::MailSend => KeyCode::MailSend,
            Code::MediaFastForward => KeyCode::MediaFastForward,
            Code::MediaPause => KeyCode::MediaPause,
            Code::MediaPlay => KeyCode::MediaPlay,
            Code::MediaRecord => KeyCode::MediaRecord,
            Code::MediaRewind => KeyCode::MediaRewind,
            Code::MicrophoneMuteToggle => KeyCode::MicrophoneMuteToggle,
            Code::PrivacyScreenToggle => KeyCode::PrivacyScreenToggle,
            Code::SelectTask => KeyCode::SelectTask,
            Code::ShowAllWindows => KeyCode::ShowAllWindows,
            Code::ZoomToggle => KeyCode::ZoomToggle,
            _ => return Hotkey(None),
        };

        let mut mods = Modifiers::empty();
        if key.mods.shift() && !matches!(key.code, Code::ShiftLeft | Code::ShiftRight) {
            mods |= Modifiers::SHIFT;
        }
        if key.mods.ctrl() && !matches!(key.code, Code::ControlLeft | Code::ControlRight) {
            mods |= Modifiers::CONTROL;
        }
        if key.mods.alt() && !matches!(key.code, Code::AltLeft | Code::AltRight) {
            mods |= Modifiers::ALT;
        }
        if key.mods.meta() && !matches!(key.code, Code::MetaLeft | Code::MetaRight) {
            mods |= Modifiers::META;
        }

        Self(Some(key_code.with_modifiers(mods)))
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(key) = &self.0 {
            if key.modifiers != Modifiers::empty() {
                fmt::Display::fmt(&key.modifiers, f)?;
                f.write_str(" + ")?;
            }
            f.write_str(
                &HOTKEY_SYSTEM
                    .read()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .resolve(key.key_code),
            )
        } else {
            Ok(())
        }
    }
}

impl Data for Hotkey {
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

struct HotkeyButton(Button<Hotkey>);

impl Widget<Hotkey> for HotkeyButton {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Hotkey, env: &Env) {
        if let Event::KeyDown(key) = event {
            if key.code == Code::Tab {
                if key.mods.shift() {
                    ctx.focus_prev();
                } else {
                    ctx.focus_next();
                }
            } else if !key.repeat {
                *data = Hotkey::new(key);
            }
        } else if let Event::MouseUp(_) = event {
            ctx.request_focus();
        }
        self.0.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &Hotkey, env: &Env) {
        if let LifeCycle::BuildFocusChain = event {
            ctx.register_for_focus();
        } else if let LifeCycle::FocusChanged(_) = event {
            ctx.request_paint();
        }
        self.0.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &Hotkey, data: &Hotkey, env: &Env) {
        self.0.update(ctx, old_data, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Hotkey,
        env: &Env,
    ) -> Size {
        self.0.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Hotkey, env: &Env) {
        let mut env = env.clone();
        if ctx.is_focused() {
            env.set(theme::BORDER_LIGHT, Color::RED);
            env.set(theme::BORDER_DARK, Color::RED);
            env.set(theme::BUTTON_BORDER_WIDTH, 4.0);
        }
        env.set(theme::BUTTON_LIGHT, Color::grey8(0x10));
        env.set(theme::BUTTON_DARK, Color::grey8(0x10));
        env.set(theme::BUTTON_BORDER_RADIUS, 0.0);
        self.0.paint(ctx, data, &env);
    }
}

pub fn widget() -> impl Widget<Hotkey> {
    Flex::row()
        .with_flex_child(
            HotkeyButton(Button::new(|data: &Hotkey, _: &_| data.to_string())).expand_width(),
            1.0,
        )
        .with_spacer(GRID_BORDER)
        .with_child(Button::new("❌").on_click(|_, hotkey, _| {
            *hotkey = Hotkey(None);
        }))
}
