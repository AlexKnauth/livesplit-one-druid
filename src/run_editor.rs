use std::{cell::RefCell, rc::Rc};

use druid::{
    commands,
    lens::Identity,
    text::{Formatter, Selection, Validation, ValidationError},
    theme,
    widget::{
        Button, ClipBox, Container, CrossAxisAlignment, Flex, Label, List, ListIter,
        Painter, Scroll, Switch, TextBox,
    },
    BoxConstraints, Color, Data, Env, Event, EventCtx, LayoutCtx, Lens, LensExt,
    LifeCycle, LifeCycleCtx, LinearGradient, Menu, MenuItem, PaintCtx, RenderContext, Selector,
    Size, TextAlignment, UnitPoint, UpdateCtx, Widget, WidgetExt,
};
use livesplit_core::{
    run::editor, settings::ImageCache, RunEditor, TimeSpan, TimingMethod
};

use crate::{
    config::Config,
    consts::{
        switch_style, ATTEMPTS_OFFSET_WIDTH, BUTTON_ACTIVE_BOTTOM, BUTTON_ACTIVE_TOP,
        BUTTON_BORDER, BUTTON_HEIGHT, BUTTON_SPACING, COLUMN_LABEL_FONT, DIALOG_BUTTON_HEIGHT,
        DIALOG_BUTTON_WIDTH, GRID_BORDER, ICON_SIZE, MARGIN, SPACING, TABLE_HORIZONTAL_MARGIN,
        TIME_COLUMN_WIDTH,
    },
    formatter_scope::{formatted, optional_time_span, validated, OnFocusLoss},
    MainState,
};

struct SegmentWidget<T> {
    inner: T,
}

impl<T> SegmentWidget<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: Widget<Segment>> Widget<Segment> for SegmentWidget<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Segment, env: &Env) {
        if let Event::MouseDown(event) = event {
            if !data.state.segments[data.index]
                .selected
                .is_selected_or_active()
            {
                ctx.request_focus();
                if event.mods.shift() {
                    data.select_range = true;
                } else if event.mods.ctrl() {
                    data.select_additionally = true;
                } else {
                    data.select_only = true;
                }
            } else if event.mods.ctrl() {
                data.unselect = true;
            }
        }
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &Segment, env: &Env) {
        // if let &LifeCycle::FocusChanged(has_now_focus) = event {
        //     let is_selected = data.state.segments[data.index]
        //         .selected
        //         .is_selected_or_active();
        //     if has_now_focus && !is_selected {
        //         data.select = true;
        //     }
        // }
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &Segment, data: &Segment, env: &Env) {
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
        data: &Segment,
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Segment, env: &Env) {
        let rect = ctx.size().to_rect();
        if data.state.segments[data.index]
            .selected
            .is_selected_or_active()
        {
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

#[derive(Clone, Data)]
pub struct State {
    state: Rc<editor::State>,
    config: Rc<RefCell<Config>>,
    // image: Rc<ImageBuf>,
    #[data(ignore)]
    pub editor: Rc<RefCell<Option<RunEditor>>>,
    #[data(ignore)]
    pub closed_with_ok: bool,
    image_cache: Rc<RefCell<ImageCache>>,
}

impl State {
    pub fn new(editor: RunEditor, config: Rc<RefCell<Config>>) -> Self {
        let mut image_cache = ImageCache::new();
        let state = Rc::new(editor.state(&mut image_cache));
        // let image = image::load_from_memory(state.icon_change.as_deref().unwrap())
        //     .unwrap()
        //     .into_rgba8();
        // let image = Rc::new(ImageBuf::from_raw(
        //     image.as_raw().as_slice(),
        //     ImageFormat::RgbaSeparate,
        //     image.width() as _,
        //     image.height() as _,
        // ));

        Self {
            state,
            config,
            // image,
            editor: Rc::new(RefCell::new(Some(editor))),
            closed_with_ok: false,
            image_cache: Rc::new(RefCell::new(image_cache)),
        }
    }
}

fn game_icon() -> impl Widget<State> {
    Container::new(Flex::row())
        // .background(Color::grey8(0x16))
        .background(Painter::new(|_ctx, _state: &State, _| {
            // let matrix = FillStrat::Contain.affine_to_fill(ctx.size(), state.image.size());
            // ctx.with_save(|ctx| {
            //     ctx.transform(matrix);
            //     let image = state.image.to_image(ctx.render_ctx);
            //     ctx.draw_image(
            //         &image,
            //         state.image.size().to_rect(),
            //         InterpolationMode::Bilinear,
            //     );
            // })
        }))
        .padding(BUTTON_SPACING)
        .border(BUTTON_BORDER, 1.0)
        .on_click(|_ctx, _, _| {
            // TODO:
            // let menu = MenuDesc::new(LocalizedString::new("foo"))
            //     .append(druid::platform_menus::win::file::open())
            //     .append_separator()
            //     .append(druid::platform_menus::win::file::exit());
            // ctx.show_context_menu::<State>(ContextMenu::new(menu, Point::ZERO));
        })
        .fix_size(ICON_SIZE, ICON_SIZE)
}

fn game_name() -> impl Widget<State> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new("Game"))
        .with_spacer(BUTTON_SPACING)
        .with_child(
            TextBox::new()
                .lens(Identity.map(
                    |state: &State| state.state.game.clone(),
                    |state: &mut State, name: String| {
                        let mut editor = state.editor.borrow_mut();
                        let editor = editor.as_mut().unwrap();
                        editor.set_game_name(name);
                        state.state = Rc::new(editor.state(&mut state.image_cache.borrow_mut()));
                    },
                ))
                .expand_width(),
        )
}

fn category_name() -> impl Widget<State> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new("Category"))
        .with_spacer(BUTTON_SPACING)
        .with_child(
            TextBox::new()
                .lens(Identity.map(
                    |state: &State| state.state.category.clone(),
                    |state: &mut State, name: String| {
                        let mut editor = state.editor.borrow_mut();
                        let editor = editor.as_mut().unwrap();
                        editor.set_category_name(name);
                        state.state = Rc::new(editor.state(&mut state.image_cache.borrow_mut()));
                    },
                ))
                .expand_width(),
        )
}

struct TimeSpanFormatter;

impl Formatter<String> for TimeSpanFormatter {
    fn format(&self, value: &String) -> String {
        value.clone()
    }

    fn validate_partial_input(&self, input: &str, _sel: &Selection) -> Validation {
        match input.parse::<TimeSpan>() {
            Ok(_) => Validation::success(),
            Err(e) => Validation::failure(e),
        }
    }

    fn value(&self, input: &str) -> Result<String, ValidationError> {
        input.parse::<TimeSpan>().map_err(ValidationError::new)?;
        Ok(input.to_string())
    }
}

fn offset() -> impl Widget<State> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new("Start Timer at").align_right())
        .with_spacer(BUTTON_SPACING)
        .with_child(
            validated(
                TextBox::new().with_text_alignment(TextAlignment::End),
                |offset| offset.parse::<TimeSpan>().is_ok(),
            )
            .lens(Identity.map(
                |state: &State| state.state.offset.clone(),
                |state: &mut State, value: String| {
                    let mut editor = state.editor.borrow_mut();
                    let editor = editor.as_mut().unwrap();
                    let _ = editor.parse_and_set_offset(&value);
                    state.state = Rc::new(editor.state(&mut state.image_cache.borrow_mut()));
                },
            ))
            .expand_width(),
        )
}

fn attempts() -> impl Widget<State> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new("Attempts").align_right())
        .with_spacer(BUTTON_SPACING)
        .with_child(
            formatted(
                TextBox::new().with_text_alignment(TextAlignment::End),
                |buf, val| {
                    use std::fmt::Write;
                    let _ = write!(buf, "{}", val);
                },
                |val| val.parse().ok(),
            )
            .lens(Identity.map(
                |state: &State| state.state.attempts,
                |state: &mut State, value: u32| {
                    let mut editor = state.editor.borrow_mut();
                    let editor = editor.as_mut().unwrap();
                    editor.set_attempt_count(value);
                    state.state = Rc::new(editor.state(&mut state.image_cache.borrow_mut()));
                },
            ))
            .expand_width(),
        )
}

fn header() -> impl Widget<State> {
    Flex::column()
        .with_child(
            Flex::row()
                .with_flex_child(game_name(), 2.0)
                .with_spacer(SPACING)
                .with_child(offset().fix_width(ATTEMPTS_OFFSET_WIDTH)),
        )
        .with_spacer(SPACING)
        .with_child(
            Flex::row()
                .with_flex_child(category_name(), 2.0)
                .with_spacer(SPACING)
                .with_child(attempts().fix_width(ATTEMPTS_OFFSET_WIDTH)),
        )
}

fn side_buttons() -> impl Widget<State> {
    Flex::column()
        .with_child(
            Button::new("Insert Above")
                .on_click(|_, state: &mut State, _| {
                    let mut editor = state.editor.borrow_mut();
                    let editor = editor.as_mut().unwrap();
                    editor.insert_segment_above();
                    state.state = Rc::new(editor.state(&mut state.image_cache.borrow_mut()));
                })
                .expand_width()
                .fix_height(BUTTON_HEIGHT),
        )
        .with_spacer(BUTTON_SPACING)
        .with_child(
            Button::new("Insert Below")
                .on_click(|_, state: &mut State, _| {
                    let mut editor = state.editor.borrow_mut();
                    let editor = editor.as_mut().unwrap();
                    editor.insert_segment_below();
                    state.state = Rc::new(editor.state(&mut state.image_cache.borrow_mut()));
                })
                .expand_width()
                .fix_height(BUTTON_HEIGHT),
        )
        .with_spacer(BUTTON_SPACING)
        .with_child(
            Button::new("Remove Segment")
                .on_click(|_, state: &mut State, _| {
                    let mut editor = state.editor.borrow_mut();
                    let editor = editor.as_mut().unwrap();
                    editor.remove_segments();
                    state.state = Rc::new(editor.state(&mut state.image_cache.borrow_mut()));
                })
                .expand_width()
                .fix_height(BUTTON_HEIGHT),
        )
        .with_spacer(BUTTON_SPACING)
        .with_child(
            Button::new("Move Up")
                .on_click(|_, state: &mut State, _| {
                    let mut editor = state.editor.borrow_mut();
                    let editor = editor.as_mut().unwrap();
                    editor.move_segments_up();
                    state.state = Rc::new(editor.state(&mut state.image_cache.borrow_mut()));
                })
                .expand_width()
                .fix_height(BUTTON_HEIGHT),
        )
        .with_spacer(BUTTON_SPACING)
        .with_child(
            Button::new("Move Down")
                .on_click(|_, state: &mut State, _| {
                    let mut editor = state.editor.borrow_mut();
                    let editor = editor.as_mut().unwrap();
                    editor.move_segments_down();
                    state.state = Rc::new(editor.state(&mut state.image_cache.borrow_mut()));
                })
                .expand_width()
                .fix_height(BUTTON_HEIGHT),
        )
        .with_spacer(BUTTON_SPACING)
        .with_child(OtherButtonWidget::new(
            Button::new("Other...")
                .expand_width()
                .fix_height(BUTTON_HEIGHT),
        ))
}

impl ListIter<Segment> for State {
    fn for_each(&self, mut cb: impl FnMut(&Segment, usize)) {
        let mut segment = Segment {
            index: 0,
            state: self.state.clone(),
            new_name: None,
            new_split_time: None,
            new_segment_time: None,
            new_best_segment_time: None,
            select_only: false,
            select_additionally: false,
            select_range: false,
            unselect: false,
        };
        for index in 0..self.data_len() {
            segment.index = index;
            cb(&segment, index);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut Segment, usize)) {
        let mut segment = Segment {
            index: 0,
            state: self.state.clone(),
            new_name: None,
            new_split_time: None,
            new_segment_time: None,
            new_best_segment_time: None,
            select_only: false,
            select_additionally: false,
            select_range: false,
            unselect: false,
        };
        let mut editor = self.editor.borrow_mut();
        let editor = editor.as_mut().unwrap();
        let mut changed = false;

        for index in 0..self.data_len() {
            segment.index = index;
            cb(&mut segment, index);
            if let Some(new_name) = segment.new_name.take() {
                editor.select_only(index);
                editor.active_segment().set_name(new_name);
                changed = true;
            }
            if let Some(new_split_time) = segment.new_split_time.take() {
                editor.select_only(index);
                let _ = editor
                    .active_segment()
                    .parse_and_set_split_time(&new_split_time);
                changed = true;
            }
            if let Some(new_segment_time) = segment.new_segment_time.take() {
                editor.select_only(index);
                let _ = editor
                    .active_segment()
                    .parse_and_set_segment_time(&new_segment_time);
                changed = true;
            }
            if let Some(new_best_segment_time) = segment.new_best_segment_time.take() {
                editor.select_only(index);
                let _ = editor
                    .active_segment()
                    .parse_and_set_best_segment_time(&new_best_segment_time);
                changed = true;
            }
            if segment.select_only {
                editor.select_only(index);
                segment.select_only = false;
                changed = true;
            }
            if segment.select_additionally {
                editor.select_additionally(index);
                segment.select_additionally = false;
                changed = true;
            }
            if segment.select_range {
                editor.select_range(index);
                segment.select_range = false;
                changed = true;
            }
            if segment.unselect {
                editor.unselect(index);
                segment.unselect = false;
                changed = true;
            }
        }

        if changed {
            self.state = Rc::new(editor.state(&mut self.image_cache.borrow_mut()));
        }
    }

    fn data_len(&self) -> usize {
        self.state.segments.len()
    }
}

#[derive(Clone, Data)]
struct Segment {
    index: usize,
    state: Rc<editor::State>,
    new_name: Option<String>,
    new_split_time: Option<String>,
    new_segment_time: Option<String>,
    new_best_segment_time: Option<String>,
    select_only: bool,
    select_additionally: bool,
    select_range: bool,
    unselect: bool,
}

fn segments() -> impl Widget<State> {
    Flex::column()
        .with_child(
            Flex::row()
                .with_spacer(TABLE_HORIZONTAL_MARGIN)
                .with_flex_child(
                    ClipBox::unmanaged(Label::new("Segment Name").with_font(COLUMN_LABEL_FONT))
                        .expand_width(),
                    1.0,
                )
                .with_spacer(GRID_BORDER)
                .with_child(
                    ClipBox::unmanaged(Label::new("Split Time").with_font(COLUMN_LABEL_FONT))
                        .align_right()
                        .fix_width(TIME_COLUMN_WIDTH),
                )
                .with_spacer(GRID_BORDER)
                .with_child(
                    ClipBox::unmanaged(Label::new("Segment Time").with_font(COLUMN_LABEL_FONT))
                        .align_right()
                        .fix_width(TIME_COLUMN_WIDTH),
                )
                .with_spacer(GRID_BORDER)
                .with_child(
                    ClipBox::unmanaged(Label::new("Best Segment").with_font(COLUMN_LABEL_FONT))
                        .align_right()
                        .fix_width(TIME_COLUMN_WIDTH),
                )
                .with_spacer(TABLE_HORIZONTAL_MARGIN)
                .fix_height(26.0)
                .border(BUTTON_BORDER, 1.0),
        )
        // .with_spacer(GRID_BORDER)
        .with_flex_child(
            Scroll::new(
                List::new(|| {
                    SegmentWidget::new(
                        Flex::row()
                            .with_spacer(TABLE_HORIZONTAL_MARGIN)
                            .with_flex_child(
                                TextBox::new()
                                    .lens(Identity.map(
                                        |s: &Segment| s.state.segments[s.index].name.clone(),
                                        |state: &mut Segment, name: String| {
                                            if name != state.state.segments[state.index].name {
                                                state.new_name = Some(name);
                                            }
                                        },
                                    ))
                                    .expand_width(),
                                1.0,
                            )
                            .with_spacer(GRID_BORDER)
                            .with_child(
                                OnFocusLoss::new(optional_time_span(
                                    TextBox::new().with_text_alignment(TextAlignment::End),
                                ))
                                .lens(Identity.map(
                                    |s: &Segment| s.state.segments[s.index].split_time.clone(),
                                    |state: &mut Segment, split_time: String| {
                                        if split_time
                                            != state.state.segments[state.index].split_time
                                        {
                                            state.new_split_time = Some(split_time);
                                        }
                                    },
                                ))
                                .fix_width(TIME_COLUMN_WIDTH),
                            )
                            .with_spacer(GRID_BORDER)
                            .with_child(
                                OnFocusLoss::new(optional_time_span(
                                    TextBox::new().with_text_alignment(TextAlignment::End),
                                ))
                                .lens(Identity.map(
                                    |s: &Segment| s.state.segments[s.index].segment_time.clone(),
                                    |state: &mut Segment, segment_time: String| {
                                        if segment_time
                                            != state.state.segments[state.index].segment_time
                                        {
                                            state.new_segment_time = Some(segment_time);
                                        }
                                    },
                                ))
                                .fix_width(TIME_COLUMN_WIDTH),
                            )
                            .with_spacer(GRID_BORDER)
                            .with_child(
                                OnFocusLoss::new(optional_time_span(
                                    TextBox::new().with_text_alignment(TextAlignment::End),
                                ))
                                .lens(Identity.map(
                                    |s: &Segment| {
                                        s.state.segments[s.index].best_segment_time.clone()
                                    },
                                    |state: &mut Segment, best_segment_time: String| {
                                        if best_segment_time
                                            != state.state.segments[state.index].best_segment_time
                                        {
                                            state.new_best_segment_time = Some(best_segment_time);
                                        }
                                    },
                                ))
                                .fix_width(TIME_COLUMN_WIDTH),
                            )
                            .with_spacer(TABLE_HORIZONTAL_MARGIN),
                    )
                })
                .border(BUTTON_BORDER, 1.0),
            )
            .vertical(),
            1.0,
        )
        .env_scope(|env, _| {
            env.set(theme::TEXTBOX_BORDER_RADIUS, 0.0);
            env.set(theme::TEXTBOX_BORDER_WIDTH, 0.0);
            env.set(theme::BACKGROUND_LIGHT, Color::rgba8(0, 0, 0, 0));
        })
}

fn tabs() -> impl Widget<State> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Flex::row()
                .with_child(
                    Button::new("Real Time")
                        .on_click(|_, state: &mut State, _| {
                            let mut editor = state.editor.borrow_mut();
                            let editor = editor.as_mut().unwrap();
                            editor.select_timing_method(TimingMethod::RealTime);
                            state.state = Rc::new(editor.state(&mut state.image_cache.borrow_mut()));
                        })
                        .env_scope(|env, data: &State| {
                            if data.state.timing_method == TimingMethod::RealTime {
                                env.set(theme::BUTTON_LIGHT, BUTTON_ACTIVE_TOP);
                                env.set(theme::BUTTON_DARK, BUTTON_ACTIVE_BOTTOM);
                            }
                        }),
                )
                .with_child(
                    Button::new("Game Time")
                        .on_click(|_, state: &mut State, _| {
                            let mut editor = state.editor.borrow_mut();
                            let editor = editor.as_mut().unwrap();
                            editor.select_timing_method(TimingMethod::GameTime);
                            state.state = Rc::new(editor.state(&mut state.image_cache.borrow_mut()));
                        })
                        .env_scope(|env, data: &State| {
                            if data.state.timing_method == TimingMethod::GameTime {
                                env.set(theme::BUTTON_LIGHT, BUTTON_ACTIVE_TOP);
                                env.set(theme::BUTTON_DARK, BUTTON_ACTIVE_BOTTOM);
                            }
                        }),
                )
                .env_scope(|env, _| {
                    env.set(theme::BUTTON_BORDER_RADIUS, 0.0);
                }),
        )
        .with_flex_child(segments(), 1.0)
}

fn body() -> impl Widget<State> {
    Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(side_buttons().fix_width(ICON_SIZE))
        .with_spacer(SPACING)
        .with_flex_child(tabs(), 1.0)
}

fn run_editor() -> impl Widget<State> {
    Flex::column()
        .with_child(
            Flex::row()
                .with_child(game_icon())
                .with_spacer(SPACING)
                .with_flex_child(header(), 1.0),
        )
        .with_spacer(SPACING)
        .with_flex_child(body(), 1.0)
}

struct Unwrap;

impl<T> Lens<Option<T>, T> for Unwrap {
    fn with<V, F: FnOnce(&T) -> V>(&self, data: &Option<T>, f: F) -> V {
        f(data.as_ref().unwrap())
    }

    fn with_mut<V, F: FnOnce(&mut T) -> V>(&self, data: &mut Option<T>, f: F) -> V {
        f(data.as_mut().unwrap())
    }
}

struct RunEditorWidget<T> {
    inner: T,
}

impl<T> RunEditorWidget<T> {
    #[allow(dead_code)]
    fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: Widget<Option<State>>> Widget<Option<State>> for RunEditorWidget<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Option<State>, env: &Env) {
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Option<State>,
        env: &Env,
    ) {
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &Option<State>,
        data: &Option<State>,
        env: &Env,
    ) {
        self.inner.update(ctx, old_data, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Option<State>,
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Option<State>, env: &Env) {
        self.inner.paint(ctx, data, env)
    }
}

struct LinkLayout<W>(W);

impl<W: Widget<bool>> Widget<State> for LinkLayout<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut State, env: &Env) {
        let had_linked_layout = has_linked_layout(data);
        let mut has_linked_layout = had_linked_layout;

        self.0.event(ctx, event, &mut has_linked_layout, env);

        if has_linked_layout && !had_linked_layout {
            data.config
                .borrow()
                .link_layout(data.editor.borrow_mut().as_mut().unwrap());
        } else if !has_linked_layout && had_linked_layout {
            let mut editor = data.editor.borrow_mut();
            let editor = editor.as_mut().unwrap();
            editor.set_linked_layout(None);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &State, env: &Env) {
        self.0.lifecycle(ctx, event, &has_linked_layout(data), env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &State, data: &State, env: &Env) {
        self.0.update(
            ctx,
            &has_linked_layout(old_data),
            &has_linked_layout(data),
            env,
        )
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &State,
        env: &Env,
    ) -> Size {
        self.0.layout(ctx, bc, &has_linked_layout(data), env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &State, env: &Env) {
        self.0.paint(ctx, &has_linked_layout(data), env)
    }
}

fn has_linked_layout(data: &State) -> bool {
    data.editor
        .borrow()
        .as_ref()
        .unwrap()
        .run()
        .linked_layout()
        .is_some()
}

fn link_layout_toggle() -> impl Widget<State> {
    LinkLayout(Switch::new().env_scope(|env, _| switch_style(env)))
}

pub fn root_widget() -> impl Widget<State> {
    Flex::column()
        .with_flex_child(run_editor(), 1.0)
        .with_spacer(MARGIN)
        .with_child(
            Flex::row()
                .with_child(link_layout_toggle())
                .with_spacer(BUTTON_SPACING)
                .with_child(Label::new("Link Layout"))
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
                        .on_click(|ctx, _state, _| {
                            ctx.submit_command(commands::CLOSE_WINDOW);
                        })
                        .fix_size(DIALOG_BUTTON_WIDTH, DIALOG_BUTTON_HEIGHT),
                ),
        )
        .padding(MARGIN)
}

struct OtherButtonWidget<T> {
    inner: T,
}

impl<T> OtherButtonWidget<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
}

const CLEAR_HISTORY: Selector = Selector::new("run-editor-clear-history");
const CLEAR_TIMES: Selector = Selector::new("run-editor-clear-times");
const CLEAN_SUM_OF_BEST: Selector = Selector::new("run-editor-clean-sum-of-best");
const GENERATE_GOAL_COMPARISON: Selector = Selector::new("run-editor-generate-goal-comparison");

impl<T: Widget<State>> Widget<State> for OtherButtonWidget<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut State, env: &Env) {
        if let Event::MouseDown(event) = event {
            ctx.show_context_menu::<MainState>(
                Menu::new("Other")
                    .entry(MenuItem::new("Clear History").command(CLEAR_HISTORY))
                    .entry(MenuItem::new("Clear Times").command(CLEAR_TIMES))
                    .entry(
                        MenuItem::new("Clean Sum of Best")
                            .command(CLEAN_SUM_OF_BEST)
                            .enabled(false),
                    )
                    .entry(
                        MenuItem::new("Generate Goal Comparison")
                            .command(GENERATE_GOAL_COMPARISON)
                            .enabled(false),
                    ),
                event.window_pos,
            );
            return;
        } else if let Event::Command(command) = event {
            if command.is(CLEAR_HISTORY) {
                let mut editor = data.editor.borrow_mut();
                let editor = editor.as_mut().unwrap();
                editor.clear_history();
                data.state = Rc::new(editor.state(&mut data.image_cache.borrow_mut()));
            } else if command.is(CLEAR_TIMES) {
                let mut editor = data.editor.borrow_mut();
                let editor = editor.as_mut().unwrap();
                editor.clear_times();
                data.state = Rc::new(editor.state(&mut data.image_cache.borrow_mut()));
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
