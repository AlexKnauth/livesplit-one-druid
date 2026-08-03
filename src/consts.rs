use druid::{theme, Color, Env, FontDescriptor, FontFamily, FontWeight};

pub const TIMER_MIN_SIZE: f64 = 15.0;

pub const ICON_SIZE: f64 = 140.0;
pub const MARGIN: f64 = 20.0;
pub const SPACING: f64 = 16.0;
pub const BUTTON_SPACING: f64 = 8.0;
pub const BUTTON_HEIGHT: f64 = 30.0;
pub const GRID_BORDER: f64 = 4.0;
pub const DIALOG_BUTTON_WIDTH: f64 = 80.0;
pub const DIALOG_BUTTON_HEIGHT: f64 = 35.0;
pub const KEY_FONT: FontDescriptor = FontDescriptor::new(FontFamily::SYSTEM_UI)
    .with_weight(FontWeight::BOLD)
    .with_size(14.0);
pub const COLUMN_LABEL_FONT: FontDescriptor = KEY_FONT;
pub const TABLE_HORIZONTAL_MARGIN: f64 = 10.0;
pub const TIME_COLUMN_WIDTH: f64 = 110.0;
pub const ATTEMPTS_OFFSET_WIDTH: f64 = 140.0;
pub const RUN_EDITOR_BUTTONS: f64 = 8.0;
pub const RUN_EDITOR_WINDOW_HEIGHT: f64 = 24.0
    + ICON_SIZE
    + SPACING
    + 3.0 * MARGIN
    + RUN_EDITOR_BUTTONS * BUTTON_HEIGHT
    + (RUN_EDITOR_BUTTONS - 1.0) * BUTTON_SPACING
    + DIALOG_BUTTON_HEIGHT;

pub const SELECTED_TEXT_BACKGROUND_COLOR: Color = Color::rgb8(5, 99, 212);
pub const BUTTON_TOP: Color = Color::grey8(0x1c);
pub const BUTTON_BOTTOM: Color = Color::grey8(0x12);
pub const BACKGROUND: Color = Color::grey8(0x1c);
pub const BUTTON_BORDER: Color = Color::grey8(0x40);
pub const TEXTBOX_BACKGROUND: Color = Color::grey8(0x27);
pub const PRIMARY_LIGHT: Color = Color::grey8(0x80);
pub const BUTTON_BORDER_RADIUS: f64 = 5.0;
pub const BUTTON_ACTIVE_TOP: Color = Color::grey8(0x21);
pub const BUTTON_ACTIVE_BOTTOM: Color = Color::grey8(0x3b);

pub fn switch_style(env: &mut Env) {
    const ON: Color = Color::rgb8(0x1e, 0x44, 0x91);
    const OFF: Color = Color::grey8(0x22);
    const KNOB: Color = Color::WHITE;
    const TRANSPARENT: Color = Color::rgba8(0, 0, 0, 0);
    env.set(theme::PRIMARY_LIGHT, ON);
    env.set(theme::PRIMARY_DARK, ON);
    env.set(theme::BACKGROUND_LIGHT, OFF);
    env.set(theme::BACKGROUND_DARK, OFF);
    env.set(theme::FOREGROUND_LIGHT, KNOB);
    env.set(theme::FOREGROUND_DARK, KNOB);
    env.set(theme::BORDER_DARK, TRANSPARENT);
    env.set(theme::TEXT_COLOR, TRANSPARENT);
}
