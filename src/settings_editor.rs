use std::{cell::RefCell, rc::Rc};

use druid::{
    commands,
    widget::{Button, Flex, ListIter, Scroll},
    Data, Widget, WidgetExt,
};
use livesplit_core::{
    settings::{SettingsDescription, Value},
    HotkeyConfig,
};

use crate::{
    consts::{BUTTON_SPACING, DIALOG_BUTTON_HEIGHT, DIALOG_BUTTON_WIDTH, MARGIN},
    settings_table::{self, SettingsRow},
};

#[derive(Clone, Data)]
pub struct State {
    state: Rc<SettingsDescription>,
    #[data(ignore)]
    pub editor: Rc<RefCell<Option<HotkeyConfig>>>,
    #[data(ignore)]
    pub closed_with_ok: bool,
}

impl State {
    pub(crate) fn new(editor: HotkeyConfig) -> Self {
        Self {
            state: Rc::new(editor.settings_description()),
            editor: Rc::new(RefCell::new(Some(editor))),
            closed_with_ok: false,
        }
    }
}

impl ListIter<SettingsRow> for State {
    fn for_each(&self, mut cb: impl FnMut(&SettingsRow, usize)) {
        let mut row = SettingsRow {
            index: 0,
            text: String::new(),
            value: Value::Bool(false),
        };

        for (index, field) in self.state.fields.iter().enumerate() {
            row.index = index;
            row.text.clone_from(&field.text);
            row.value.clone_from(&field.value);
            cb(&row, index);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut SettingsRow, usize)) {
        let mut row = SettingsRow {
            index: 0,
            text: String::new(),
            value: Value::Bool(false),
        };

        let mut editor = self.editor.borrow_mut();
        let editor = editor.as_mut().unwrap();
        let mut changed = false;

        for (index, field) in self.state.fields.iter().enumerate() {
            row.index = index;
            row.text.clone_from(&field.text);
            row.value.clone_from(&field.value);
            cb(&mut row, index);
            if row.value != field.value {
                let _ = editor.set_value(index, row.value.clone());
                changed = true;
            }
        }

        if changed {
            self.state = Rc::new(editor.settings_description());
        }
    }

    fn data_len(&self) -> usize {
        self.state.fields.len()
    }
}

pub fn root_widget() -> impl Widget<State> {
    Flex::column()
        .with_flex_child(settings_editor(), 1.0)
        .with_child(dialog_buttons())
}

fn settings_editor() -> impl Widget<State> {
    Scroll::new(settings_table::widget().padding(MARGIN))
        .vertical()
        .expand_height()
}

fn dialog_buttons() -> impl Widget<State> {
    Flex::row()
        .with_flex_spacer(1.0)
        .with_child(
            Button::new("OK")
                .on_click(|ctx, state: &mut State, _| {
                    state.closed_with_ok = true;
                    ctx.submit_command(commands::CLOSE_WINDOW);
                })
                .fix_size(DIALOG_BUTTON_WIDTH, DIALOG_BUTTON_HEIGHT),
        )
        .with_spacer(BUTTON_SPACING)
        .with_child(
            Button::new("Cancel")
                .on_click(|ctx, _, _| {
                    ctx.submit_command(commands::CLOSE_WINDOW);
                })
                .fix_size(DIALOG_BUTTON_WIDTH, DIALOG_BUTTON_HEIGHT),
        )
        .padding(MARGIN)
}
