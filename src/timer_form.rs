use std::{path::Path, sync::Arc};

use druid::{
    commands,
    menu::MenuEntry,
    piet::PietImage,
    theme,
    widget::{Controller, Flex},
    AppDelegate, AppLauncher, BoxConstraints, DelegateCtx, Env, Event, EventCtx, FileDialogOptions,
    FileInfo, FileSpec, LayoutCtx, LifeCycle, LifeCycleCtx, Menu, MenuItem, Point, Selector, Size,
    UpdateCtx, Widget, WidgetExt, WindowDesc, WindowId, WindowLevel,
};
use livesplit_core::{LayoutEditor, RunEditor, TimerPhase, TimingMethod};
#[cfg(not(target_os = "macos"))]
use native_dialog::MessageType;

use crate::{
    config::or_show_error,
    consts::{
        BACKGROUND, BUTTON_BORDER, BUTTON_BORDER_RADIUS, BUTTON_BOTTOM, BUTTON_TOP, PRIMARY_LIGHT,
        SELECTED_TEXT_BACKGROUND_COLOR, TEXTBOX_BACKGROUND,
    },
    layout_editor, run_editor, settings_editor, software_renderer, LayoutEditorLens, MainState,
    OpenWindow, RunEditorLens, SettingsEditorLens, HOTKEY_SYSTEM,
};

struct WithMenu<T> {
    // device: Device,
    renderer: livesplit_core::rendering::software::Renderer,
    bottom_image: Option<PietImage>,
    inner: T,
    intent: Intent,
    intent_path: Option<Arc<Path>>,
}

impl<T> WithMenu<T> {
    fn new(inner: T) -> Self {
        // let mut device = Device::new().unwrap();
        let renderer = livesplit_core::rendering::software::Renderer::new();
        Self {
            // bottom_image: {
            //     let mut target = device.bitmap_target(1, 1, 1.0).unwrap();
            //     let mut ctx = target.render_context();
            //     let image = ctx
            //         .make_image(1, 1, &[0; 4], ImageFormat::RgbaPremul)
            //         .unwrap();
            //     ctx.finish().unwrap();
            //     image
            // },
            // device,
            renderer,
            bottom_image: None,
            inner,
            intent: Intent::NONE,
            intent_path: None,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
struct Intent(u16);

impl Intent {
    const NONE: Self = Self(0);
    const RESET: Self = Self(1 << 0);
    const MAYBE_SAVE_SPLITS: Self = Self(1 << 1);
    const SAVE_SPLITS: Self = Self(1 << 2);
    const SAVE_SPLITS_AS: Self = Self(1 << 3);
    const NEW_SPLITS: Self = Self(1 << 4);
    const OPEN_SPLITS: Self = Self(1 << 5);
    const MAYBE_SAVE_LAYOUT: Self = Self(1 << 6);
    const SAVE_LAYOUT: Self = Self(1 << 7);
    const SAVE_LAYOUT_AS: Self = Self(1 << 8);
    const NEW_LAYOUT: Self = Self(1 << 9);
    const OPEN_LAYOUT: Self = Self(1 << 10);
    const EXIT: Self = Self(1 << 11);
    const OPEN_AUTO_SPLITTER: Self = Self(1 << 12);

    fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

const CONTEXT_MENU_EDIT_SPLITS: Selector = Selector::new("context-menu-edit-splits");
const CONTEXT_MENU_SET_INTENT: Selector<Intent> = Selector::new("context-menu-set-intent");
const CONTEXT_MENU_SET_INTENT_WITH_PATH: Selector<(Intent, Arc<Path>)> =
    Selector::new("context-menu-set-intent-with-path");
const CONTEXT_MENU_OPEN_SPLITS: Selector<FileInfo> = Selector::new("context-menu-open-splits");
const CONTEXT_MENU_SAVE_SPLITS_AS: Selector<FileInfo> =
    Selector::new("context-menu-save-splits-as");
const CONTEXT_MENU_EDIT_LAYOUT: Selector = Selector::new("context-menu-edit-layout");
const CONTEXT_MENU_OPEN_LAYOUT: Selector<FileInfo> = Selector::new("context-menu-open-layout");
const CONTEXT_MENU_SAVE_LAYOUT_AS: Selector<FileInfo> =
    Selector::new("context-menu-save-layout-as");
const CONTEXT_MENU_OPEN_AUTO_SPLITTER: Selector<FileInfo> =
    Selector::new("context-menu-open-auto-splitter");
const CONTEXT_MENU_START_OR_SPLIT: Selector = Selector::new("context-menu-start-or-split");
const CONTEXT_MENU_UNDO_SPLIT: Selector = Selector::new("context-menu-undo-split");
const CONTEXT_MENU_SKIP_SPLIT: Selector = Selector::new("context-menu-skip-split");
const CONTEXT_MENU_TOGGLE_PAUSE: Selector = Selector::new("context-menu-toggle-pause");
const CONTEXT_MENU_UNDO_ALL_PAUSES: Selector = Selector::new("context-menu-undo-all-pauses");
const CONTEXT_MENU_SET_COMPARISON: Selector<String> = Selector::new("context-menu-set-comparison");
const CONTEXT_MENU_SET_TIMING_METHOD: Selector<TimingMethod> =
    Selector::new("context-menu-set-timing-method");
const CONTEXT_MENU_EDIT_SETTINGS: Selector = Selector::new("context-menu-edit-settings");

impl<T: Widget<MainState>> Widget<MainState> for WithMenu<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut MainState, env: &Env) {
        match event {
            Event::AnimFrame(_) => {
                ctx.request_anim_frame();
                ctx.request_paint();
            }
            Event::Wheel(event) => {
                if event.wheel_delta.y > 0.0 {
                    data.layout_data.borrow_mut().layout.scroll_down();
                } else {
                    data.layout_data.borrow_mut().layout.scroll_up();
                }
            }
            Event::MouseUp(event) => {
                if (event.button.is_right() || (event.button.is_left() && event.mods.ctrl()))
                    && data.run_editor.is_none()
                    && data.layout_editor.is_none()
                    && data.settings_editor.is_none()
                {
                    let mut compare_against = Menu::new("Compare Against");

                    let timer = data.timer.read().unwrap();

                    let current_split_index = timer.current_split_index();
                    let segment_count = timer.run().len();
                    let current_comparison = timer.current_comparison();
                    let current_timing_method = timer.current_timing_method();
                    let current_phase = timer.current_phase();

                    let mut open_recent = Menu::new("Open Recent");
                    for (game, categories) in data.config.borrow().splits_history() {
                        let mut game = Menu::new(game.clone());
                        for (category, paths) in categories {
                            let mut category = Menu::new(category.clone());
                            for path in paths {
                                let name = match path.file_name() {
                                    Some(name) => name.to_string_lossy().into_owned(),
                                    None => String::from("Untitled"),
                                };
                                category = category.entry(
                                    MenuItem::new(name).command(
                                        CONTEXT_MENU_SET_INTENT_WITH_PATH.with((
                                            Intent::RESET
                                                .with(Intent::MAYBE_SAVE_SPLITS)
                                                .with(Intent::OPEN_SPLITS),
                                            path.clone(),
                                        )),
                                    ),
                                );
                            }
                            game = game.entry(category);
                        }
                        open_recent = open_recent.entry(game);
                    }

                    for comparison in timer.run().comparisons() {
                        compare_against = compare_against.entry(
                            MenuItem::new(comparison)
                                .command(CONTEXT_MENU_SET_COMPARISON.with(comparison.to_owned()))
                                .selected(comparison == current_comparison),
                        );
                    }
                    compare_against = compare_against.separator();
                    for (name, timing_method) in [
                        ("Real Time", TimingMethod::RealTime),
                        ("Game Time", TimingMethod::GameTime),
                    ] {
                        compare_against = compare_against.entry(
                            MenuItem::new(name)
                                .command(CONTEXT_MENU_SET_TIMING_METHOD.with(timing_method))
                                .selected(timing_method == current_timing_method),
                        );
                    }

                    let control_menu: MenuEntry<_> = if current_phase == TimerPhase::NotRunning {
                        MenuItem::new("Start Timer")
                            .command(CONTEXT_MENU_START_OR_SPLIT)
                            .into()
                    } else {
                        Menu::new("Control")
                            .entry(
                                MenuItem::new("Split")
                                    .enabled(current_phase == TimerPhase::Running)
                                    .command(CONTEXT_MENU_START_OR_SPLIT),
                            )
                            .entry(
                                MenuItem::new("Reset")
                                    .command(CONTEXT_MENU_SET_INTENT.with(Intent::RESET)),
                            )
                            .separator()
                            .entry(
                                MenuItem::new("Undo Split")
                                    .enabled(current_split_index > Some(0))
                                    .command(CONTEXT_MENU_UNDO_SPLIT),
                            )
                            .entry(
                                MenuItem::new("Skip Split")
                                    .enabled(
                                        current_split_index
                                            .map_or(false, |x| x + 1 < segment_count),
                                    )
                                    .command(CONTEXT_MENU_SKIP_SPLIT),
                            )
                            .separator()
                            .entry(
                                MenuItem::new(if current_phase == TimerPhase::Paused {
                                    "Resume"
                                } else {
                                    "Pause"
                                })
                                .enabled(current_phase != TimerPhase::Ended)
                                .command(CONTEXT_MENU_TOGGLE_PAUSE),
                            )
                            .entry(
                                MenuItem::new("Undo All Pauses")
                                    .command(CONTEXT_MENU_UNDO_ALL_PAUSES),
                            )
                            .into()
                    };

                    ctx.show_context_menu::<MainState>(
                        Menu::new("LiveSplit")
                            .entry(
                                Menu::new("Splits")
                                    .entry(
                                        MenuItem::new("Edit...")
                                            .enabled(current_phase == TimerPhase::NotRunning)
                                            .command(CONTEXT_MENU_EDIT_SPLITS),
                                    )
                                    .separator()
                                    .entry(
                                        MenuItem::new("New").command(
                                            CONTEXT_MENU_SET_INTENT.with(
                                                Intent::RESET
                                                    .with(Intent::MAYBE_SAVE_SPLITS)
                                                    .with(Intent::NEW_SPLITS),
                                            ),
                                        ),
                                    )
                                    .entry(
                                        MenuItem::new("Open...").command(
                                            CONTEXT_MENU_SET_INTENT.with(
                                                Intent::RESET
                                                    .with(Intent::MAYBE_SAVE_SPLITS)
                                                    .with(Intent::OPEN_SPLITS),
                                            ),
                                        ),
                                    )
                                    .entry(open_recent)
                                    .separator()
                                    .entry(
                                        MenuItem::new("Save").command(
                                            CONTEXT_MENU_SET_INTENT
                                                .with(Intent::RESET.with(Intent::SAVE_SPLITS)),
                                        ),
                                    )
                                    .entry(
                                        MenuItem::new("Save As...").command(
                                            CONTEXT_MENU_SET_INTENT
                                                .with(Intent::RESET.with(Intent::SAVE_SPLITS_AS)),
                                        ),
                                    ),
                            )
                            .entry(
                                Menu::new("Layout")
                                    .entry(
                                        MenuItem::new("Edit...").command(CONTEXT_MENU_EDIT_LAYOUT),
                                    )
                                    .separator()
                                    .entry(MenuItem::new("New").command(
                                        CONTEXT_MENU_SET_INTENT.with(
                                            Intent::MAYBE_SAVE_LAYOUT.with(Intent::NEW_LAYOUT),
                                        ),
                                    ))
                                    .entry(MenuItem::new("Open...").command(
                                        CONTEXT_MENU_SET_INTENT.with(
                                            Intent::MAYBE_SAVE_LAYOUT.with(Intent::OPEN_LAYOUT),
                                        ),
                                    ))
                                    .entry(
                                        MenuItem::new("Save").command(
                                            CONTEXT_MENU_SET_INTENT.with(Intent::SAVE_LAYOUT),
                                        ),
                                    )
                                    .entry(MenuItem::new("Save As...").command(
                                        CONTEXT_MENU_SET_INTENT.with(Intent::SAVE_LAYOUT_AS),
                                    )),
                            )
                            .entry(
                                MenuItem::new("Open Auto-splitter...").command(
                                    CONTEXT_MENU_SET_INTENT.with(Intent::OPEN_AUTO_SPLITTER),
                                ),
                            )
                            .separator()
                            .entry(control_menu)
                            .entry(compare_against)
                            .separator()
                            .entry(MenuItem::new("Settings").command(CONTEXT_MENU_EDIT_SETTINGS))
                            .separator()
                            .entry(
                                MenuItem::new("Exit").command(
                                    CONTEXT_MENU_SET_INTENT.with(
                                        Intent::RESET
                                            .with(Intent::MAYBE_SAVE_SPLITS)
                                            .with(Intent::MAYBE_SAVE_LAYOUT)
                                            .with(Intent::EXIT),
                                    ),
                                ),
                            ),
                        event.pos,
                    );
                }
            }
            Event::Command(command) => {
                if command.is(CONTEXT_MENU_EDIT_SPLITS) {
                    let _ = HOTKEY_SYSTEM
                        .write()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .deactivate();
                    let run = data.timer.read().unwrap().run().clone();
                    let editor = RunEditor::new(run).unwrap();
                    let window = WindowDesc::new(run_editor::root_widget().lens(RunEditorLens))
                        .title("Splits Editor")
                        .with_min_size((690.0, 495.0))
                        .window_size((690.0, 495.0))
                        // TODO: WindowLevel::Modal(ctx.window().clone())
                        .set_level(WindowLevel::AppWindow)
                        .set_always_on_top(true);
                    let window_id = window.id;
                    ctx.new_window(window);
                    data.run_editor = Some(OpenWindow {
                        id: window_id,
                        state: run_editor::State::new(
                            editor,
                            data.config.clone(),
                            data.image_cache.clone(),
                        ),
                    });
                } else if let Some(file_info) = command.get(CONTEXT_MENU_OPEN_SPLITS) {
                    let result = data.config.borrow_mut().open_splits(
                        &data.timer,
                        &mut data.layout_data.borrow_mut(),
                        #[cfg(feature = "auto-splitting")]
                        &data.auto_splitter,
                        file_info.path().to_path_buf(),
                    );
                    or_show_error(result);
                } else if let Some(file_info) = command.get(CONTEXT_MENU_SAVE_SPLITS_AS) {
                    let result = data.config.borrow_mut().save_splits_as(
                        &mut data.timer.write().unwrap(),
                        #[cfg(feature = "auto-splitting")]
                        &data.auto_splitter,
                        file_info.path().to_path_buf(),
                    );
                    or_show_error(result);
                } else if command.is(CONTEXT_MENU_EDIT_LAYOUT) {
                    let _ = HOTKEY_SYSTEM
                        .write()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .deactivate();
                    let layout = data.layout_data.borrow().layout.clone();
                    let editor = LayoutEditor::new(layout).unwrap();
                    let window =
                        WindowDesc::new(layout_editor::root_widget().lens(LayoutEditorLens))
                            .title("Layout Editor")
                            .with_min_size((500.0, 600.0))
                            .window_size((550.0, 650.0))
                            // TODO: WindowLevel::Modal(ctx.window().clone())
                            .set_level(WindowLevel::AppWindow)
                            .set_always_on_top(true);
                    let window_id = window.id;
                    ctx.new_window(window);
                    data.layout_editor = Some(OpenWindow {
                        id: window_id,
                        state: layout_editor::State::new(editor, data.image_cache.clone()),
                    });
                } else if let Some(file_info) = command.get(CONTEXT_MENU_OPEN_LAYOUT) {
                    let result = data.config.borrow_mut().open_layout(
                        Some(&mut data.timer.write().unwrap()),
                        &mut data.layout_data.borrow_mut(),
                        file_info.path(),
                    );
                    or_show_error(result);
                } else if let Some(file_info) = command.get(CONTEXT_MENU_SAVE_LAYOUT_AS) {
                    let settings = data.layout_data.borrow_mut().layout.settings();
                    let result = data.config.borrow_mut().save_layout_as(
                        &mut data.timer.write().unwrap(),
                        settings,
                        file_info.path().to_path_buf(),
                    );
                    if result.is_ok() {
                        data.layout_data.borrow_mut().is_modified = false;
                    }
                    or_show_error(result);
                } else if let Some(file_info) = command.get(CONTEXT_MENU_OPEN_AUTO_SPLITTER) {
                    let result = data.config.borrow_mut().open_auto_splitter(
                        #[cfg(feature = "auto-splitting")]
                        &data.timer,
                        #[cfg(feature = "auto-splitting")]
                        &data.auto_splitter,
                        file_info.path(),
                    );
                    or_show_error(result);
                } else if command.is(CONTEXT_MENU_START_OR_SPLIT) {
                    data.timer.write().unwrap().split_or_start().ok();
                } else if command.is(CONTEXT_MENU_UNDO_SPLIT) {
                    data.timer.write().unwrap().undo_split().ok();
                } else if command.is(CONTEXT_MENU_SKIP_SPLIT) {
                    data.timer.write().unwrap().skip_split().ok();
                } else if command.is(CONTEXT_MENU_TOGGLE_PAUSE) {
                    data.timer.write().unwrap().toggle_pause().ok();
                } else if command.is(CONTEXT_MENU_UNDO_ALL_PAUSES) {
                    data.timer.write().unwrap().undo_all_pauses().ok();
                } else if let Some(comparison) = command.get(CONTEXT_MENU_SET_COMPARISON) {
                    // The comparison should always exist.
                    let _ = data
                        .timer
                        .write()
                        .unwrap()
                        .set_current_comparison(comparison.as_str());
                    data.config.borrow_mut().set_comparison(comparison.clone());
                } else if let Some(timing_method) = command.get(CONTEXT_MENU_SET_TIMING_METHOD) {
                    data.timer
                        .write()
                        .unwrap()
                        .set_current_timing_method(*timing_method);
                    data.config.borrow_mut().set_timing_method(*timing_method);
                } else if command.is(CONTEXT_MENU_EDIT_SETTINGS) {
                    let _ = HOTKEY_SYSTEM
                        .write()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .deactivate();
                    let window =
                        WindowDesc::new(settings_editor::root_widget().lens(SettingsEditorLens))
                            .title("Settings")
                            .with_min_size((550.0, 400.0))
                            .window_size((550.0, 450.0))
                            // TODO: WindowLevel::Modal(ctx.window().clone())
                            .set_level(WindowLevel::AppWindow)
                            .set_always_on_top(true);
                    let window_id = window.id;
                    ctx.new_window(window);
                    let config = HOTKEY_SYSTEM.read().unwrap().as_ref().unwrap().config();
                    data.settings_editor = Some(OpenWindow {
                        id: window_id,
                        state: settings_editor::State::new(config),
                    });
                } else if let Some(intent) = command.get(CONTEXT_MENU_SET_INTENT) {
                    self.intent = *intent;
                } else if let Some((intent, path)) = command.get(CONTEXT_MENU_SET_INTENT_WITH_PATH)
                {
                    self.intent = *intent;
                    self.intent_path = Some(path.clone());
                }

                while self.intent != Intent::NONE {
                    if self.intent.contains(Intent::RESET) {
                        self.intent = self.intent.without(Intent::RESET);
                        let wants_to_save_times = if data
                            .timer
                            .read()
                            .unwrap()
                            .current_attempt_has_new_best_times()
                        {
                            // TODO: fix this MessageDialog so that it doesn't cause
                            // crashes on save while timer is running or ended (on Mac)
                            #[cfg(not(target_os = "macos"))]
                            {
                                let result = native_dialog::MessageDialog::new()
                                    .set_title("Update Times?")
                                    .set_text("You have beaten some of your best times. Do you want to update them?")
                                    .set_type(MessageType::Warning)
                                    .show_confirm();

                                if let Ok(wants_to_save_times) = result {
                                    wants_to_save_times
                                } else {
                                    self.intent = Intent::NONE;
                                    break;
                                }
                            }
                            // since the MessageDialog isn't working on Mac, assume Yes for now
                            #[cfg(target_os = "macos")]
                            true
                        } else {
                            true
                        };
                        data.timer.write().unwrap().reset(wants_to_save_times).ok();
                    }

                    if self.intent.contains(Intent::MAYBE_SAVE_SPLITS) {
                        self.intent = self.intent.without(Intent::MAYBE_SAVE_SPLITS);
                        if data.timer.read().unwrap().run().has_been_modified() {
                            // TODO: fix this MessageDialog so that it doesn't cause crashes on Mac
                            #[cfg(not(target_os = "macos"))]
                            {
                                let result = native_dialog::MessageDialog::new()
                                    .set_title("Save Splits?")
                                    .set_text("Your splits have been updated but not yet saved. Do you want to save your splits now?")
                                    .set_type(MessageType::Warning)
                                    .show_confirm();

                                if let Ok(wants_to_save) = result {
                                    if wants_to_save {
                                        self.intent = self.intent.with(Intent::SAVE_SPLITS);
                                    }
                                } else {
                                    self.intent = Intent::NONE;
                                }
                            }
                            // since the MessageDialog isn't working on Mac, assume Yes for now
                            #[cfg(target_os = "macos")]
                            {
                                self.intent = self.intent.with(Intent::SAVE_SPLITS);
                            }
                        }
                    }

                    if self.intent.contains(Intent::SAVE_SPLITS) {
                        self.intent = self.intent.without(Intent::SAVE_SPLITS);
                        if data.config.borrow().can_directly_save_splits() {
                            let result = data.config.borrow_mut().save_splits(
                                &mut data.timer.write().unwrap(),
                                #[cfg(feature = "auto-splitting")]
                                &data.auto_splitter,
                            );
                            or_show_error(result);
                        } else {
                            self.intent = self.intent.with(Intent::SAVE_SPLITS_AS);
                        }
                    }

                    if self.intent.contains(Intent::SAVE_SPLITS_AS) {
                        self.intent = self.intent.without(Intent::SAVE_SPLITS_AS);
                        ctx.submit_command(build_save_splits_as());
                        break;
                    }

                    if self.intent.contains(Intent::NEW_SPLITS) {
                        self.intent = self.intent.without(Intent::NEW_SPLITS);
                        data.config
                            .borrow_mut()
                            .new_splits(&mut data.timer.write().unwrap());
                    }

                    if self.intent.contains(Intent::OPEN_SPLITS) {
                        self.intent = self.intent.without(Intent::OPEN_SPLITS);
                        let command = if let Some(path) = self.intent_path.take() {
                            CONTEXT_MENU_OPEN_SPLITS.with(FileInfo {
                                path: path.to_path_buf(),
                                format: None,
                            })
                        } else {
                            commands::SHOW_OPEN_PANEL.with(
                                FileDialogOptions::new()
                                    .title("Open Splits")
                                    .allowed_types(vec![
                                        FileSpec {
                                            name: "LiveSplit Splits",
                                            extensions: &["lss"],
                                        },
                                        FileSpec {
                                            name: "All Files",
                                            extensions: &["*.*"],
                                        },
                                    ])
                                    .accept_command(CONTEXT_MENU_OPEN_SPLITS),
                            )
                        };
                        ctx.submit_command(command);
                        break;
                    }

                    if self.intent.contains(Intent::MAYBE_SAVE_LAYOUT) {
                        self.intent = self.intent.without(Intent::MAYBE_SAVE_LAYOUT);
                        if data.layout_data.borrow().is_modified {
                            // TODO: fix this MessageDialog so that it doesn't cause crashes on Mac
                            #[cfg(not(target_os = "macos"))]
                            {
                                let result = native_dialog::MessageDialog::new()
                                    .set_title("Save Layout?")
                                    .set_text("Your layout has been updated but not yet saved. Do you want to save your layout now?")
                                    .set_type(MessageType::Warning)
                                    .show_confirm();

                                if let Ok(wants_to_save) = result {
                                    if wants_to_save {
                                        self.intent = self.intent.with(Intent::SAVE_LAYOUT);
                                    }
                                } else {
                                    self.intent = Intent::NONE;
                                }
                            }
                            // since the MessageDialog isn't working on Mac, assume Yes for now
                            #[cfg(target_os = "macos")]
                            {
                                self.intent = self.intent.with(Intent::SAVE_LAYOUT);
                            }
                        }
                    }

                    if self.intent.contains(Intent::SAVE_LAYOUT) {
                        self.intent = self.intent.without(Intent::SAVE_LAYOUT);
                        if data.config.borrow().can_directly_save_layout() {
                            let settings = data.layout_data.borrow_mut().layout.settings();
                            let result = data.config.borrow_mut().save_layout(settings);
                            if result.is_ok() {
                                data.layout_data.borrow_mut().is_modified = false;
                            }
                            or_show_error(result);
                        } else {
                            self.intent = self.intent.with(Intent::SAVE_LAYOUT_AS);
                        }
                    }

                    if self.intent.contains(Intent::SAVE_LAYOUT_AS) {
                        self.intent = self.intent.without(Intent::SAVE_LAYOUT_AS);
                        ctx.submit_command(build_save_layout_as());
                        break;
                    }

                    if self.intent.contains(Intent::NEW_LAYOUT) {
                        self.intent = self.intent.without(Intent::NEW_LAYOUT);
                        data.config.borrow_mut().new_layout(
                            Some(&mut data.timer.write().unwrap()),
                            &mut data.layout_data.borrow_mut(),
                        );
                    }

                    if self.intent.contains(Intent::OPEN_LAYOUT) {
                        self.intent = self.intent.without(Intent::OPEN_LAYOUT);
                        let open_dialog = commands::SHOW_OPEN_PANEL.with(
                            FileDialogOptions::new()
                                .title("Open Layout")
                                .allowed_types(vec![
                                    FileSpec {
                                        name: "LiveSplit Layouts",
                                        extensions: &["lsl", "ls1l"],
                                    },
                                    FileSpec {
                                        name: "All Files",
                                        extensions: &["*.*"],
                                    },
                                ])
                                .accept_command(CONTEXT_MENU_OPEN_LAYOUT),
                        );
                        ctx.submit_command(open_dialog);
                        break;
                    }

                    if self.intent.contains(Intent::OPEN_AUTO_SPLITTER) {
                        self.intent = self.intent.without(Intent::OPEN_AUTO_SPLITTER);
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
                                .accept_command(CONTEXT_MENU_OPEN_AUTO_SPLITTER),
                        );
                        ctx.submit_command(open_dialog);
                        break;
                    }

                    if self.intent.contains(Intent::EXIT) {
                        self.intent = self.intent.without(Intent::EXIT);
                        ctx.submit_command(commands::QUIT_APP);
                        break;
                    }
                }
            }
            _ => {}
        }
        self.set_mouse_pass_through_while_running(ctx, data);
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        _data: &MainState,
        _env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            ctx.request_anim_frame();
            ctx.request_paint();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &MainState, data: &MainState, env: &Env) {
        self.inner.update(ctx, old_data, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &MainState,
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &MainState, _env: &Env) {
        let mut layout_data = data.layout_data.borrow_mut();
        let layout_data = &mut *layout_data;

        if let Some(editor) = &data.layout_editor {
            editor
                .state
                .editor
                .borrow_mut()
                .as_mut()
                .unwrap()
                .update_layout_state(
                    &mut layout_data.layout_state,
                    &mut data.image_cache.borrow_mut(),
                    &data.timer.read().unwrap().snapshot(),
                );
        } else {
            layout_data.layout.update_state(
                &mut layout_data.layout_state,
                &mut data.image_cache.borrow_mut(),
                &data.timer.read().unwrap().snapshot(),
            );
        }

        // let size = ctx.size();

        // if let Some((new_width, new_height)) = layout_data.scene_manager.update_scene(
        //     PietResourceAllocator,
        //     (size.width as f32, size.height as f32),
        //     &layout_data.layout_state,
        // ) {
        //     ctx.window()
        //         .set_size(Size::new(new_width as _, new_height as _));
        // }

        // software_renderer::render_scene(
        //     ctx,
        //     &mut self.bottom_image,
        //     &mut self.device,
        //     layout_data.scene_manager.scene(),
        //     &data.image_cache.borrow(),
        // );

        if let Some((new_width, new_height)) = software_renderer::render_scene(
            ctx,
            &mut self.bottom_image,
            &mut self.renderer,
            &layout_data.layout_state,
            &data.image_cache.borrow(),
        ) {
            ctx.window().set_size(Size::new(new_width, new_height));
        }
        data.image_cache.borrow_mut().collect();
    }
}

impl<T: Widget<MainState>> WithMenu<T> {
    fn set_mouse_pass_through_while_running(&mut self, ctx: &mut EventCtx, data: &mut MainState) {
        let pass_while_running = data
            .layout_data
            .borrow()
            .layout_state
            .mouse_pass_through_while_running;
        let timer = data.timer.read().unwrap();
        let current_phase = timer.current_phase();
        let mouse_pass_through = pass_while_running
            && current_phase == TimerPhase::Running
            && !ctx.window().is_foreground_window();
        if data.mouse_pass_through != mouse_pass_through {
            data.mouse_pass_through = mouse_pass_through;
            ctx.window().set_mouse_pass_through(mouse_pass_through);
        }
    }
}

fn build_save_splits_as() -> druid::Command {
    commands::SHOW_SAVE_PANEL.with(
        FileDialogOptions::new()
            .title("Save Splits")
            .allowed_types(vec![
                FileSpec {
                    name: "LiveSplit Splits",
                    extensions: &["lss"],
                },
                FileSpec {
                    name: "All Files",
                    extensions: &["*.*"],
                },
            ])
            .accept_command(CONTEXT_MENU_SAVE_SPLITS_AS),
    )
}

fn build_save_layout_as() -> druid::Command {
    commands::SHOW_SAVE_PANEL.with(
        FileDialogOptions::new()
            .title("Save Layout")
            .allowed_types(vec![
                FileSpec {
                    name: "LiveSplit One Layouts",
                    extensions: &["ls1l"],
                },
                FileSpec {
                    name: "All Files",
                    extensions: &["*.*"],
                },
            ])
            .accept_command(CONTEXT_MENU_SAVE_LAYOUT_AS),
    )
}

struct DragWindowController {
    init_pos: Option<Point>,
}

impl DragWindowController {
    pub fn new() -> Self {
        DragWindowController { init_pos: None }
    }
}

impl<T, W: Widget<T>> Controller<T, W> for DragWindowController {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseDown(me) if me.buttons.has_left() => {
                ctx.set_active(true);
                self.init_pos = Some(me.window_pos)
            }
            Event::MouseMove(me) if ctx.is_active() && me.buttons.has_left() => {
                if let Some(init_pos) = self.init_pos {
                    let within_window_change = me.window_pos.to_vec2() - init_pos.to_vec2();
                    let old_pos = ctx.window().get_position();
                    let new_pos = old_pos + within_window_change;
                    ctx.window().set_position(new_pos)
                }
            }
            Event::MouseUp(_me) if ctx.is_active() => {
                self.init_pos = None;
                ctx.set_active(false)
            }
            _ => (),
        }
        child.event(ctx, event, data, env)
    }
}

pub fn root_widget() -> impl Widget<MainState> {
    WithMenu::new(Flex::row()).controller(DragWindowController::new())
}

struct WindowManagement;

impl AppDelegate<MainState> for WindowManagement {
    fn window_removed(
        &mut self,
        id: WindowId,
        data: &mut MainState,
        _env: &Env,
        _ctx: &mut DelegateCtx,
    ) {
        if let Some(window) = &data.run_editor {
            if id == window.id {
                if window.state.closed_with_ok {
                    let run = window.state.editor.borrow_mut().take().unwrap().close();
                    data.timer
                        .write()
                        .unwrap()
                        .set_run(run)
                        .map_err(drop)
                        .unwrap();
                }
                data.run_editor = None;
                let _ = HOTKEY_SYSTEM.write().unwrap().as_mut().unwrap().activate();
                return;
            }
        }

        if let Some(window) = &data.layout_editor {
            if id == window.id {
                if window.state.closed_with_ok {
                    let layout = window.state.editor.borrow_mut().take().unwrap().close();
                    let mut layout_data = data.layout_data.borrow_mut();
                    layout_data.layout = layout;
                    layout_data.is_modified = true;
                }
                data.layout_editor = None;
                let _ = HOTKEY_SYSTEM.write().unwrap().as_mut().unwrap().activate();
                return;
            }
        }

        if let Some(window) = &data.settings_editor {
            if id == window.id {
                if window.state.closed_with_ok {
                    let hotkey_config = window.state.editor.borrow_mut().take().unwrap();
                    let _ = HOTKEY_SYSTEM
                        .write()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .set_config(hotkey_config);
                    data.config.borrow_mut().set_hotkeys(hotkey_config);
                }
                data.settings_editor = None;
                let _ = HOTKEY_SYSTEM.write().unwrap().as_mut().unwrap().activate();
                return;
            }
        }
    }
}

pub fn launch(state: MainState, window: WindowDesc<MainState>) {
    AppLauncher::with_window(window)
        .configure_env(|env, _| {
            env.set(
                theme::SELECTED_TEXT_BACKGROUND_COLOR,
                SELECTED_TEXT_BACKGROUND_COLOR,
            );
            env.set(
                theme::SELECTED_TEXT_INACTIVE_BACKGROUND_COLOR,
                druid::Color::TRANSPARENT,
            );
            env.set(theme::BUTTON_LIGHT, BUTTON_TOP);
            env.set(theme::BUTTON_DARK, BUTTON_BOTTOM);
            env.set(theme::WINDOW_BACKGROUND_COLOR, BACKGROUND);
            env.set(theme::BORDER_DARK, BUTTON_BORDER);
            env.set(theme::BACKGROUND_LIGHT, TEXTBOX_BACKGROUND);
            env.set(theme::PRIMARY_LIGHT, PRIMARY_LIGHT);
            env.set(theme::BUTTON_BORDER_RADIUS, BUTTON_BORDER_RADIUS);
        })
        .delegate(WindowManagement)
        .launch(state)
        .unwrap();
}
