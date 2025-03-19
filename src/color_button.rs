use druid::{
    kurbo::{Circle, Line, PathEl},
    lens::Unit,
    piet::{Image, ImageFormat, InterpolationMode, PietImage},
    theme,
    widget::{Controller, Flex, Label, Painter, Slider, TextBox},
    BoxConstraints, Color, Cursor, Data, Env, Event, EventCtx, LayoutCtx, Lens, LifeCycle,
    LifeCycleCtx, LinearGradient, MouseButton, PaintCtx, Point, RenderContext, Size, TextAlignment,
    UnitPoint, UpdateCtx, Vec2, Widget, WidgetExt, WidgetPod, WindowConfig, WindowLevel,
};
use image::{ImageBuffer, RgbaImage};

use std::fmt::Write;

use crate::formatter_scope::{formatted, percentage};

#[derive(Copy, Clone, Data, Lens)]
pub struct ColorState {
    pub hue: f64,
    pub saturation: f64,
    pub value: f64,
    pub alpha: f64,
}

impl ColorState {
    pub fn hsv(hue: f64, saturation: f64, value: f64) -> Self {
        Self {
            hue,
            saturation,
            value,
            alpha: 1.0,
        }
    }

    pub fn hsva(hue: f64, saturation: f64, value: f64, alpha: f64) -> Self {
        Self {
            hue,
            saturation,
            value,
            alpha,
        }
    }

    fn color(&self) -> Color {
        let Self {
            hue,
            saturation,
            value,
            alpha,
            ..
        } = *self;
        const RECIP_60: f64 = 1.0 / 60.0;
        let x_div_c = 1.0 - ((hue * RECIP_60) % 2.0 - 1.0).abs();
        let ((rc, rx), (gc, gx), (bc, bx)) = if hue < 60.0 {
            ((1.0, 0.0), (0.0, 1.0), (0.0, 0.0)) // yellow to red
        } else if hue < 120.0 {
            ((0.0, 1.0), (1.0, 0.0), (0.0, 0.0)) // green to yellow
        } else if hue < 180.0 {
            ((0.0, 0.0), (1.0, 0.0), (0.0, 1.0)) // cyan to green
        } else if hue < 240.0 {
            ((0.0, 0.0), (0.0, 1.0), (1.0, 0.0)) // blue to cyan
        } else if hue < 300.0 {
            ((0.0, 1.0), (0.0, 0.0), (1.0, 0.0)) // magenta to blue
        } else {
            ((1.0, 0.0), (0.0, 0.0), (0.0, 1.0)) // red to magenta
        };
        let value_times_255 = value * 255.0;
        let c_times_255 = value_times_255 * saturation;
        let x_times_255 = x_div_c * c_times_255;
        let m_times_255 = value_times_255 - c_times_255;

        let r = rc * c_times_255 + rx * x_times_255 + m_times_255;
        let g = gc * c_times_255 + gx * x_times_255 + m_times_255;
        let b = bc * c_times_255 + bx * x_times_255 + m_times_255;

        unsafe {
            Color::rgba8(
                r.to_int_unchecked(),
                g.to_int_unchecked(),
                b.to_int_unchecked(),
                (alpha * 255.0) as u8,
            )
        }
    }
}

fn draw_color_pick_image(hue: f32, image: &mut RgbaImage) {
    const RECIP_60: f32 = 1.0 / 60.0;
    let x_div_c = 1.0 - ((hue * RECIP_60) % 2.0 - 1.0).abs();

    let ((rc, rx), (gc, gx), (bc, bx)) = if hue < 60.0 {
        ((1.0, 0.0), (0.0, 1.0), (0.0, 0.0)) // yellow to red
    } else if hue < 120.0 {
        ((0.0, 1.0), (1.0, 0.0), (0.0, 0.0)) // green to yellow
    } else if hue < 180.0 {
        ((0.0, 0.0), (1.0, 0.0), (0.0, 1.0)) // cyan to green
    } else if hue < 240.0 {
        ((0.0, 0.0), (0.0, 1.0), (1.0, 0.0)) // blue to cyan
    } else if hue < 300.0 {
        ((0.0, 1.0), (0.0, 0.0), (1.0, 0.0)) // magenta to blue
    } else {
        ((1.0, 0.0), (0.0, 0.0), (0.0, 1.0)) // red to magenta
    };

    let (width, height) = image.dimensions();
    let (rec_width, rec_height_times_255) = ((width as f32).recip(), 255.0 / (height as f32));
    let height_m1 = height.saturating_sub(1);

    for (y, row) in image.enumerate_rows_mut() {
        let value_times_255 = (height_m1 - y) as f32 * rec_height_times_255;

        for (x, _, pixel) in row {
            let saturation = x as f32 * rec_width;

            let c_times_255 = value_times_255 * saturation;
            let x_times_255 = x_div_c * c_times_255;
            let m_times_255 = value_times_255 - c_times_255;

            let r = rc * c_times_255 + rx * x_times_255 + m_times_255;
            let g = gc * c_times_255 + gx * x_times_255 + m_times_255;
            let b = bc * c_times_255 + bx * x_times_255 + m_times_255;

            pixel.0 = unsafe {
                [
                    r.to_int_unchecked(),
                    g.to_int_unchecked(),
                    b.to_int_unchecked(),
                    0xFF,
                ]
            };
        }
    }
}

struct PickerController;

impl<W: Widget<ColorState>> Controller<ColorState, W> for PickerController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut ColorState,
        env: &Env,
    ) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.request_focus();
            }
            Event::MouseUp(_) => {
                ctx.set_active(false);
            }
            _ => {}
        }
        match event {
            Event::MouseDown(mouse) | Event::MouseMove(mouse) => {
                if ctx.is_active() && mouse.buttons.contains(MouseButton::Left) {
                    let size = ctx.size();
                    data.saturation = (mouse.pos.x / size.width).clamp(0.0, 1.0);
                    data.value = (1.0 - (mouse.pos.y / size.height)).clamp(0.0, 1.0);
                    ctx.request_paint();
                }
            }
            _ => {}
        }
        child.event(ctx, event, data, env)
    }
}

fn picker() -> impl Widget<ColorState> {
    let mut image_buffer = None::<RgbaImage>;
    let mut image = None::<PietImage>;
    let mut last_hue = 0.0f64;
    Painter::new(move |ctx, state: &ColorState, _| {
        let size = ctx.size();
        let bbx = size.to_rect();
        ctx.clip(bbx);
        let (width, height) = (size.width as u32 / 8, size.height as u32 / 8);
        if state.hue.to_bits() != last_hue.to_bits()
            || image.as_ref().map_or(true, |i| i.size() != size)
        {
            last_hue = state.hue;
            let mut raw_buffer = image_buffer.take().unwrap_or_default().into_raw();
            raw_buffer.resize(4 * width as usize * height as usize, 0);
            let mut new_image_buffer = ImageBuffer::from_raw(width, height, raw_buffer).unwrap();
            draw_color_pick_image(last_hue as f32, &mut new_image_buffer);
            image = ctx
                .make_image(
                    width as usize,
                    height as usize,
                    &new_image_buffer,
                    ImageFormat::RgbaPremul,
                )
                .ok();
            image_buffer = Some(new_image_buffer);
        }
        if let Some(image) = &image {
            let x = state.saturation * size.width;
            let inverted_value = 1.0 - state.value;
            let y = inverted_value * size.height;
            ctx.draw_image(image, bbx, InterpolationMode::Bilinear);
            ctx.stroke(Circle::new(Point::new(x, y), 5.0), &Color::BLACK, 1.0);
            ctx.stroke(Circle::new(Point::new(x, y), 6.0), &Color::WHITE, 1.0);
        }
    })
    .controller(PickerController)
}

fn chosen_color() -> impl Widget<ColorState> {
    let mut checkerboard = None;
    Painter::new(move |ctx, state: &ColorState, _| {
        let bbx = ctx.size().to_rect();
        let shape = Circle::new(bbx.center(), 0.5 * bbx.width().min(bbx.height()));
        let checkerboard = checkerboard.get_or_insert_with(|| {
            ctx.make_image(
                5,
                5,
                &[
                    0xB0, 0xE0, 0xB0, 0xE0, 0xB0, //
                    0xE0, 0xB0, 0xE0, 0xB0, 0xE0, //
                    0xB0, 0xE0, 0xB0, 0xE0, 0xB0, //
                    0xE0, 0xB0, 0xE0, 0xB0, 0xE0, //
                    0xB0, 0xE0, 0xB0, 0xE0, 0xB0, //
                ],
                ImageFormat::Grayscale,
            )
            .unwrap()
        });
        ctx.clip(shape);
        ctx.draw_image(checkerboard, bbx, InterpolationMode::NearestNeighbor);
        ctx.fill(shape, &state.color());
        ctx.stroke(shape, &Color::grey8(80), 2.0);
    })
}

struct HueSlider(Slider);

impl HueSlider {
    fn new() -> Self {
        Self(Slider::new().with_range(0.0, 360.0))
    }
}

impl Widget<f64> for HueSlider {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut f64, env: &Env) {
        self.0.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &f64, env: &Env) {
        self.0.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &f64, data: &f64, env: &Env) {
        self.0.update(ctx, old_data, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &f64, env: &Env) -> Size {
        self.0.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &f64, env: &Env) {
        let rect = ctx
            .size()
            .to_rect()
            .inflate(-3.0, -3.0)
            .to_rounded_rect(2.0);
        ctx.fill(
            rect,
            &LinearGradient::new(
                UnitPoint::LEFT,
                UnitPoint::RIGHT,
                &[
                    Color::rgb8(255, 0, 0),
                    Color::rgb8(255, 255, 0),
                    Color::rgb8(0, 255, 0),
                    Color::rgb8(0, 255, 255),
                    Color::rgb8(0, 0, 255),
                    Color::rgb8(255, 0, 255),
                    Color::rgb8(255, 0, 0),
                ][..],
            ),
        );
        self.0.paint(ctx, data, env)
    }
}

struct AlphaSlider(Slider, Option<PietImage>);

impl AlphaSlider {
    fn new() -> Self {
        Self(Slider::new().with_range(0.0, 1.0), None)
    }
}

impl Widget<ColorState> for AlphaSlider {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut ColorState, env: &Env) {
        self.0.event(ctx, event, &mut data.alpha, env)
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &ColorState,
        env: &Env,
    ) {
        self.0.lifecycle(ctx, event, &data.alpha, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &ColorState, data: &ColorState, env: &Env) {
        self.0.update(ctx, &old_data.alpha, &data.alpha, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &ColorState,
        env: &Env,
    ) -> Size {
        self.0.layout(ctx, bc, &data.alpha, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &ColorState, env: &Env) {
        let sharp_rect = ctx.size().to_rect().inflate(-3.0, -3.0);
        let rect = sharp_rect.to_rounded_rect(2.0);

        let checkerboard = self.1.get_or_insert_with(|| {
            let mut arr = [0; 1023 * 2];
            for val in arr.chunks_exact_mut(2) {
                val[0] = 0xB0;
                val[1] = 0xE0;
            }
            ctx.make_image(1023, 2, &arr, ImageFormat::Grayscale)
                .unwrap()
        });

        ctx.with_save(|ctx| {
            ctx.clip(rect);

            let src_width = sharp_rect.width() / sharp_rect.height() * 2.0;

            ctx.draw_image_area(
                checkerboard,
                Size::new(src_width, 2.0).to_rect(),
                sharp_rect,
                InterpolationMode::NearestNeighbor,
            );
            ctx.fill(
                sharp_rect,
                &LinearGradient::new(
                    UnitPoint::LEFT,
                    UnitPoint::RIGHT,
                    (Color::TRANSPARENT, data.color().with_alpha(1.0)),
                ),
            );
        });

        self.0.paint(ctx, &data.alpha, env)
    }
}

fn separator() -> impl Widget<()> {
    Painter::new(|ctx, &(), _| {
        let bbx = ctx.size();
        let rect = bbx.to_rect();
        let y = rect.center().y;
        ctx.clip(rect);
        ctx.stroke(
            Line::new(Point::new(rect.x0, y), Point::new(rect.x1, y)),
            &Color::grey8(0x50),
            1.0,
        );
    })
    .expand_width()
    .fix_height(1.0)
}

fn controls() -> impl Widget<ColorState> {
    Flex::column()
        .with_child(
            Flex::row()
                .with_spacer(5.0)
                .with_child(chosen_color().fix_size(35.0, 35.0))
                .with_spacer(10.0)
                .with_flex_child(
                    Flex::column()
                        .with_child(HueSlider::new().lens(ColorState::hue).expand_width())
                        .with_spacer(5.0)
                        .with_child(AlphaSlider::new().expand_width())
                        .env_scope(|env, _| {
                            env.set(theme::BACKGROUND_LIGHT, Color::TRANSPARENT);
                            env.set(theme::BACKGROUND_DARK, Color::TRANSPARENT);
                            env.set(theme::BORDER_DARK, Color::TRANSPARENT);
                        }),
                    1.0,
                ),
        )
        .with_spacer(10.0)
        .with_child(
            Flex::row()
                .with_flex_child(
                    Flex::column()
                        .with_child(
                            formatted(
                                TextBox::new().with_text_alignment(TextAlignment::Center),
                                |buf: &mut String, &val: &f64| {
                                    let _ = write!(buf, "{val:.0}°");
                                },
                                |input: &str| {
                                    let parsed = input.strip_suffix('°')?.parse::<f64>().ok()?;
                                    (0.0..=360.0).contains(&parsed).then_some(parsed)
                                },
                            )
                            .lens(ColorState::hue)
                            .expand_width(),
                        )
                        .with_spacer(5.0)
                        .with_child(
                            Label::new("H")
                                .with_text_color(Color::GRAY)
                                .with_text_size(12.0),
                        ),
                    1.0,
                )
                .with_spacer(5.0)
                .with_flex_child(
                    Flex::column()
                        .with_child(
                            percentage(TextBox::new().with_text_alignment(TextAlignment::Center))
                                .lens(ColorState::saturation)
                                .expand_width(),
                        )
                        .with_spacer(5.0)
                        .with_child(
                            Label::new("S")
                                .with_text_color(Color::grey8(0xA0))
                                .with_text_size(12.0),
                        ),
                    1.0,
                )
                .with_spacer(5.0)
                .with_flex_child(
                    Flex::column()
                        .with_child(
                            percentage(TextBox::new().with_text_alignment(TextAlignment::Center))
                                .lens(ColorState::value)
                                .expand_width(),
                        )
                        .with_spacer(5.0)
                        .with_child(
                            Label::new("V")
                                .with_text_color(Color::grey8(0xA0))
                                .with_text_size(12.0),
                        ),
                    1.0,
                )
                .with_spacer(5.0)
                .with_flex_child(
                    Flex::column()
                        .with_child(
                            percentage(TextBox::new().with_text_alignment(TextAlignment::Center))
                                .lens(ColorState::alpha)
                                .expand_width(),
                        )
                        .with_spacer(5.0)
                        .with_child(
                            Label::new("A")
                                .with_text_color(Color::grey8(0xA0))
                                .with_text_size(12.0),
                        ),
                    1.0,
                ),
        )
        .padding(10.0)
}

fn palette_button(color: ColorState) -> impl Widget<ColorState> {
    PaletteButton {
        pick: color,
        pick_color: color.color(),
    }
}

struct PaletteButton {
    pick: ColorState,
    pick_color: Color,
}

impl Widget<ColorState> for PaletteButton {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut ColorState, _env: &Env) {
        match event {
            Event::MouseDown(mouse_event) => {
                if mouse_event.button == MouseButton::Left && !ctx.is_disabled() {
                    ctx.set_active(true);
                    ctx.request_paint();
                }
            }
            Event::MouseUp(mouse_event) => {
                if ctx.is_active() && mouse_event.button == MouseButton::Left {
                    ctx.set_active(false);
                    if ctx.is_hot() && !ctx.is_disabled() {
                        *data = self.pick;
                    }
                    ctx.request_paint();
                }
            }
            Event::MouseMove(_) => {
                if !ctx.is_disabled() {
                    ctx.set_cursor(&Cursor::Pointer);
                } else {
                    ctx.set_disabled(false);
                    ctx.clear_cursor();
                }
            }
            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        _data: &ColorState,
        _env: &Env,
    ) {
        if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx,
        _old_data: &ColorState,
        _data: &ColorState,
        _env: &Env,
    ) {
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &ColorState,
        _env: &Env,
    ) -> Size {
        bc.constrain((f64::INFINITY, 20.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &ColorState, _env: &Env) {
        let mut rect = ctx.size().to_rect();
        if ctx.is_active() {
            rect.y0 += 2.0;
        }
        let rect = rect.to_rounded_rect(10.0);
        ctx.fill(rect, &self.pick_color);
        if ctx.is_hot() {
            ctx.clip(rect);
            let color = if ctx.is_active() {
                Color::WHITE
            } else {
                Color::WHITE.with_alpha(0.75)
            };
            ctx.stroke(rect, &color, 4.0);
        }
    }
}

fn palette() -> impl Widget<ColorState> {
    Flex::column()
        .with_child(
            Flex::row()
                .with_flex_child(palette_button(ColorState::hsv(4.0, 0.79, 0.96)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(340.0, 0.86, 0.91)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(291.0, 0.78, 0.69)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(262.0, 0.68, 0.71)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(231.0, 0.65, 0.71)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(207.0, 0.87, 0.95)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(199.0, 0.99, 0.95)), 1.0),
        )
        .with_spacer(10.0)
        .with_child(
            Flex::row()
                .with_flex_child(palette_button(ColorState::hsv(187.0, 1.0, 0.84)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(174.0, 1.0, 0.58)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(122.0, 0.56, 0.68)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(88.0, 0.61, 0.77)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(66.0, 0.75, 0.86)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(54.0, 0.76, 1.0)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(45.0, 0.98, 1.0)), 1.0),
        )
        .with_spacer(10.0)
        .with_child(
            Flex::row()
                .with_flex_child(palette_button(ColorState::hsv(36.0, 1.0, 1.0)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(14.0, 0.86, 1.0)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(16.0, 0.40, 0.48)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(0.0, 0.0, 0.62)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(200.0, 0.31, 0.54)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(0.0, 0.0, 0.0)), 1.0)
                .with_spacer(10.0)
                .with_flex_child(palette_button(ColorState::hsv(0.0, 0.0, 1.0)), 1.0),
        )
        .padding(10.0)
}

fn arrow() -> impl Widget<()> {
    Painter::new(|ctx, _, env| {
        let region = ctx.size().to_rect().inset((1.01, 0.5));
        ctx.clip(region);
        ctx.clear(region.expand(), Color::TRANSPARENT);
        let center = region.center().x;
        let shape = &[
            PathEl::MoveTo(Point::new(center - 15.0, region.y1)),
            PathEl::LineTo(Point::new(center, 0.0)),
            PathEl::LineTo(Point::new(center + 15.0, region.y1)),
        ][..];
        let color = env.get(theme::WINDOW_BACKGROUND_COLOR);
        ctx.fill(shape, &color);
        ctx.stroke(shape, &Color::grey8(0x80), 1.0);
    })
    .expand_width()
    .fix_height(8.0)
}

fn color_picker() -> impl Widget<ColorState> {
    Flex::column()
        .with_child(arrow().lens(Unit))
        .with_flex_child(picker(), 1.0)
        .with_child(controls())
        .with_child(separator().lens(Unit))
        .with_child(palette())
        .border(Color::grey8(0x50), 1.0)
        .controller(CloseOnFocusLoss)
}

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

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &T,
        env: &Env,
    ) {
        child.lifecycle(ctx, event, data, env)
    }
}

struct ColorButtonPod(WidgetPod<ColorState, ColorButton>);

pub fn widget() -> impl Widget<ColorState> {
    ColorButtonPod(WidgetPod::new(ColorButton))
}

impl Widget<ColorState> for ColorButtonPod {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut ColorState, env: &Env) {
        self.0.event(ctx, event, data, env)
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &ColorState,
        env: &Env,
    ) {
        self.0.lifecycle(ctx, event, data, env)
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        _old_data: &ColorState,
        data: &ColorState,
        env: &Env,
    ) {
        self.0.update(ctx, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &ColorState,
        env: &Env,
    ) -> Size {
        let size = self.0.layout(ctx, bc, data, env);
        self.0.set_origin(ctx, Point::new(0.0, 0.0));
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &ColorState, env: &Env) {
        self.0.paint(ctx, data, env)
    }
}

struct ColorButton;

impl Widget<ColorState> for ColorButton {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut ColorState, env: &Env) {
        match event {
            Event::MouseDown(mouse_event) => {
                if mouse_event.button == MouseButton::Left && !ctx.is_disabled() {
                    ctx.set_active(true);
                    ctx.request_paint();
                }
            }
            Event::MouseUp(mouse_event) => {
                if ctx.is_active() && mouse_event.button == MouseButton::Left {
                    ctx.set_active(false);
                    if ctx.is_hot() && !ctx.is_disabled() {
                        ctx.new_sub_window(
                            WindowConfig::default()
                                .show_titlebar(false)
                                .resizable(false)
                                .transparent(true)
                                .window_size(Size::new(225., 355.))
                                .set_position(ctx.to_window(
                                    ctx.size().to_rect().center()
                                        + Vec2::new(-0.5 * 225.0, 0.5 * ctx.size().height),
                                ))
                                .set_level(WindowLevel::DropDown(ctx.window().clone())),
                            color_picker(),
                            *data,
                            env.clone(),
                        );
                    }
                    ctx.request_paint();
                }
            }
            Event::MouseMove(_) => {
                if !ctx.is_disabled() {
                    ctx.set_cursor(&Cursor::Pointer);
                } else {
                    ctx.set_disabled(false);
                    ctx.clear_cursor();
                }
            }
            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &ColorState,
        _env: &Env,
    ) {
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &ColorState,
        data: &ColorState,
        _env: &Env,
    ) {
        if !old_data.same(data) {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &ColorState,
        env: &Env,
    ) -> Size {
        bc.constrain(Size::new(45.0, env.get(theme::BORDERED_WIDGET_HEIGHT)))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &ColorState, _env: &Env) {
        let stroke_width = 2.0;

        let rect = ctx
            .size()
            .to_rect()
            .inset(-stroke_width / 2.0)
            .to_rounded_rect(2.0);

        ctx.fill(rect, &data.color());
        ctx.stroke(rect, &Color::WHITE, stroke_width);
    }
}
