use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use druid::{
    commands,
    widget::{Button, Flex, ListIter, Scroll},
    Data, Widget, WidgetExt,
};

use livesplit_core::{
    auto_splitting::{
        settings::{
            FileFilter, Map as SettingsMap, Value as SettingValue, Widget as SettingsWidget,
            WidgetKind,
        },
        wasi_path, Runtime,
    },
    event::CommandSink,
    SharedTimer, StoredAutoSplitterSettings,
};

use crate::{
    combo_box,
    config::Config,
    consts::{BUTTON_SPACING, DIALOG_BUTTON_HEIGHT, DIALOG_BUTTON_WIDTH, MARGIN},
};

#[derive(Clone, Data)]
pub struct State {
    use_local_auto_splitter: bool,
    #[data(ignore)]
    pub runtime: Rc<Runtime<SharedTimer>>,
    #[data(ignore)]
    pub closed_with_ok: bool,
    /// Original use_local_auto_splitter value when the dialog was opened,
    /// used to revert on cancel
    #[data(ignore)]
    original_use_local_auto_splitter: bool,
    /// Original loaded_path when the dialog was opened, used to revert on cancel.
    #[data(ignore)]
    original_loaded_path: Option<PathBuf>,
    /// Original settings map when the dialog was opened, used to revert on cancel.
    #[data(ignore)]
    original_settings: Option<SettingsMap>,
}

impl State {
    pub(crate) fn new(runtime: Rc<Runtime<SharedTimer>>, use_local_auto_splitter: bool) -> Self {
        let settings_map = runtime.settings_map();
        Self {
            use_local_auto_splitter,
            original_use_local_auto_splitter: use_local_auto_splitter,
            original_loaded_path: runtime.loaded_path(),
            original_settings: settings_map,
            runtime,
            closed_with_ok: false,
        }
    }

    /// Revert the runtime settings to the original state when the dialog was opened.
    pub(crate) fn revert_settings(&self, config: &mut Config, timer: SharedTimer) {
        config.set_use_local_auto_splitter(self.original_use_local_auto_splitter);
        let settings = self.original_settings.clone().unwrap_or_default();
        if &self.runtime.loaded_path() == &self.original_loaded_path {
            self.runtime.set_settings_map(settings);
        } else {
            self.runtime.unload().ok();
            let mut s = StoredAutoSplitterSettings::new();
            if self.original_use_local_auto_splitter {
                s.set_script_path(
                    self.original_loaded_path
                        .as_ref()
                        .map(|p| p.to_string_lossy()),
                );
            }
            s.set_settings_map(settings);
            drop(timer.set_auto_splitter_settings(s));
            if self.original_use_local_auto_splitter {
                self.runtime.load(timer).ok();
            } else if let Some(p) = &self.original_loaded_path {
                self.runtime.load_from_path(timer, p.to_path_buf()).ok();
            } else {
                self.runtime.load(timer).ok();
            }
        }
    }
}

pub fn root_widget() -> impl Widget<State> {
    Flex::column()
        .with_flex_child(settings_editor(), 1.0)
        .with_child(dialog_buttons())
}

fn settings_editor() -> impl Widget<State> {
    Scroll::new(settings_widget().padding(MARGIN))
        .vertical()
        .expand_height()
}

fn settings_widget() -> impl Widget<State> {
    Flex::row().with_spacer(BUTTON_SPACING)
}

fn dialog_buttons() -> impl Widget<State> {
    Flex::row()
        .with_flex_spacer(1.0)
        .with_child(
            Button::new("OK")
                .on_click(|ctx, state: &mut State, _| {
                    // Changes are applied immediately in for_each_mut, so just close
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
