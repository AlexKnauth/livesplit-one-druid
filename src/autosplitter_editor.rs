use std::{
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use druid::{
    commands::{self, CLOSE_WINDOW},
    lens::Identity,
    widget::{Button, Controller, Flex, Label, List, ListIter, Scroll, Switch},
    Data, Env, Event, EventCtx, LensExt, LifeCycle, LifeCycleCtx, Point, RenderContext, TimerToken,
    Widget, WidgetExt, WindowConfig, WindowId, WindowLevel, WindowSizePolicy,
};
use livesplit_core::{
    auto_splitting::{
        settings::{
            FileFilter, Map as SettingsMap, Value as SettingValue, Widget as SettingsWidget,
            WidgetKind,
        },
        Runtime,
    },
    SharedTimer,
};

use crate::{
    combo_box,
    consts::{BUTTON_SPACING, DIALOG_BUTTON_HEIGHT, DIALOG_BUTTON_WIDTH, MARGIN},
};

#[derive(Clone, Data)]
pub struct State {
    rows: Arc<Vec<SettingRow>>,
    #[data(ignore)]
    pub runtime: Rc<Runtime<SharedTimer>>,
    #[data(ignore)]
    pub closed_with_ok: bool,
}

#[derive(Clone, Data, PartialEq)]
struct SettingRow {
    index: usize,
    key: Arc<str>,
    description: Arc<str>,
    tooltip: Option<Arc<str>>,
    value: SettingRowValue,
    indent_level: u32,
}

#[derive(Clone, Data, PartialEq)]
enum SettingRowValue {
    Title {
        heading_level: u32,
    },
    Bool(bool),
    Choice {
        current: usize,
        options: Arc<Vec<ChoiceOption>>,
    },
    FileSelect {
        /// Current file path (empty string if none selected)
        path: Arc<str>,
        /// File filters for the dialog
        #[data(ignore)]
        filters: Arc<Vec<FileFilter>>,
    },
}

#[derive(Clone, PartialEq)]
struct ChoiceOption {
    key: Arc<str>,
    description: Arc<str>,
}

impl Data for ChoiceOption {
    fn same(&self, other: &Self) -> bool {
        self.key == other.key && self.description == other.description
    }
}

impl State {
    pub(crate) fn new(runtime: Rc<Runtime<SharedTimer>>) -> Self {
        let widgets = runtime.settings_widgets().unwrap_or_default();
        let settings_map = runtime.settings_map();
        let rows = build_rows(&widgets, settings_map.as_ref());
        Self {
            rows: Arc::new(rows),
            runtime,
            closed_with_ok: false,
        }
    }

    /// Sync UI state with the runtime's current settings.
    /// Returns true if the state was updated.
    fn sync_from_runtime(&mut self) -> bool {
        let widgets = self.runtime.settings_widgets().unwrap_or_default();
        let settings_map = self.runtime.settings_map();
        let new_rows = build_rows(&widgets, settings_map.as_ref());

        if *self.rows != new_rows {
            self.rows = Arc::new(new_rows);
            true
        } else {
            false
        }
    }
}

fn build_rows(widgets: &[SettingsWidget], settings_map: Option<&SettingsMap>) -> Vec<SettingRow> {
    let mut rows = Vec::new();
    // Track the current heading level for indentation of subsequent settings
    let mut current_heading_level: Option<u32> = None;

    for (index, widget) in widgets.iter().enumerate() {
        let (value, indent_level) = match &widget.kind {
            WidgetKind::Title { heading_level } => {
                current_heading_level = Some(*heading_level);
                (
                    SettingRowValue::Title {
                        heading_level: *heading_level,
                    },
                    *heading_level,
                )
            }
            WidgetKind::Bool { default_value } => {
                let current = settings_map
                    .and_then(|m| m.get(widget.key.as_ref()))
                    .and_then(|v| match v {
                        SettingValue::Bool(b) => Some(*b),
                        _ => None,
                    })
                    .unwrap_or(*default_value);
                // Indent one level deeper than the current title
                let indent = current_heading_level.map_or(0, |h| h + 1);
                (SettingRowValue::Bool(current), indent)
            }
            WidgetKind::Choice {
                default_option_key,
                options,
            } => {
                let choice_options: Vec<ChoiceOption> = options
                    .iter()
                    .map(|opt| ChoiceOption {
                        key: opt.key.clone(),
                        description: opt.description.clone(),
                    })
                    .collect();

                let current_key = settings_map
                    .and_then(|m| m.get(widget.key.as_ref()))
                    .and_then(|v| match v {
                        SettingValue::String(s) => Some(s.as_ref()),
                        _ => None,
                    })
                    .unwrap_or(default_option_key.as_ref());

                let current = choice_options
                    .iter()
                    .position(|opt| opt.key.as_ref() == current_key)
                    .unwrap_or(0);

                // Indent one level deeper than the current title
                let indent = current_heading_level.map_or(0, |h| h + 1);
                (
                    SettingRowValue::Choice {
                        current,
                        options: Arc::new(choice_options),
                    },
                    indent,
                )
            }
            WidgetKind::FileSelect { filters } => {
                let path = settings_map
                    .and_then(|m| m.get(widget.key.as_ref()))
                    .and_then(|v| match v {
                        SettingValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "".into());
                let indent = current_heading_level.map_or(0, |h| h + 1);
                (
                    SettingRowValue::FileSelect {
                        path,
                        filters: filters.clone(),
                    },
                    indent,
                )
            }
        };

        rows.push(SettingRow {
            index,
            key: widget.key.clone(),
            description: widget.description.clone(),
            tooltip: widget.tooltip.clone(),
            value,
            indent_level,
        });
    }

    rows
}

impl ListIter<SettingRow> for State {
    fn for_each(&self, mut cb: impl FnMut(&SettingRow, usize)) {
        for (idx, row) in self.rows.iter().enumerate() {
            cb(row, idx);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut SettingRow, usize)) {
        let mut rows = (*self.rows).clone();
        let mut changed_rows: Vec<(Arc<str>, SettingRowValue)> = Vec::new();

        for (idx, row) in rows.iter_mut().enumerate() {
            let old_value = row.value.clone();
            cb(row, idx);
            if row.value != old_value {
                changed_rows.push((row.key.clone(), row.value.clone()));
            }
        }

        if !changed_rows.is_empty() {
            // Apply changes to the runtime immediately so sync doesn't overwrite them
            let mut new_map = self.runtime.settings_map().unwrap_or_default();
            for (key, value) in changed_rows {
                let setting_value = match &value {
                    SettingRowValue::Title { .. } => continue,
                    SettingRowValue::Bool(b) => SettingValue::Bool(*b),
                    SettingRowValue::Choice { current, options } => {
                        if let Some(opt) = options.get(*current) {
                            SettingValue::String(opt.key.to_string().into())
                        } else {
                            continue;
                        }
                    }
                    SettingRowValue::FileSelect { path, .. } => {
                        SettingValue::String(path.to_string().into())
                    }
                };
                new_map.insert(key.to_string().into(), setting_value);
            }
            self.runtime.set_settings_map(new_map);
            self.rows = Arc::new(rows);
        }
    }

    fn data_len(&self) -> usize {
        self.rows.len()
    }
}

pub fn root_widget() -> impl Widget<State> {
    Flex::column()
        .with_flex_child(settings_editor(), 1.0)
        .with_child(dialog_buttons())
        .controller(SyncController)
}

/// Controller that periodically syncs the UI with the runtime's settings.
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
        if let Event::AnimFrame(_) = event {
            // Sync with runtime settings
            if data.sync_from_runtime() {
                ctx.request_update();
            }
            // Request another frame to keep polling
            ctx.request_anim_frame();
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
    Scroll::new(List::new(setting_row_widget).padding(MARGIN))
        .vertical()
        .expand_height()
}

/// Controller that shows a tooltip when hovering over a setting row.
enum TooltipState {
    Fresh,
    Waiting {
        last_move: Instant,
        timer_expire: Instant,
        token: TimerToken,
        position: Point,
    },
    Showing(WindowId),
}

struct RowTooltipController {
    state: TooltipState,
}

impl RowTooltipController {
    fn new() -> Self {
        Self {
            state: TooltipState::Fresh,
        }
    }
}

impl<W: Widget<SettingRow>> Controller<SettingRow, W> for RowTooltipController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut SettingRow,
        env: &Env,
    ) {
        let tooltip = match &data.tooltip {
            Some(t) if !t.is_empty() => t.clone(),
            _ => {
                // No tooltip, just pass through
                child.event(ctx, event, data, env);
                return;
            }
        };

        let wait_duration = Duration::from_millis(500);
        let resched_dur = Duration::from_millis(50);
        let cursor_size = druid::Size::new(15., 15.);
        let now = Instant::now();

        let new_state = match &self.state {
            TooltipState::Fresh => match event {
                Event::MouseMove(me) if ctx.is_hot() => Some(TooltipState::Waiting {
                    last_move: now,
                    timer_expire: now + wait_duration,
                    token: ctx.request_timer(wait_duration),
                    position: me.window_pos,
                }),
                _ => None,
            },
            TooltipState::Waiting {
                last_move,
                timer_expire,
                token,
                position,
            } => match event {
                Event::MouseMove(me) if ctx.is_hot() => {
                    let (cur_token, cur_expire) = if *timer_expire - now < resched_dur {
                        (ctx.request_timer(wait_duration), now + wait_duration)
                    } else {
                        (*token, *timer_expire)
                    };
                    Some(TooltipState::Waiting {
                        last_move: now,
                        timer_expire: cur_expire,
                        token: cur_token,
                        position: me.window_pos,
                    })
                }
                Event::Timer(tok) if tok == token => {
                    let deadline = *last_move + wait_duration;
                    ctx.set_handled();
                    if deadline > now {
                        let wait_for = deadline - now;
                        Some(TooltipState::Waiting {
                            last_move: *last_move,
                            timer_expire: deadline,
                            token: ctx.request_timer(wait_for),
                            position: *position,
                        })
                    } else {
                        let tooltip_position =
                            (position.to_vec2() + cursor_size.to_vec2()).to_point();
                        let win_id = ctx.new_sub_window(
                            WindowConfig::default()
                                .show_titlebar(false)
                                .window_size_policy(WindowSizePolicy::Content)
                                .set_level(WindowLevel::Tooltip(ctx.window().clone()))
                                .set_position(tooltip_position),
                            Label::<()>::new(tooltip.to_string())
                                .with_line_break_mode(druid::widget::LineBreaking::WordWrap)
                                .fix_width(300.0)
                                .padding(4.0)
                                .background(druid::Color::grey8(0x40)),
                            (),
                            env.clone(),
                        );
                        Some(TooltipState::Showing(win_id))
                    }
                }
                _ => None,
            },
            TooltipState::Showing(win_id) => match event {
                Event::MouseMove(_) if !ctx.is_hot() => {
                    ctx.submit_command(CLOSE_WINDOW.to(*win_id));
                    Some(TooltipState::Fresh)
                }
                _ => None,
            },
        };

        if let Some(state) = new_state {
            self.state = state;
        }

        if !ctx.is_handled() {
            child.event(ctx, event, data, env);
        }
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &SettingRow,
        env: &Env,
    ) {
        if let LifeCycle::HotChanged(false) = event {
            if let TooltipState::Showing(win_id) = self.state {
                ctx.submit_command(CLOSE_WINDOW.to(win_id));
            }
            self.state = TooltipState::Fresh;
        }
        child.lifecycle(ctx, event, data, env)
    }
}

const INDENT_WIDTH: f64 = 20.0;

fn setting_row_widget() -> impl Widget<SettingRow> {
    druid::widget::ViewSwitcher::new(
        |row: &SettingRow, _| matches!(row.value, SettingRowValue::Title { .. }),
        |is_title, row: &SettingRow, _| {
            let indent = row.indent_level as f64 * INDENT_WIDTH;
            if *is_title {
                // Title row: bold label spanning the full width
                Box::new(
                    Flex::row()
                        .with_spacer(indent)
                        .with_flex_child(
                            Label::new(|row: &SettingRow, _: &Env| row.description.to_string())
                                .with_font(druid::theme::UI_FONT_BOLD)
                                .with_line_break_mode(druid::widget::LineBreaking::WordWrap)
                                .controller(RowTooltipController::new()),
                            1.0,
                        )
                        .padding(8.0)
                        .background(druid::widget::Painter::new(title_background)),
                )
            } else {
                // Setting row: label + value widget
                Box::new(
                    Flex::row()
                        .with_spacer(indent)
                        .with_child(
                            Label::new(|row: &SettingRow, _: &Env| row.description.to_string())
                                .with_line_break_mode(druid::widget::LineBreaking::WordWrap)
                                .fix_width(300.0 - indent)
                                .controller(RowTooltipController::new()),
                        )
                        .with_flex_spacer(1.0)
                        .with_child(setting_value_widget())
                        .padding(8.0)
                        .background(druid::widget::Painter::new(setting_background)),
                )
            }
        },
    )
}

fn title_background(ctx: &mut druid::PaintCtx, _row: &SettingRow, _env: &Env) {
    let rect = ctx.size().to_rect();
    ctx.fill(rect, &druid::Color::grey8(0x20));
}

fn setting_background(ctx: &mut druid::PaintCtx, row: &SettingRow, _env: &Env) {
    let rect = ctx.size().to_rect();
    let color = if row.index % 2 == 0 {
        druid::Color::grey8(0x14)
    } else {
        druid::Color::grey8(0x0b)
    };
    ctx.fill(rect, &color);
}

fn setting_value_widget() -> impl Widget<SettingRow> {
    druid::widget::ViewSwitcher::new(
        |row: &SettingRow, _| std::mem::discriminant(&row.value),
        |_, row: &SettingRow, _| match &row.value {
            SettingRowValue::Title { .. } => {
                // Title rows don't have a value widget, but this is needed for exhaustiveness
                Box::new(druid::widget::SizedBox::empty())
            }
            SettingRowValue::Bool(_) => Box::new(Switch::new().lens(Identity.map(
                |row: &SettingRow| matches!(row.value, SettingRowValue::Bool(true)),
                |row: &mut SettingRow, val: bool| {
                    row.value = SettingRowValue::Bool(val);
                },
            ))),
            SettingRowValue::Choice { options, .. } => {
                let options_clone: Arc<Vec<ChoiceOption>> = options.clone();
                Box::new(
                    combo_box::dynamic_list(ChoiceList(options_clone.clone()))
                        .lens(Identity.map(
                            move |row: &SettingRow| {
                                if let SettingRowValue::Choice { current, .. } = &row.value {
                                    *current
                                } else {
                                    0
                                }
                            },
                            move |row: &mut SettingRow, val: usize| {
                                if let SettingRowValue::Choice { current, options } = &mut row.value
                                {
                                    if val < options.len() {
                                        *current = val;
                                    }
                                }
                            },
                        ))
                        .fix_width(200.0),
                )
            }
            SettingRowValue::FileSelect { filters, .. } => {
                let filters_clone = filters.clone();
                Box::new(
                    Button::new(|row: &SettingRow, _: &Env| {
                        if let SettingRowValue::FileSelect { path, .. } = &row.value {
                            if path.is_empty() {
                                "Browse...".to_string()
                            } else {
                                // Show just the filename, not the full path
                                std::path::Path::new(path.as_ref())
                                    .file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| path.to_string())
                            }
                        } else {
                            "Browse...".to_string()
                        }
                    })
                    .on_click(move |_ctx, row: &mut SettingRow, _env| {
                        // Get current path for starting directory
                        let current_path =
                            if let SettingRowValue::FileSelect { path, .. } = &row.value {
                                if path.is_empty() {
                                    None
                                } else {
                                    std::path::Path::new(path.as_ref())
                                        .parent()
                                        .map(|p| p.to_path_buf())
                                }
                            } else {
                                None
                            };

                        // Show file dialog with filters
                        let result = show_file_dialog(&filters_clone, current_path.as_deref());

                        if let Some(path) = result {
                            if let SettingRowValue::FileSelect {
                                path: ref mut p, ..
                            } = &mut row.value
                            {
                                *p = path.to_string_lossy().into();
                            }
                        }
                    })
                    .fix_width(200.0),
                )
            }
        },
    )
}

/// Show a file dialog with the given filters. Returns the selected path, or None if cancelled.
fn show_file_dialog(
    filters: &[FileFilter],
    start_dir: Option<&std::path::Path>,
) -> Option<std::path::PathBuf> {
    // Collect all extensions from all filters into a single list
    let mut all_extensions: Vec<String> = Vec::new();

    for filter in filters {
        match filter {
            FileFilter::Name { pattern, .. } => {
                // Extract extensions from the pattern (e.g., "*.txt *.log" -> ["txt", "log"])
                for ext in pattern
                    .split_whitespace()
                    .filter_map(|p| p.strip_prefix("*."))
                {
                    if !all_extensions.contains(&ext.to_string()) {
                        all_extensions.push(ext.to_string());
                    }
                }
            }
            FileFilter::MimeType(mime) => {
                // Handle common MIME types by mapping to extensions
                let extensions: &[&str] = match mime.as_ref() {
                    "image/*" => &["png", "jpg", "jpeg", "gif", "bmp", "webp"],
                    "image/png" => &["png"],
                    "image/jpeg" => &["jpg", "jpeg"],
                    "text/*" => &["txt", "log", "md"],
                    "text/plain" => &["txt"],
                    "application/json" => &["json"],
                    "application/xml" => &["xml"],
                    _ => &[],
                };
                for ext in extensions {
                    if !all_extensions.contains(&ext.to_string()) {
                        all_extensions.push(ext.to_string());
                    }
                }
            }
        }
    }

    let ext_refs: Vec<&str> = all_extensions.iter().map(|s| s.as_str()).collect();

    let mut dialog = native_dialog::FileDialog::new();

    // Add a single filter with all extensions combined
    if !ext_refs.is_empty() {
        dialog = dialog.add_filter("Supported Files", &ext_refs);
    }

    // Set starting directory if provided
    if let Some(dir) = start_dir {
        dialog = dialog.set_location(dir);
    }

    dialog.show_open_single_file().ok().flatten()
}

#[derive(Clone)]
struct ChoiceList(Arc<Vec<ChoiceOption>>);

impl combo_box::ComboList for ChoiceList {
    type Label = ChoiceLabel;
    fn slice(&self) -> &[Self::Label] {
        // Safety: ChoiceLabel is a repr(transparent) wrapper around ChoiceOption
        unsafe { std::mem::transmute(self.0.as_slice()) }
    }
}

#[repr(transparent)]
struct ChoiceLabel(ChoiceOption);

impl combo_box::ComboLabel for ChoiceLabel {
    fn to_arc_str(&self) -> Arc<str> {
        self.0.description.clone()
    }
    fn to_label_text<T>(&self) -> druid::widget::LabelText<T> {
        druid::widget::LabelText::from(self.0.description.to_string())
    }
    fn as_str(&self) -> &str {
        &self.0.description
    }
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
