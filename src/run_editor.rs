use std::{cell::RefCell, rc::Rc};

use druid::{
    commands,
    lens::Identity,
    theme,
    widget::{
        Button, ClipBox, Container, CrossAxisAlignment, Flex, Label, List, ListIter, Painter,
        Scroll, Switch, TextBox,
    },
    BoxConstraints, Color, Data, Env, Event, EventCtx, LayoutCtx, LensExt, LifeCycle, LifeCycleCtx,
    LinearGradient, Menu, MenuItem, PaintCtx, RenderContext, Selector, Size, TextAlignment,
    UnitPoint, UpdateCtx, Widget, WidgetExt,
};
use livesplit_core::{
    run::editor::{self, RowState},
    settings::ImageCache,
    RunEditor, TimeSpan, TimingMethod,
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

struct RowWidget<T> {
    inner: T,
}

impl<T> RowWidget<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: Widget<RowT>> Widget<RowT> for RowWidget<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut RowT, env: &Env) {
        if let Event::MouseDown(event) = event {
            if !row_state_selected_or_active(&data.state.rows[data.row_index]) {
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

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &RowT, env: &Env) {
        // if let &LifeCycle::FocusChanged(has_now_focus) = event {
        //     let is_selected = data.state.rows[data.index]
        //         .selected
        //         .is_selected_or_active();
        //     if has_now_focus && !is_selected {
        //         data.select = true;
        //     }
        // }
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &RowT, data: &RowT, env: &Env) {
        // TODO: We honestly really only need to care about its selected state
        if !old_data.same(data) {
            ctx.request_paint();
        }
        self.inner.update(ctx, old_data, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &RowT, env: &Env) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &RowT, env: &Env) {
        let rect = ctx.size().to_rect();
        if row_state_selected_or_active(&data.state.rows[data.row_index]) {
            ctx.fill(
                rect,
                &LinearGradient::new(
                    UnitPoint::TOP,
                    UnitPoint::BOTTOM,
                    (Color::rgb8(0x33, 0x73, 0xf4), Color::rgb8(0x15, 0x35, 0x74)),
                ),
            );
        } else {
            let color = if data.row_index & 1 == 0 {
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
    pub fn new(
        editor: RunEditor,
        config: Rc<RefCell<Config>>,
        image_cache: Rc<RefCell<ImageCache>>,
    ) -> Self {
        let state =
            Rc::new(editor.state(&mut image_cache.borrow_mut(), livesplit_core::Lang::English));
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
            image_cache,
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
                        state.state = Rc::new(editor.state(
                            &mut state.image_cache.borrow_mut(),
                            livesplit_core::Lang::English,
                        ));
                        state.image_cache.borrow_mut().collect();
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
                        state.state = Rc::new(editor.state(
                            &mut state.image_cache.borrow_mut(),
                            livesplit_core::Lang::English,
                        ));
                        state.image_cache.borrow_mut().collect();
                    },
                ))
                .expand_width(),
        )
}

fn offset() -> impl Widget<State> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new("Start Timer at").align_right())
        .with_spacer(BUTTON_SPACING)
        .with_child(
            validated(
                TextBox::new().with_text_alignment(TextAlignment::End),
                |offset| TimeSpan::parse(offset, livesplit_core::Lang::English).is_ok(),
            )
            .lens(Identity.map(
                |state: &State| state.state.offset.clone(),
                |state: &mut State, value: String| {
                    let mut editor = state.editor.borrow_mut();
                    let editor = editor.as_mut().unwrap();
                    let _ = editor.parse_and_set_offset(&value, livesplit_core::Lang::English);
                    state.state = Rc::new(editor.state(
                        &mut state.image_cache.borrow_mut(),
                        livesplit_core::Lang::English,
                    ));
                    state.image_cache.borrow_mut().collect();
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
                    state.state = Rc::new(editor.state(
                        &mut state.image_cache.borrow_mut(),
                        livesplit_core::Lang::English,
                    ));
                    state.image_cache.borrow_mut().collect();
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
                    // image cache stuff common to all of these
                    state.state = Rc::new(editor.state(
                        &mut state.image_cache.borrow_mut(),
                        livesplit_core::Lang::English,
                    ));
                    state.image_cache.borrow_mut().collect();
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
                    // image cache stuff common to all of these
                    state.state = Rc::new(editor.state(
                        &mut state.image_cache.borrow_mut(),
                        livesplit_core::Lang::English,
                    ));
                    state.image_cache.borrow_mut().collect();
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
                    // image cache stuff common to all of these
                    state.state = Rc::new(editor.state(
                        &mut state.image_cache.borrow_mut(),
                        livesplit_core::Lang::English,
                    ));
                    state.image_cache.borrow_mut().collect();
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
                    // image cache stuff common to all of these
                    state.state = Rc::new(editor.state(
                        &mut state.image_cache.borrow_mut(),
                        livesplit_core::Lang::English,
                    ));
                    state.image_cache.borrow_mut().collect();
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
                    // image cache stuff common to all of these
                    state.state = Rc::new(editor.state(
                        &mut state.image_cache.borrow_mut(),
                        livesplit_core::Lang::English,
                    ));
                    state.image_cache.borrow_mut().collect();
                })
                .expand_width()
                .fix_height(BUTTON_HEIGHT),
        )
        .with_spacer(BUTTON_SPACING)
        .with_child(
            Button::new("Create Group")
                .on_click(|_, state: &mut State, _| {
                    let mut editor = state.editor.borrow_mut();
                    let editor = editor.as_mut().unwrap();
                    editor.create_segment_group_from_selection::<&str>(None).ok();
                    // image cache stuff common to all of these
                    state.state = Rc::new(editor.state(
                        &mut state.image_cache.borrow_mut(),
                        livesplit_core::Lang::English,
                    ));
                    state.image_cache.borrow_mut().collect();
                })
                .expand_width()
                .fix_height(BUTTON_HEIGHT),
        )
        .with_spacer(BUTTON_SPACING)
        .with_child(
            Button::new("Remove Group")
                .on_click(|_, state: &mut State, _| {
                    let mut editor = state.editor.borrow_mut();
                    let editor = editor.as_mut().unwrap();
                    editor.remove_selected_segment_groups().ok();
                    // image cache stuff common to all of these
                    state.state = Rc::new(editor.state(
                        &mut state.image_cache.borrow_mut(),
                        livesplit_core::Lang::English,
                    ));
                    state.image_cache.borrow_mut().collect();
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

impl ListIter<RowT> for State {
    fn for_each(&self, mut cb: impl FnMut(&RowT, usize)) {
        let mut row = RowT {
            row_index: 0,
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
            row.row_index = index;
            cb(&row, index);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut RowT, usize)) {
        let mut row = RowT {
            row_index: 0,
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

        for row_index in 0..self.data_len() {
            row.row_index = row_index;
            cb(&mut row, row_index);
            if let Some(new_name) = row.new_name.take() {
                match &row.state.rows[row_index] {
                    RowState::Segment(s) => {
                        editor.select_only(s.segment_index);
                        editor.active_segment().set_name(new_name);
                    }
                    RowState::SegmentGroup(g) => {
                        editor.select_segment_group(g.group_index).ok();
                        editor.rename_segment_group(g.group_index, Some(new_name)).ok();
                    }
                }
                changed = true;
            }
            if let Some(new_split_time) = row.new_split_time.take() {
                match &row.state.rows[row_index] {
                    RowState::Segment(s) => {
                        editor.select_only(s.segment_index);
                        editor
                            .active_segment()
                            .parse_and_set_split_time(&new_split_time, livesplit_core::Lang::English)
                            .ok();
                        changed = true;
                    }
                    RowState::SegmentGroup(_) => (),
                }
            }
            if let Some(new_segment_time) = row.new_segment_time.take() {
                match &row.state.rows[row_index] {
                    RowState::Segment(s) => {
                        editor.select_only(s.segment_index);
                        editor
                            .active_segment()
                            .parse_and_set_segment_time(&new_segment_time, livesplit_core::Lang::English)
                            .ok();
                        changed = true;
                    }
                    RowState::SegmentGroup(_) => (),
                }
            }
            if let Some(new_best_segment_time) = row.new_best_segment_time.take() {
                match &row.state.rows[row_index] {
                    RowState::Segment(s) => {
                        editor.select_only(s.segment_index);
                        editor.active_segment().parse_and_set_best_segment_time(
                            &new_best_segment_time,
                            livesplit_core::Lang::English,
                        ).ok();
                        changed = true;
                    }
                    RowState::SegmentGroup(_) => (),
                }
            }
            if row.select_only {
                match &row.state.rows[row_index] {
                    RowState::Segment(s) => {
                        editor.select_only(s.segment_index);
                    }
                    RowState::SegmentGroup(g) => {
                        editor.select_segment_group(g.group_index).ok();
                    }
                }
                row.select_only = false;
                changed = true;
            }
            if row.select_additionally {
                match &row.state.rows[row_index] {
                    RowState::Segment(s) => {
                        editor.select_additionally(s.segment_index);
                    }
                    RowState::SegmentGroup(_) => {
                        // TODO: how to select a group additionally?
                    }
                }
                row.select_additionally = false;
                changed = true;
            }
            if row.select_range {
                match &row.state.rows[row_index] {
                    RowState::Segment(s) => {
                        editor.select_range(s.segment_index);
                    }
                    RowState::SegmentGroup(g) => {
                        editor.select_segment_group_range(g.group_index).ok();
                    }
                }
                row.select_range = false;
                changed = true;
            }
            if row.unselect {
                match &row.state.rows[row_index] {
                    RowState::Segment(s) => {
                        editor.unselect(s.segment_index);
                    }
                    RowState::SegmentGroup(_) => {
                        // TODO: how to unselect a group?
                    }
                }
                row.unselect = false;
                changed = true;
            }
        }

        if changed {
            self.state = Rc::new(editor.state(
                &mut self.image_cache.borrow_mut(),
                livesplit_core::Lang::English,
            ));
        }
        self.image_cache.borrow_mut().collect();
    }

    fn data_len(&self) -> usize {
        self.state.rows.len()
    }
}

#[derive(Clone, Data)]
struct RowT {
    row_index: usize,
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

fn rows() -> impl Widget<State> {
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
                    RowWidget::new(
                        Flex::row()
                            .with_spacer(TABLE_HORIZONTAL_MARGIN)
                            .with_flex_child(
                                TextBox::new()
                                    .lens(Identity.map(
                                        |s: &RowT| {
                                            row_state_name(&s.state.rows[s.row_index]).to_string()
                                        },
                                        |state: &mut RowT, name: String| {
                                            if &name
                                                != row_state_name(
                                                    &state.state.rows[state.row_index],
                                                )
                                            {
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
                                    |s: &RowT| {
                                        row_state_split_time(&s.state.rows[s.row_index]).to_string()
                                    },
                                    |state: &mut RowT, split_time: String| {
                                        if let RowState::Segment(s) =
                                            &state.state.rows[state.row_index]
                                        {
                                            if split_time != s.split_time {
                                                state.new_split_time = Some(split_time);
                                            }
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
                                    |s: &RowT| {
                                        row_state_segment_time(&s.state.rows[s.row_index])
                                            .to_string()
                                    },
                                    |state: &mut RowT, segment_time: String| {
                                        if let RowState::Segment(s) =
                                            &state.state.rows[state.row_index]
                                        {
                                            if segment_time != s.segment_time {
                                                state.new_segment_time = Some(segment_time);
                                            }
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
                                    |s: &RowT| {
                                        row_state_best_segment_time(&s.state.rows[s.row_index])
                                            .to_string()
                                    },
                                    |state: &mut RowT, best_segment_time: String| {
                                        if let RowState::Segment(s) =
                                            &state.state.rows[state.row_index]
                                        {
                                            if best_segment_time != s.best_segment_time {
                                                state.new_best_segment_time =
                                                    Some(best_segment_time);
                                            }
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
                            state.state = Rc::new(editor.state(
                                &mut state.image_cache.borrow_mut(),
                                livesplit_core::Lang::English,
                            ));
                            state.image_cache.borrow_mut().collect();
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
                            state.state = Rc::new(editor.state(
                                &mut state.image_cache.borrow_mut(),
                                livesplit_core::Lang::English,
                            ));
                            state.image_cache.borrow_mut().collect();
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
        .with_flex_child(rows(), 1.0)
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
                data.state = Rc::new(editor.state(
                    &mut data.image_cache.borrow_mut(),
                    livesplit_core::Lang::English,
                ));
            } else if command.is(CLEAR_TIMES) {
                let mut editor = data.editor.borrow_mut();
                let editor = editor.as_mut().unwrap();
                editor.clear_times();
                data.state = Rc::new(editor.state(
                    &mut data.image_cache.borrow_mut(),
                    livesplit_core::Lang::English,
                ));
            }
        }
        data.image_cache.borrow_mut().collect();
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

// --------------------------------------------------------

fn row_state_selected_or_active(r: &RowState) -> bool {
    match r {
        RowState::Segment(s) => s.selected.is_selected_or_active(),
        RowState::SegmentGroup(g) => g.selected,
    }
}

fn row_state_name(r: &RowState) -> &str {
    match r {
        RowState::Segment(s) => &s.name,
        RowState::SegmentGroup(g) => &g.name,
    }
}

fn row_state_split_time(r: &RowState) -> &str {
    match r {
        RowState::Segment(s) => &s.split_time,
        RowState::SegmentGroup(_) => "",
    }
}

fn row_state_segment_time(r: &RowState) -> &str {
    match r {
        RowState::Segment(s) => &s.segment_time,
        RowState::SegmentGroup(_) => "",
    }
}

fn row_state_best_segment_time(r: &RowState) -> &str {
    match r {
        RowState::Segment(s) => &s.best_segment_time,
        RowState::SegmentGroup(_) => "",
    }
}
