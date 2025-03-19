use std::{borrow::Cow, mem, ops::Range};

use druid::{
    piet::{self, PietTextLayoutBuilder, TextAttribute, TextLayoutBuilder},
    text::{self, EditableText, StringCursor},
    Color, Command, Data, Env, Point, Selector, Target, Widget, WidgetPod,
};
use livesplit_core::TimeSpan;

#[derive(Clone)]
pub struct ValidatedString {
    value: String,
    valid: bool,
}

impl Data for ValidatedString {
    fn same(&self, other: &Self) -> bool {
        self.value == other.value && self.valid == other.valid
    }
}

impl text::TextStorage for ValidatedString {
    fn add_attributes(&self, builder: PietTextLayoutBuilder, _: &Env) -> PietTextLayoutBuilder {
        if self.valid {
            builder
        } else {
            builder.default_attribute(TextAttribute::TextColor(Color::RED))
        }
    }
}

impl piet::TextStorage for ValidatedString {
    fn as_str(&self) -> &str {
        &self.value
    }
}

impl EditableText for ValidatedString {
    fn cursor(&self, position: usize) -> Option<StringCursor> {
        self.value.cursor(position)
    }

    fn edit(&mut self, range: Range<usize>, new: impl Into<String>) {
        self.value.edit(range, new);
    }

    fn slice(&self, range: Range<usize>) -> Option<Cow<str>> {
        self.value.slice(range)
    }

    fn len(&self) -> usize {
        self.value.len()
    }

    fn prev_word_offset(&self, offset: usize) -> Option<usize> {
        self.value.prev_word_offset(offset)
    }

    fn next_word_offset(&self, offset: usize) -> Option<usize> {
        self.value.next_word_offset(offset)
    }

    fn prev_grapheme_offset(&self, offset: usize) -> Option<usize> {
        self.value.prev_grapheme_offset(offset)
    }

    fn next_grapheme_offset(&self, offset: usize) -> Option<usize> {
        self.value.next_grapheme_offset(offset)
    }

    fn prev_codepoint_offset(&self, offset: usize) -> Option<usize> {
        self.value.prev_codepoint_offset(offset)
    }

    fn next_codepoint_offset(&self, offset: usize) -> Option<usize> {
        self.value.next_codepoint_offset(offset)
    }

    fn preceding_line_break(&self, offset: usize) -> usize {
        self.value.preceding_line_break(offset)
    }

    fn next_line_break(&self, offset: usize) -> usize {
        self.value.next_line_break(offset)
    }

    fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    fn from_str(s: &str) -> Self {
        ValidatedString {
            value: s.to_string(),
            valid: true,
        }
    }
}

pub fn formatted<T: Data>(
    inner: impl Widget<ValidatedString>,
    format: impl Fn(&mut String, &T),
    parse: impl Fn(&str) -> Option<T>,
) -> impl Widget<T> {
    Formatted {
        inner,
        cached: None,
        format,
        parse,
    }
}

struct FormattedCache<T> {
    data: T,
    string_new: ValidatedString,
    string_old: ValidatedString,
}

impl<T: Clone> FormattedCache<T> {
    fn new(data: &T, buf: String) -> Self {
        Self {
            data: data.clone(),
            string_new: ValidatedString {
                value: buf.clone(),
                valid: true,
            },
            string_old: ValidatedString {
                value: buf,
                valid: true,
            },
        }
    }
}

struct Formatted<T, W, F, P> {
    inner: W,
    cached: Option<FormattedCache<T>>,
    format: F,
    parse: P,
}

impl<T, W, F, P> Widget<T> for Formatted<T, W, F, P>
where
    T: Data,
    W: Widget<ValidatedString>,
    F: Fn(&mut String, &T),
    P: Fn(&str) -> Option<T>,
{
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        outer_data: &mut T,
        env: &Env,
    ) {
        let cache = self.cached.get_or_insert_with(|| {
            let mut buf = String::new();
            (self.format)(&mut buf, outer_data);
            FormattedCache::new(outer_data, buf)
        });
        self.inner.event(ctx, event, &mut cache.string_new, env);
        if cache.string_new.value != cache.string_old.value {
            cache.string_old.value.clear();
            cache.string_old.value.push_str(&cache.string_new.value);
            if let Some(new_outer) = (self.parse)(&cache.string_old.value) {
                cache.data = new_outer.clone();
                *outer_data = new_outer;
                cache.string_new.valid = true;
            } else {
                cache.string_new.valid = false;
            }
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        data: &T,
        env: &Env,
    ) {
        let cache = self.cached.get_or_insert_with(|| {
            let mut buf = String::new();
            (self.format)(&mut buf, data);
            FormattedCache::new(data, buf)
        });
        self.inner.lifecycle(ctx, event, &cache.string_new, env)
    }

    fn update(&mut self, ctx: &mut druid::UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        let cache = self.cached.get_or_insert_with(|| {
            let mut buf = String::new();
            (self.format)(&mut buf, data);
            FormattedCache::new(data, buf)
        });
        if !data.same(&cache.data) {
            cache.data = data.clone();
            mem::swap(&mut cache.string_new, &mut cache.string_old);
            cache.string_new.valid = true;
            cache.string_new.value.clear();
            (self.format)(&mut cache.string_new.value, data);
        }
        self.inner
            .update(ctx, &cache.string_old, &cache.string_new, env)
    }

    fn layout(
        &mut self,
        ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &T,
        env: &Env,
    ) -> druid::Size {
        let cache = self.cached.get_or_insert_with(|| {
            let mut buf = String::new();
            (self.format)(&mut buf, data);
            FormattedCache::new(data, buf)
        });
        self.inner.layout(ctx, bc, &cache.string_new, env)
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &T, env: &Env) {
        let cache = self.cached.get_or_insert_with(|| {
            let mut buf = String::new();
            (self.format)(&mut buf, data);
            FormattedCache::new(data, buf)
        });
        self.inner.paint(ctx, &cache.string_new, env)
    }
}

pub fn percentage(inner: impl Widget<ValidatedString>) -> impl Widget<f64> {
    formatted(
        inner,
        |buf: &mut String, &val: &f64| {
            use std::fmt::Write;
            let _ = write!(buf, "{:.0}%", 100.0 * val);
        },
        |input: &str| {
            let parsed = input.strip_suffix('%')?.parse::<f64>().ok()?;
            (0.0..=100.0).contains(&parsed).then_some(0.01 * parsed)
        },
    )
}

struct Validated<W, F> {
    inner: W,
    cached: Option<ValidatedString>,
    validate: F,
}

impl<W: Widget<ValidatedString>, F: Fn(&str) -> bool> Widget<String> for Validated<W, F> {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        outer_data: &mut String,
        env: &Env,
    ) {
        let data = self.cached.get_or_insert_with(|| ValidatedString {
            value: outer_data.clone(),
            valid: true,
        });
        self.inner.event(ctx, event, data, env);
        if data.value != *outer_data {
            let is_valid = (self.validate)(&data.value);
            data.valid = is_valid;
            if is_valid {
                outer_data.clear();
                outer_data.push_str(&data.value);
            }
        } else {
            data.valid = true;
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        data: &String,
        env: &Env,
    ) {
        let data = self.cached.get_or_insert_with(|| ValidatedString {
            value: data.clone(),
            valid: true,
        });
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut druid::UpdateCtx, old_data: &String, data: &String, env: &Env) {
        let inner_data = self.cached.get_or_insert_with(|| ValidatedString {
            value: data.clone(),
            valid: true,
        });
        if data != old_data {
            let old_inner_data = mem::replace(
                inner_data,
                ValidatedString {
                    value: data.clone(),
                    valid: true,
                },
            );
            self.inner.update(ctx, &old_inner_data, inner_data, env)
        } else {
            self.inner.update(ctx, inner_data, inner_data, env)
        }
    }

    fn layout(
        &mut self,
        ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &String,
        env: &Env,
    ) -> druid::Size {
        let data = self.cached.get_or_insert_with(|| ValidatedString {
            value: data.clone(),
            valid: true,
        });
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &String, env: &Env) {
        let data = self.cached.get_or_insert_with(|| ValidatedString {
            value: data.clone(),
            valid: true,
        });
        self.inner.paint(ctx, data, env)
    }
}

pub fn validated(
    inner: impl Widget<ValidatedString>,
    validate: impl Fn(&str) -> bool,
) -> impl Widget<String> {
    Validated {
        inner,
        cached: None,
        validate,
    }
}

pub fn optional_time_span(inner: impl Widget<ValidatedString>) -> impl Widget<String> {
    validated(inner, |val: &str| {
        val.is_empty() || val.parse::<TimeSpan>().is_ok()
    })
}

pub struct OnFocusLoss<W> {
    inner: WidgetPod<String, W>,
    cached: Option<String>,
    had_focus: bool,
}

impl<W: Widget<String>> OnFocusLoss<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner: WidgetPod::new(inner),
            cached: None,
            had_focus: false,
        }
    }
}

const FOCUS_LOST_UPDATE_STATE: Selector = Selector::new("focus-lost-update-state");

impl<W: Widget<String>> Widget<String> for OnFocusLoss<W> {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        outer_data: &mut String,
        env: &druid::Env,
    ) {
        let data = self.cached.get_or_insert_with(|| outer_data.clone());
        self.inner.event(ctx, event, data, env);
        let has_focus = ctx.has_focus();
        if self.had_focus && !has_focus {
            outer_data.clear();
            outer_data.push_str(data);
        }
        self.had_focus = has_focus;
    }

    fn lifecycle(
        &mut self,
        ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        outer_data: &String,
        env: &druid::Env,
    ) {
        let data = self.cached.get_or_insert_with(|| outer_data.clone());
        self.inner.lifecycle(ctx, event, data, env);
        let has_focus = ctx.has_focus();
        if self.had_focus && !has_focus {
            ctx.submit_command(Command::new(
                FOCUS_LOST_UPDATE_STATE,
                (),
                Target::Widget(self.inner.id()),
            ));
        }
    }

    fn update(
        &mut self,
        ctx: &mut druid::UpdateCtx,
        old_data: &String,
        outer_data: &String,
        env: &druid::Env,
    ) {
        let data = self.cached.get_or_insert_with(|| outer_data.clone());
        if old_data != outer_data {
            data.clear();
            data.push_str(outer_data);
        }
        self.inner.update(ctx, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &String,
        env: &druid::Env,
    ) -> druid::Size {
        let data = self.cached.get_or_insert_with(|| data.clone());
        let size = self.inner.layout(ctx, bc, data, env);
        self.inner.set_origin(ctx, Point::new(0.0, 0.0));
        size
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &String, env: &druid::Env) {
        let data = self.cached.get_or_insert_with(|| data.clone());
        self.inner.paint(ctx, data, env)
    }
}
