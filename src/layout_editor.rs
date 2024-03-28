use std::{cell::RefCell, rc::Rc};

use druid::{
    commands,
    theme,
    widget::{
        Button, CrossAxisAlignment, Flex, Label, List, ListIter, Scroll,
    },
    BoxConstraints, Color, Data, Env, Event, EventCtx, LayoutCtx, Lens, LifeCycle,
    LifeCycleCtx, LinearGradient, LocalizedString, Menu, MenuItem, PaintCtx, RenderContext,
    Selector, Size, UnitPoint, UpdateCtx, Widget, WidgetExt,
};
use livesplit_core::{
    component,
    layout::editor,
    settings::{ImageCache, Value},
    LayoutEditor,
};

use crate::{
    consts::{
        BUTTON_ACTIVE_BOTTOM, BUTTON_ACTIVE_TOP, BUTTON_BORDER, BUTTON_HEIGHT, BUTTON_SPACING,
        DIALOG_BUTTON_HEIGHT, DIALOG_BUTTON_WIDTH, MARGIN, SPACING,
    },
    settings_table::{self, SettingsRow},
    MainState,
};

#[derive(Clone, Data)]
pub struct State {
    state: Rc<editor::State>,
    #[data(ignore)]
    pub editor: Rc<RefCell<Option<LayoutEditor>>>,
    #[data(ignore)]
    pub closed_with_ok: bool,
    on_component_settings_tab: bool,
    image_cache: Rc<RefCell<ImageCache>>,
}

impl State {
    pub fn new(editor: LayoutEditor) -> Self {
        let mut image_cache = ImageCache::new();
        Self {
            state: Rc::new(editor.state(&mut image_cache)),
            editor: Rc::new(RefCell::new(Some(editor))),
            closed_with_ok: false,
            on_component_settings_tab: false,
            image_cache: Rc::new(RefCell::new(image_cache)),
        }
    }

    fn mutate(&mut self, f: impl FnOnce(&mut LayoutEditor)) {
        let mut editor = self.editor.borrow_mut();
        let editor = editor.as_mut().unwrap();
        f(editor);
        self.state = Rc::new(editor.state(&mut self.image_cache.borrow_mut()));
        self.image_cache.borrow_mut().collect();
    }
}

impl ListIter<SettingsRow> for State {
    fn for_each(&self, mut cb: impl FnMut(&SettingsRow, usize)) {
        let settings = if self.on_component_settings_tab {
            &self.state.component_settings
        } else {
            &self.state.general_settings
        };

        let mut row = SettingsRow {
            index: 0,
            text: String::new(),
            value: Value::Bool(false),
        };

        for (index, field) in settings.fields.iter().enumerate() {
            row.index = index;
            row.text.clone_from(&field.text);
            row.value.clone_from(&field.value);
            cb(&row, index);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut SettingsRow, usize)) {
        let settings = if self.on_component_settings_tab {
            &self.state.component_settings
        } else {
            &self.state.general_settings
        };

        let mut row = SettingsRow {
            index: 0,
            text: String::new(),
            value: Value::Bool(false),
        };

        let mut editor = self.editor.borrow_mut();
        let editor = editor.as_mut().unwrap();
        let mut changed = false;

        for (index, field) in settings.fields.iter().enumerate() {
            row.index = index;
            row.text.clone_from(&field.text);
            row.value.clone_from(&field.value);
            cb(&mut row, index);
            if row.value != field.value {
                if self.on_component_settings_tab {
                    editor.set_component_settings_value(index, row.value.clone());
                } else {
                    editor.set_general_settings_value(index, row.value.clone(), &self.image_cache.borrow());
                }
                changed = true;
            }
        }

        if changed {
            self.state = Rc::new(editor.state(&mut self.image_cache.borrow_mut()));
        }
        self.image_cache.borrow_mut().collect();
    }

    fn data_len(&self) -> usize {
        if self.on_component_settings_tab {
            self.state.component_settings.fields.len()
        } else {
            self.state.general_settings.fields.len()
        }
    }
}

impl ListIter<ComponentRow> for State {
    fn for_each(&self, mut cb: impl FnMut(&ComponentRow, usize)) {
        let mut row = ComponentRow {
            name: String::new(),
            index: 0,
            is_selected: false,
            select: false,
        };
        for (index, component) in self.state.components.iter().enumerate() {
            row.name.clone_from(component);
            row.index = index;
            row.is_selected = self.state.selected_component as usize == index;
            cb(&row, index);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut ComponentRow, usize)) {
        let mut row = ComponentRow {
            name: String::new(),
            index: 0,
            is_selected: false,
            select: false,
        };
        let mut editor = self.editor.borrow_mut();
        let editor = editor.as_mut().unwrap();
        let mut changed = false;

        for (index, component) in self.state.components.iter().enumerate() {
            row.name.clone_from(component);
            row.index = index;
            row.is_selected = self.state.selected_component as usize == index;
            cb(&mut row, index);
            if row.select {
                editor.select(index);
                row.select = false;
                changed = true;
            }
        }

        if changed {
            self.state = Rc::new(editor.state(&mut self.image_cache.borrow_mut()));
        }
        self.image_cache.borrow_mut().collect();
    }

    fn data_len(&self) -> usize {
        self.state.components.len()
    }
}

struct AddComponentWidget<T> {
    inner: T,
}

impl<T> AddComponentWidget<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: Widget<State>> Widget<State> for AddComponentWidget<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut State, env: &Env) {
        if let Event::MouseDown(event) = event {
            ctx.show_context_menu::<MainState>(
                Menu::new(LocalizedString::new("Components"))
                    .entry(
                        MenuItem::new(LocalizedString::new("Current Comparison"))
                            .command(ADD_COMPONENT_CURRENT_COMPARISON),
                    )
                    .entry(
                        MenuItem::new(LocalizedString::new("Current Pace"))
                            .command(ADD_COMPONENT_CURRENT_PACE),
                    )
                    .entry(
                        MenuItem::new(LocalizedString::new("Delta")).command(ADD_COMPONENT_DELTA),
                    )
                    .entry(
                        MenuItem::new(LocalizedString::new("Detailed Timer"))
                            .command(ADD_COMPONENT_DETAILED_TIMER),
                    )
                    .entry(
                        MenuItem::new(LocalizedString::new("Graph")).command(ADD_COMPONENT_GRAPH),
                    )
                    .entry(
                        MenuItem::new(LocalizedString::new("PB Chance"))
                            .command(ADD_COMPONENT_PB_CHANCE),
                    )
                    .entry(
                        MenuItem::new(LocalizedString::new("Possible Time Save"))
                            .command(ADD_COMPONENT_POSSIBLE_TIME_SAVE),
                    )
                    .entry(
                        MenuItem::new(LocalizedString::new("Previous Segment"))
                            .command(ADD_COMPONENT_PREVIOUS_SEGMENT),
                    )
                    .entry(
                        MenuItem::new(LocalizedString::new("Segment Time"))
                            .command(ADD_COMPONENT_SEGMENT_TIME),
                    )
                    .entry(
                        MenuItem::new(LocalizedString::new("Splits")).command(ADD_COMPONENT_SPLITS),
                    )
                    .entry(
                        MenuItem::new(LocalizedString::new("Sum of Best Segments"))
                            .command(ADD_COMPONENT_SUM_OF_BEST_SEGMENTS),
                    )
                    .entry(MenuItem::new(LocalizedString::new("Text")).command(ADD_COMPONENT_TEXT))
                    .entry(
                        MenuItem::new(LocalizedString::new("Timer")).command(ADD_COMPONENT_TIMER),
                    )
                    .entry(
                        MenuItem::new(LocalizedString::new("Title")).command(ADD_COMPONENT_TITLE),
                    )
                    .entry(
                        MenuItem::new(LocalizedString::new("Total Playtime"))
                            .command(ADD_COMPONENT_TOTAL_PLAYTIME),
                    )
                    .separator()
                    .entry(
                        MenuItem::new(LocalizedString::new("Blank Space"))
                            .command(ADD_COMPONENT_BLANK_SPACE),
                    )
                    .entry(
                        MenuItem::new(LocalizedString::new("Separator"))
                            .command(ADD_COMPONENT_SEPARATOR),
                    ),
                event.window_pos,
            );
            return;
        } else if let Event::Command(command) = event {
            if command.is(ADD_COMPONENT_CURRENT_COMPARISON) {
                data.mutate(|editor| editor.add_component(component::CurrentComparison::new()));
            } else if command.is(ADD_COMPONENT_CURRENT_PACE) {
                data.mutate(|editor| editor.add_component(component::CurrentPace::new()));
            } else if command.is(ADD_COMPONENT_DELTA) {
                data.mutate(|editor| editor.add_component(component::Delta::new()));
            } else if command.is(ADD_COMPONENT_DETAILED_TIMER) {
                data.mutate(|editor| {
                    editor.add_component(Box::new(component::DetailedTimer::new()))
                });
            } else if command.is(ADD_COMPONENT_GRAPH) {
                data.mutate(|editor| editor.add_component(component::Graph::new()));
            } else if command.is(ADD_COMPONENT_PB_CHANCE) {
                data.mutate(|editor| editor.add_component(component::PbChance::new()));
            } else if command.is(ADD_COMPONENT_POSSIBLE_TIME_SAVE) {
                data.mutate(|editor| editor.add_component(component::PossibleTimeSave::new()));
            } else if command.is(ADD_COMPONENT_PREVIOUS_SEGMENT) {
                data.mutate(|editor| editor.add_component(component::PreviousSegment::new()));
            } else if command.is(ADD_COMPONENT_SEGMENT_TIME) {
                data.mutate(|editor| editor.add_component(component::SegmentTime::new()));
            } else if command.is(ADD_COMPONENT_SPLITS) {
                data.mutate(|editor| editor.add_component(component::Splits::new()));
            } else if command.is(ADD_COMPONENT_SUM_OF_BEST_SEGMENTS) {
                data.mutate(|editor| editor.add_component(component::SumOfBest::new()));
            } else if command.is(ADD_COMPONENT_TEXT) {
                data.mutate(|editor| editor.add_component(component::Text::new()));
            } else if command.is(ADD_COMPONENT_TIMER) {
                data.mutate(|editor| editor.add_component(component::Timer::new()));
            } else if command.is(ADD_COMPONENT_TITLE) {
                data.mutate(|editor| editor.add_component(component::Title::new()));
            } else if command.is(ADD_COMPONENT_TOTAL_PLAYTIME) {
                data.mutate(|editor| editor.add_component(component::TotalPlaytime::new()));
            } else if command.is(ADD_COMPONENT_BLANK_SPACE) {
                data.mutate(|editor| editor.add_component(component::BlankSpace::new()));
            } else if command.is(ADD_COMPONENT_SEPARATOR) {
                data.mutate(|editor| editor.add_component(component::Separator::new()));
            }
        }
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &State, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &State, data: &State, env: &Env) {
        self.inner.update(ctx, old_data, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &State,
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &State, env: &Env) {
        self.inner.paint(ctx, data, env)
    }
}

struct ComponentRowWidget<T> {
    inner: T,
}

impl<T> ComponentRowWidget<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
}

#[derive(Clone, Data, Lens)]
struct ComponentRow {
    name: String,
    index: usize,
    is_selected: bool,
    select: bool,
}

impl<T: Widget<ComponentRow>> Widget<ComponentRow> for ComponentRowWidget<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut ComponentRow, env: &Env) {
        if let Event::MouseDown(_) = event {
            if !data.is_selected {
                data.select = true;
            }
        }
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &ComponentRow,
        env: &Env,
    ) {
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &ComponentRow,
        data: &ComponentRow,
        env: &Env,
    ) {
        // TODO: We honestly really only need to care about its selected state
        if !old_data.same(data) {
            ctx.request_paint();
        }
        self.inner.update(ctx, old_data, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &ComponentRow,
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &ComponentRow, env: &Env) {
        let rect = ctx.size().to_rect();
        if data.is_selected {
            ctx.fill(
                rect,
                &LinearGradient::new(
                    UnitPoint::TOP,
                    UnitPoint::BOTTOM,
                    (Color::rgb8(0x33, 0x73, 0xf4), Color::rgb8(0x15, 0x35, 0x74)),
                ),
            );
        } else {
            let color = if data.index & 1 == 0 {
                Color::grey8(0x12)
            } else {
                Color::grey8(0xb)
            };
            ctx.fill(rect, &color);
        }
        self.inner.paint(ctx, data, env)
    }
}

const ADD_COMPONENT_CURRENT_COMPARISON: Selector =
    Selector::new("layout-editor-add-current-comparison");
const ADD_COMPONENT_CURRENT_PACE: Selector = Selector::new("layout-editor-add-current-pace");
const ADD_COMPONENT_DELTA: Selector = Selector::new("layout-editor-add-delta");
const ADD_COMPONENT_DETAILED_TIMER: Selector = Selector::new("layout-editor-add-detailed-timer");
const ADD_COMPONENT_GRAPH: Selector = Selector::new("layout-editor-add-graph");
const ADD_COMPONENT_PB_CHANCE: Selector = Selector::new("layout-editor-add-pb-chance");
const ADD_COMPONENT_POSSIBLE_TIME_SAVE: Selector =
    Selector::new("layout-editor-add-possible-time-save");
const ADD_COMPONENT_PREVIOUS_SEGMENT: Selector =
    Selector::new("layout-editor-add-previous-segment");
const ADD_COMPONENT_SEGMENT_TIME: Selector = Selector::new("layout-editor-add-segment-time");
const ADD_COMPONENT_SPLITS: Selector = Selector::new("layout-editor-add-splits");
const ADD_COMPONENT_SUM_OF_BEST_SEGMENTS: Selector =
    Selector::new("layout-editor-add-sum-of-best-segments");
const ADD_COMPONENT_TEXT: Selector = Selector::new("layout-editor-add-text");
const ADD_COMPONENT_TIMER: Selector = Selector::new("layout-editor-add-timer");
const ADD_COMPONENT_TITLE: Selector = Selector::new("layout-editor-add-title");
const ADD_COMPONENT_TOTAL_PLAYTIME: Selector = Selector::new("layout-editor-add-total-playtime");
const ADD_COMPONENT_BLANK_SPACE: Selector = Selector::new("layout-editor-add-blank-space");
const ADD_COMPONENT_SEPARATOR: Selector = Selector::new("layout-editor-add-separator");

fn side_buttons() -> impl Widget<State> {
    Flex::column()
        .with_child(AddComponentWidget::new(
            Button::new("Add").expand_width().fix_height(BUTTON_HEIGHT),
        ))
        .with_spacer(BUTTON_SPACING)
        .with_child(
            Button::new("Remove")
                .on_click(|_, state: &mut State, _| {
                    state.mutate(|editor| editor.remove_component());
                })
                .expand_width()
                .fix_height(BUTTON_HEIGHT),
        )
        .with_spacer(BUTTON_SPACING)
        .with_child(
            Button::new("Duplicate")
                .on_click(|_, state: &mut State, _| {
                    state.mutate(|editor| editor.duplicate_component());
                })
                .expand_width()
                .fix_height(BUTTON_HEIGHT),
        )
        .with_spacer(BUTTON_SPACING)
        .with_child(
            Button::new("Move Up")
                .on_click(|_, state: &mut State, _| {
                    state.mutate(|editor| editor.move_component_up());
                })
                .expand_width()
                .fix_height(BUTTON_HEIGHT),
        )
        .with_spacer(BUTTON_SPACING)
        .with_child(
            Button::new("Move Down")
                .on_click(|_, state: &mut State, _| {
                    state.mutate(|editor| editor.move_component_down());
                })
                .expand_width()
                .fix_height(BUTTON_HEIGHT),
        )
}

fn components_list() -> impl Widget<State> {
    List::new(|| {
        ComponentRowWidget::new(
            Label::new(|data: &String, _env: &_| data.to_owned())
                .lens(ComponentRow::name)
                .padding(2.0)
                .expand_width(),
        )
    })
    .border(BUTTON_BORDER, 1.0)
}

fn components_editor() -> impl Widget<State> {
    Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(side_buttons().fix_width(120.0))
        .with_spacer(SPACING)
        .with_flex_child(components_list(), 1.0)
}

fn settings_editor() -> impl Widget<State> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Flex::row()
                .with_child(
                    Button::new("Layout")
                        .on_click(|_, state: &mut State, _| {
                            state.on_component_settings_tab = false;
                        })
                        .env_scope(|env, data: &State| {
                            if !data.on_component_settings_tab {
                                env.set(theme::BUTTON_LIGHT, BUTTON_ACTIVE_TOP);
                                env.set(theme::BUTTON_DARK, BUTTON_ACTIVE_BOTTOM);
                            }
                        }),
                )
                .with_child(
                    Button::new("Component")
                        .on_click(|_, state: &mut State, _| {
                            state.on_component_settings_tab = true;
                        })
                        .env_scope(|env, data: &State| {
                            if data.on_component_settings_tab {
                                env.set(theme::BUTTON_LIGHT, BUTTON_ACTIVE_TOP);
                                env.set(theme::BUTTON_DARK, BUTTON_ACTIVE_BOTTOM);
                            }
                        }),
                )
                .env_scope(|env, _| {
                    env.set(theme::BUTTON_BORDER_RADIUS, 0.0);
                }),
        )
        .with_child(settings_table::widget())
        .expand_width()
}

fn editor() -> impl Widget<State> {
    Scroll::new(
        Flex::column()
            .with_child(components_editor())
            .with_spacer(SPACING)
            .with_child(settings_editor())
            .padding(MARGIN),
    )
    .vertical()
    .expand_height()
}

pub fn root_widget() -> impl Widget<State> {
    Flex::column()
        .with_flex_child(editor(), 1.0)
        .with_child(dialog_buttons())
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
