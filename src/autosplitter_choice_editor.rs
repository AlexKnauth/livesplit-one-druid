use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use druid::{
    commands,
    lens::Identity,
    widget::{Button, Controller, Flex, Label, ListIter, Scroll, Switch},
    Data, Env, Event, EventCtx, FileDialogOptions, FileInfo, FileSpec, LensExt, LifeCycle,
    LifeCycleCtx, Selector, Widget, WidgetExt,
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

const CHOICE_EDITOR_OPEN_AUTO_SPLITTER: Selector<FileInfo> =
    Selector::new("autosplitter-choice-editor-open-auto-splitter");

#[derive(Clone, Data)]
pub struct State {
    use_local_auto_splitter: bool,
    #[data(ignore)]
    pub timer: SharedTimer,
    #[data(ignore)]
    pub runtime: Rc<Runtime<SharedTimer>>,
    #[data(ignore)]
    pub config: Rc<RefCell<Config>>,
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
    pub(crate) fn new(
        timer: SharedTimer,
        runtime: Rc<Runtime<SharedTimer>>,
        config: Rc<RefCell<Config>>,
    ) -> Self {
        let settings_map = runtime.settings_map();
        let use_local_auto_splitter = config.borrow().get_use_local_auto_splitter();
        Self {
            use_local_auto_splitter,
            original_use_local_auto_splitter: use_local_auto_splitter,
            original_loaded_path: runtime.loaded_path(),
            original_settings: settings_map,
            timer,
            runtime,
            config,
            closed_with_ok: false,
        }
    }

    /// Revert the runtime settings to the original state when the dialog was opened.
    pub(crate) fn revert_settings(&self) {
        self.config
            .borrow_mut()
            .set_use_local_auto_splitter(self.original_use_local_auto_splitter);
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
            drop(self.timer.set_auto_splitter_settings(s));
            if self.original_use_local_auto_splitter {
                self.runtime.load(self.timer.clone()).ok();
            } else if let Some(p) = &self.original_loaded_path {
                self.runtime
                    .load_from_path(self.timer.clone(), p.to_path_buf())
                    .ok();
            } else {
                self.runtime.load(self.timer.clone()).ok();
            }
        }
    }
}

pub fn root_widget() -> impl Widget<State> {
    Flex::column()
        .with_flex_child(settings_editor(), 1.0)
        .with_child(dialog_buttons())
        .controller(SyncController)
}

struct SyncController;

impl<W: Widget<State>> Controller<State, W> for SyncController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut State,
        env: &Env,
    ) {
        // Handle file dialog request command
        if let Event::Command(cmd) = event {
            if let Some(file_info) = cmd.get(CHOICE_EDITOR_OPEN_AUTO_SPLITTER) {
                data.runtime
                    .load_from_path(data.timer.clone(), file_info.path().to_path_buf());

                ctx.set_handled();
                return;
            }
        }

        child.event(ctx, event, data, env);
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &State,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            // Start the animation frame loop
            ctx.request_anim_frame();
        }
        child.lifecycle(ctx, event, data, env);
    }
}

fn settings_editor() -> impl Widget<State> {
    Scroll::new(settings_widget().padding(MARGIN))
        .vertical()
        .expand_height()
}

fn settings_widget() -> impl Widget<State> {
    Flex::column()
        .with_child(external_auto_splitter_widget())
        .with_child(use_local_auto_splitter_widget())
        .with_child(local_auto_splitter_path_widget())
}

fn external_auto_splitter_widget() -> impl Widget<State> {
    Flex::row().with_spacer(BUTTON_SPACING).with_child(
        Button::new("Activate")
            .on_click(|_ctx, s: &mut State, _env| {
                // TODO: store the autosplitter to be activated somewhere
                todo!("Activate")
            })
            .disabled_if(|s: &State, _| s.use_local_auto_splitter),
    )
}

fn use_local_auto_splitter_widget() -> impl Widget<State> {
    Flex::row()
        .with_child(Label::new("Use local auto splitter"))
        .with_spacer(BUTTON_SPACING)
        .with_child(
            Switch::new()
                .lens(Identity.map(
                    |s: &State| s.use_local_auto_splitter,
                    |s: &mut State, val: bool| {
                        s.use_local_auto_splitter = val;
                    },
                ))
                .center(),
        )
}

fn local_auto_splitter_path_widget() -> impl Widget<State> {
    Flex::row()
        .with_spacer(BUTTON_SPACING)
        .with_child(
            Button::new("Open Auto-splitter").on_click(|ctx, _s: &mut State, _env| {
                let open_dialog = commands::SHOW_OPEN_PANEL.with(
                    FileDialogOptions::new()
                        .title("Open Auto-splitter")
                        .allowed_types(vec![
                            FileSpec {
                                name: "WASM Auto-splitters",
                                extensions: &["wasm"],
                            },
                            FileSpec {
                                name: "All Files",
                                extensions: &["*.*"],
                            },
                        ])
                        .accept_command(CHOICE_EDITOR_OPEN_AUTO_SPLITTER),
                );
                ctx.submit_command(open_dialog);
            }),
        )
        .disabled_if(|s: &State, _| !s.use_local_auto_splitter)
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
