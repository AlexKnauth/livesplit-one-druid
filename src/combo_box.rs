use druid::{
    commands::CLOSE_WINDOW,
    kurbo::BezPath,
    lens::Identity,
    theme,
    widget::{Button, Controller, Flex, Label, LabelText, Painter, Scroll, TextBox},
    BoxConstraints, Color, Env, Event, EventCtx, LayoutCtx, LensExt, LifeCycle, LifeCycleCtx,
    PaintCtx, Point, RenderContext, Size, UpdateCtx, Widget, WidgetExt, WindowConfig, WindowLevel,
};

struct CloseOnFocusLoss;

impl<T, W: Widget<T>> Controller<T, W> for CloseOnFocusLoss {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        /*
        if let Event::WindowLostFocus = event {
            ctx.submit_command(CLOSE_WINDOW);
        }
        */
        child.event(ctx, event, data, env)
    }
}

struct ComboBox<W>(W);

impl<T, W: Widget<T>> Widget<T> for ComboBox<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.0.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.0.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.0.update(ctx, old_data, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.0.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.0.paint(ctx, data, env);
        let Size { width, height } = ctx.size();
        let l_x = width - 24.0 + 7.0;
        let t_y = 0.5 * height - 2.0;
        let r_x = width - 7.0;
        let m_x = 0.5 * (l_x + r_x);
        let m_y = 0.5 * height + 3.0;
        let mut path = BezPath::new();
        path.move_to((l_x, t_y));
        path.line_to((m_x, m_y));
        path.line_to((r_x, t_y));
        ctx.stroke(path, &env.get(theme::TEXT_COLOR), 2.0);
    }
}

pub trait ComboLabel {
    fn to_arc_str(&self) -> std::sync::Arc<str>;
    fn to_label_text<T>(&self) -> LabelText<T>;
    fn as_str(&self) -> &str;
}

pub trait ComboList: 'static + Clone {
    type Label: ComboLabel;
    fn slice(&self) -> &[Self::Label];
}

impl ComboList for &'static [&'static str] {
    type Label = &'static str;
    fn slice(&self) -> &[Self::Label] {
        self
    }
}

impl ComboLabel for &'static str {
    fn to_arc_str(&self) -> std::sync::Arc<str> {
        (*self).into()
    }

    fn to_label_text<T>(&self) -> LabelText<T> {
        (*self).into()
    }

    fn as_str(&self) -> &str {
        self
    }
}

impl<L: ComboLabel + 'static> ComboList for std::sync::Arc<[L]> {
    type Label = L;
    fn slice(&self) -> &[Self::Label] {
        self
    }
}

impl ComboLabel for std::sync::Arc<str> {
    fn to_arc_str(&self) -> std::sync::Arc<str> {
        self.clone()
    }

    fn to_label_text<T>(&self) -> LabelText<T> {
        self.to_arc_str().into()
    }

    fn as_str(&self) -> &str {
        self
    }
}

struct Pod<W>(druid::WidgetPod<String, W>);

impl<W: Widget<String>> Widget<String> for Pod<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut String, env: &Env) {
        self.0.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &String, env: &Env) {
        self.0.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &String, data: &String, env: &Env) {
        self.0.update(ctx, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &String,
        env: &Env,
    ) -> Size {
        self.0.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &String, env: &Env) {
        self.0.paint(ctx, data, env)
    }
}

struct OnClick<W, L> {
    widget: W,
    list: L,
}
impl<W: Widget<String>, L: ComboList> Widget<String> for OnClick<W, L> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut String, env: &Env) {
        let Size { width, height } = ctx.size();
        match event {
            Event::MouseDown(event) => {
                if event.button == druid::MouseButton::Left && event.pos.x >= width - 24.0 {
                    ctx.new_sub_window(
                        WindowConfig::default()
                            .show_titlebar(false)
                            .resizable(false)
                            .transparent(true)
                            .window_size(Size::new(
                                width,
                                25.0 * self.list.slice().len().min(8) as f64 + 2.0,
                            ))
                            .set_position(ctx.to_window(Point::new(0.0, height - 1.0)))
                            .set_level(WindowLevel::DropDown(ctx.window().clone())),
                        drop_down(&self.list).lens(Identity.map(
                            {
                                let list = self.list.clone();
                                move |row: &String| {
                                    list.slice()
                                        .iter()
                                        .position(|l| l.as_str() == row)
                                        .unwrap_or(list.slice().len())
                                }
                            },
                            {
                                let list = self.list.clone();
                                move |row: &mut String, index: usize| {
                                    if let Some(element) = list.slice().get(index) {
                                        row.clear();
                                        row.push_str(element.as_str());
                                    }
                                }
                            },
                        )),
                        data.clone(),
                        env.clone(),
                    );
                }
            }
            Event::MouseMove(event) => {
                if event.pos.x >= width - 24.0 {
                    ctx.set_cursor(&druid::Cursor::Arrow);
                    return;
                }
            }
            _ => {}
        }
        self.widget.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &String, env: &Env) {
        self.widget.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &String, data: &String, env: &Env) {
        self.widget.update(ctx, old_data, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &String,
        env: &Env,
    ) -> Size {
        self.widget.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &String, env: &Env) {
        self.widget.paint(ctx, data, env);
    }
}

pub fn custom_user_string(list: impl ComboList) -> impl Widget<String> {
    Pod(druid::WidgetPod::new(ComboBox(OnClick {
        widget: TextBox::new(),
        list,
    })))
}

pub fn static_list(list: &'static [&'static str]) -> impl Widget<usize> {
    dynamic_list(list)
}

pub fn dynamic_list(list: impl ComboList) -> impl Widget<usize> {
    ComboBox(
        Button::new({
            let list = list.clone();
            move |&index: &usize, _: &_| list.slice()[index].to_arc_str()
        })
        .on_click(move |ctx, &mut index: &mut usize, env| {
            ctx.new_sub_window(
                WindowConfig::default()
                    .show_titlebar(false)
                    .resizable(false)
                    .transparent(true)
                    .window_size(Size::new(
                        ctx.size().width,
                        25.0 * list.slice().len().min(8) as f64 + 2.0,
                    ))
                    .set_position(ctx.to_window(Point::new(0.0, ctx.size().height - 1.0)))
                    .set_level(WindowLevel::DropDown(ctx.window().clone())),
                drop_down(&list),
                index,
                env.clone(),
            );
        })
        .env_scope(|env, _| {
            env.set(theme::BUTTON_BORDER_RADIUS, 0.0);
            env.set(theme::BUTTON_LIGHT, Color::grey8(0x10));
            env.set(theme::BUTTON_DARK, Color::grey8(0x10));
        }),
    )
}

fn drop_down(list: &impl ComboList) -> impl Widget<usize> {
    let mut flex = Flex::column();
    for (index, item) in list.slice().iter().enumerate() {
        let label = Label::new(item.to_label_text())
            .expand_width()
            .center()
            .fix_height(25.0)
            .padding((5.0, 0.0))
            .background(Painter::new(move |ctx, selected_index, _env| {
                let shape = ctx.size().to_rect();
                if ctx.is_hot() {
                    ctx.fill(shape, &Color::rgb8(30, 144, 255));
                } else if index == *selected_index {
                    ctx.fill(shape, &Color::rgb8(0x1e, 0x44, 0x91));
                }
            }))
            .on_click(move |ctx, selected_index, _env| {
                *selected_index = index;
                ctx.submit_command(CLOSE_WINDOW);
            });
        flex.add_child(label);
    }
    Scroll::new(flex)
        .vertical()
        .background(Color::grey8(0x10))
        .border(Color::grey8(0x50), 1.0)
        .controller(CloseOnFocusLoss)
}
