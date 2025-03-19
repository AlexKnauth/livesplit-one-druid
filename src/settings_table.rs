use std::mem;

use crate::{
    combo_box,
    consts::{switch_style, BUTTON_BORDER, GRID_BORDER},
    formatter_scope::formatted,
    hotkey_button::{self, Hotkey},
    FONT_FAMILIES,
};
use druid::{
    lens::Identity,
    widget::{Flex, Label, LineBreaking, List, ListIter, Stepper, Switch, TextBox, ViewSwitcher},
    BoxConstraints, Color, Data, Env, Event, EventCtx, Insets, LayoutCtx, Lens, LensExt, LifeCycle,
    LifeCycleCtx, PaintCtx, RenderContext, Size, TextAlignment, UpdateCtx, Widget, WidgetExt,
};
use livesplit_core::{
    component::{
        splits::{ColumnStartWith, ColumnUpdateTrigger, ColumnUpdateWith},
        timer::DeltaGradient,
    },
    layout::LayoutDirection,
    settings::{
        self, Alignment, ColumnKind, Font, FontStretch, FontStyle, FontWeight, Gradient,
        LayoutBackground, ListGradient, Value,
    },
    timing::formatter::{Accuracy, DigitsFormat},
    TimingMethod,
};

#[derive(Clone, PartialEq, Lens)]
pub struct SettingsRow {
    pub index: usize,
    pub text: String,
    pub value: Value,
}

impl Data for SettingsRow {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

struct SettingsRowWidget<T> {
    inner: T,
}

impl<T> SettingsRowWidget<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: Widget<SettingsRow>> Widget<SettingsRow> for SettingsRowWidget<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut SettingsRow, env: &Env) {
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &SettingsRow,
        env: &Env,
    ) {
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &SettingsRow,
        data: &SettingsRow,
        env: &Env,
    ) {
        // TODO: We honestly really only need to care about its index maybe?
        if !old_data.same(data) {
            ctx.request_paint();
        }
        self.inner.update(ctx, old_data, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &SettingsRow,
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &SettingsRow, env: &Env) {
        let rect = ctx.size().to_rect();

        let color = if data.index & 1 == 0 {
            Color::grey8(0x12)
        } else {
            Color::grey8(0xb)
        };
        ctx.fill(rect, &color);

        self.inner.paint(ctx, data, env)
    }
}

pub fn widget<T: ListIter<SettingsRow>>() -> impl Widget<T> {
    List::new(|| {
        SettingsRowWidget::new(
            Flex::row()
                .with_spacer(6.0)
                .with_child(
                    Label::new(|data: &String, _env: &_| data.to_owned())
                        .with_line_break_mode(LineBreaking::WordWrap)
                        .lens(SettingsRow::text)
                        .fix_width(225.0),
                )
                .with_spacer(GRID_BORDER)
                .with_flex_child(
                    ViewSwitcher::new(
                        |row: &SettingsRow, _| mem::discriminant(&row.value),
                        |_, row, _| match row.value {
                            Value::Bool(_) => Box::new(
                                Switch::new()
                                    .lens(Identity.map(
                                        |row: &SettingsRow| match row.value {
                                            Value::Bool(v) => v,
                                            // TODO: What
                                            _ => false,
                                        },
                                        |row: &mut SettingsRow, value: bool| {
                                            if let Value::Bool(v) = &mut row.value {
                                                *v = value;
                                            }
                                        },
                                    ))
                                    .env_scope(|env, _| switch_style(env))
                                    .center(),
                            ),
                            Value::OptionalString(_) => Box::new(optional_string()),
                            Value::String(_) => Box::new(
                                TextBox::new()
                                    .lens(Identity.map(
                                        |row: &SettingsRow| match &row.value {
                                            Value::String(v) => v.clone(),
                                            // TODO: What
                                            _ => String::new(),
                                        },
                                        |row: &mut SettingsRow, value: String| {
                                            if let Value::String(v) = &mut row.value {
                                                *v = value;
                                            }
                                        },
                                    ))
                                    .expand_width(),
                            ),
                            Value::UInt(_) => Box::new(
                                Flex::row()
                                    .with_flex_child(
                                        formatted(
                                            TextBox::new().with_text_alignment(TextAlignment::End),
                                            |buf, val| {
                                                use std::fmt::Write;
                                                let _ = write!(buf, "{}", val);
                                            },
                                            |val| val.parse().ok(),
                                        )
                                        .lens(Identity.map(
                                            |row: &SettingsRow| match row.value {
                                                Value::UInt(v) => v,
                                                // TODO: What
                                                _ => 0,
                                            },
                                            |row: &mut SettingsRow, value: u64| {
                                                if let Value::UInt(v) = &mut row.value {
                                                    *v = value;
                                                }
                                            },
                                        ))
                                        .expand_width(),
                                        1.0,
                                    )
                                    .with_child(Stepper::new().with_range(0.0, 100_000.0).lens(
                                        Identity.map(
                                            |row: &SettingsRow| match row.value {
                                                Value::UInt(v) => v as _,
                                                // TODO: What
                                                _ => 0.0,
                                            },
                                            |row: &mut SettingsRow, value: f64| {
                                                if let Value::UInt(v) = &mut row.value {
                                                    *v = value as _;
                                                }
                                            },
                                        ),
                                    ))
                                    .expand_width(),
                            ),
                            Value::Alignment(_) => Box::new(
                                combo_box::static_list(&["Automatic", "Left", "Center"])
                                    .lens(Identity.map(
                                        |row: &SettingsRow| match &row.value {
                                            Value::Alignment(v) => match v {
                                                Alignment::Auto => 0,
                                                Alignment::Left => 1,
                                                Alignment::Center => 2,
                                            },
                                            // TODO: What
                                            _ => 3,
                                        },
                                        |row: &mut SettingsRow, value: usize| {
                                            if let Value::Alignment(v) = &mut row.value {
                                                *v = match value {
                                                    0 => Alignment::Auto,
                                                    1 => Alignment::Left,
                                                    2 => Alignment::Center,
                                                    _ => return,
                                                };
                                            }
                                        },
                                    ))
                                    .expand_width()
                                    .center(),
                            ),
                            Value::LayoutDirection(_) => Box::new(
                                combo_box::static_list(&["Vertical", "Horizontal"])
                                    .lens(Identity.map(
                                        |row: &SettingsRow| match &row.value {
                                            Value::LayoutDirection(v) => match v {
                                                LayoutDirection::Vertical => 0,
                                                LayoutDirection::Horizontal => 1,
                                            },
                                            // TODO: What
                                            _ => 2,
                                        },
                                        |row: &mut SettingsRow, value: usize| {
                                            if let Value::LayoutDirection(v) = &mut row.value {
                                                *v = match value {
                                                    0 => LayoutDirection::Vertical,
                                                    1 => LayoutDirection::Horizontal,
                                                    _ => return,
                                                };
                                            }
                                        },
                                    ))
                                    .expand_width()
                                    .center(),
                            ),
                            Value::LayoutBackground(_) => Box::new(layout_background()),
                            Value::Color(_) => Box::new(color()),
                            Value::OptionalColor(_) => Box::new(optional_color()),
                            Value::Font(_) => Box::new(font()),
                            Value::Accuracy(_) => Box::new(accuracy()),
                            Value::DigitsFormat(_) => Box::new(digits_format()),
                            Value::Gradient(_) => Box::new(gradient()),
                            Value::ColumnStartWith(_) => Box::new(column_start_with()),
                            Value::ColumnUpdateWith(_) => Box::new(column_update_with()),
                            Value::ColumnUpdateTrigger(_) => Box::new(column_update_trigger()),
                            Value::OptionalTimingMethod(_) => Box::new(optional_timing_method()),
                            Value::ListGradient(_) => Box::new(list_gradient()),
                            Value::Hotkey(_) => Box::new(hotkey()),
                            Value::Int(_) => todo!(),
                            Value::DeltaGradient(_) => Box::new(delta_gradient()),
                            Value::ColumnKind(_) => Box::new(column_kind()),
                        },
                    ),
                    1.0,
                )
                .with_spacer(6.0)
                .padding(4.0),
        )
    })
    .border(BUTTON_BORDER, 1.0)
}

fn hotkey() -> impl Widget<SettingsRow> {
    hotkey_button::widget().lens(Identity.map(
        |row: &SettingsRow| {
            Hotkey(match &row.value {
                Value::Hotkey(v) => *v,
                _ => None,
            })
        },
        |row: &mut SettingsRow, Hotkey(value): Hotkey| {
            if let Value::Hotkey(v) = &mut row.value {
                *v = value;
            }
        },
    ))
}

fn column_kind() -> impl Widget<SettingsRow> {
    combo_box::static_list(&["Time", "Variable"])
        .lens(Identity.map(
            |row: &SettingsRow| match &row.value {
                Value::ColumnKind(v) => match v {
                    ColumnKind::Time => 0,
                    ColumnKind::Variable => 1,
                },
                // TODO: What
                _ => 2,
            },
            |row: &mut SettingsRow, value: usize| {
                if let Value::ColumnKind(v) = &mut row.value {
                    *v = match value {
                        0 => ColumnKind::Time,
                        1 => ColumnKind::Variable,
                        _ => return,
                    };
                }
            },
        ))
        .expand_width()
}

fn column_start_with() -> impl Widget<SettingsRow> {
    combo_box::static_list(&[
        "Empty",
        "Comparison Time",
        "Comparison Segment Time",
        "Possible Time Save",
    ])
    .lens(Identity.map(
        |row: &SettingsRow| match &row.value {
            Value::ColumnStartWith(v) => match v {
                ColumnStartWith::Empty => 0,
                ColumnStartWith::ComparisonTime => 1,
                ColumnStartWith::ComparisonSegmentTime => 2,
                ColumnStartWith::PossibleTimeSave => 3,
            },
            // TODO: What
            _ => 4,
        },
        |row: &mut SettingsRow, value: usize| {
            if let Value::ColumnStartWith(v) = &mut row.value {
                *v = match value {
                    0 => ColumnStartWith::Empty,
                    1 => ColumnStartWith::ComparisonTime,
                    2 => ColumnStartWith::ComparisonSegmentTime,
                    3 => ColumnStartWith::PossibleTimeSave,
                    _ => return,
                };
            }
        },
    ))
    .expand_width()
}

fn column_update_with() -> impl Widget<SettingsRow> {
    combo_box::static_list(&[
        "Don't Update",
        "Split Time",
        "Time Ahead / Behind",
        "Time Ahead / Behind or Split Time If Empty",
        "Segment Time",
        "Time Saved / Lost",
        "Time Saved / Lost or Segment Time If Empty",
    ])
    .lens(Identity.map(
        |row: &SettingsRow| match &row.value {
            Value::ColumnUpdateWith(v) => match v {
                ColumnUpdateWith::DontUpdate => 0,
                ColumnUpdateWith::SplitTime => 1,
                ColumnUpdateWith::Delta => 2,
                ColumnUpdateWith::DeltaWithFallback => 3,
                ColumnUpdateWith::SegmentTime => 4,
                ColumnUpdateWith::SegmentDelta => 5,
                ColumnUpdateWith::SegmentDeltaWithFallback => 6,
            },
            // TODO: What
            _ => 7,
        },
        |row: &mut SettingsRow, value: usize| {
            if let Value::ColumnUpdateWith(v) = &mut row.value {
                *v = match value {
                    0 => ColumnUpdateWith::DontUpdate,
                    1 => ColumnUpdateWith::SplitTime,
                    2 => ColumnUpdateWith::Delta,
                    3 => ColumnUpdateWith::DeltaWithFallback,
                    4 => ColumnUpdateWith::SegmentTime,
                    5 => ColumnUpdateWith::SegmentDelta,
                    6 => ColumnUpdateWith::SegmentDeltaWithFallback,
                    _ => return,
                };
            }
        },
    ))
    .expand_width()
}

fn column_update_trigger() -> impl Widget<SettingsRow> {
    combo_box::static_list(&["On Starting Segment", "Contextual", "On Ending Segment"])
        .lens(Identity.map(
            |row: &SettingsRow| match &row.value {
                Value::ColumnUpdateTrigger(v) => match v {
                    ColumnUpdateTrigger::OnStartingSegment => 0,
                    ColumnUpdateTrigger::Contextual => 1,
                    ColumnUpdateTrigger::OnEndingSegment => 2,
                },
                // TODO: What
                _ => 3,
            },
            |row: &mut SettingsRow, value: usize| {
                if let Value::ColumnUpdateTrigger(v) = &mut row.value {
                    *v = match value {
                        0 => ColumnUpdateTrigger::OnStartingSegment,
                        1 => ColumnUpdateTrigger::Contextual,
                        2 => ColumnUpdateTrigger::OnEndingSegment,
                        _ => return,
                    };
                }
            },
        ))
        .expand_width()
}

fn gradient() -> impl Widget<SettingsRow> {
    Flex::column()
        .with_child(
            combo_box::static_list(&["Transparent", "Plain", "Vertical", "Horizontal"])
                .lens(Identity.map(
                    |row: &SettingsRow| match &row.value {
                        Value::Gradient(v) => match v {
                            Gradient::Transparent => 0,
                            Gradient::Plain(_) => 1,
                            Gradient::Vertical(_, _) => 2,
                            Gradient::Horizontal(_, _) => 3,
                        },
                        // TODO: What
                        _ => 4,
                    },
                    |row: &mut SettingsRow, value: usize| {
                        if let Value::Gradient(v) = &mut row.value {
                            let [a, b] = match *v {
                                Gradient::Transparent => [settings::Color::transparent(); 2],
                                Gradient::Plain(v) => [v; 2],
                                Gradient::Vertical(a, b) | Gradient::Horizontal(a, b) => [a, b],
                            };
                            *v = match value {
                                0 => Gradient::Transparent,
                                1 => Gradient::Plain(a),
                                2 => Gradient::Vertical(a, b),
                                3 => Gradient::Horizontal(a, b),
                                _ => return,
                            };
                        }
                    },
                ))
                .expand_width()
                .center(),
        )
        .with_child(ViewSwitcher::new(
            |row: &SettingsRow, _| match &row.value {
                Value::Gradient(v) => match v {
                    Gradient::Transparent => 0,
                    Gradient::Plain(_) => 1,
                    Gradient::Vertical(_, _) => 2,
                    Gradient::Horizontal(_, _) => 3,
                },
                // TODO: What
                _ => 4,
            },
            |_, row, _| match &row.value {
                Value::Gradient(v) => match v {
                    Gradient::Transparent => Box::new(Flex::column()),
                    Gradient::Plain(_) => Box::new(
                        color_editor()
                            .padding(Insets::new(0.0, GRID_BORDER, 0.0, 0.0))
                            .lens(Identity.map(
                                |row: &SettingsRow| {
                                    ColorData(match row.value {
                                        Value::Gradient(Gradient::Plain(v)) => v,
                                        // TODO: What
                                        _ => livesplit_core::settings::Color::transparent(),
                                    })
                                },
                                |row: &mut SettingsRow, color: ColorData| {
                                    if let Value::Gradient(Gradient::Plain(v)) = &mut row.value {
                                        *v = color.0;
                                    }
                                },
                            )),
                    ),
                    Gradient::Vertical(_, _) => {
                        Box::new(
                            Flex::row()
                                .with_flex_child(
                                    color_editor().lens(Identity.map(
                                        |row: &SettingsRow| {
                                            ColorData(match row.value {
                                                Value::Gradient(Gradient::Vertical(v, _)) => v,
                                                // TODO: What
                                                _ => livesplit_core::settings::Color::transparent(),
                                            })
                                        },
                                        |row: &mut SettingsRow, color: ColorData| {
                                            if let Value::Gradient(Gradient::Vertical(v, _)) =
                                                &mut row.value
                                            {
                                                *v = color.0;
                                            }
                                        },
                                    )),
                                    1.0,
                                )
                                .with_spacer(GRID_BORDER)
                                .with_flex_child(
                                    color_editor().lens(Identity.map(
                                        |row: &SettingsRow| {
                                            ColorData(match row.value {
                                                Value::Gradient(Gradient::Vertical(_, v)) => v,
                                                // TODO: What
                                                _ => livesplit_core::settings::Color::transparent(),
                                            })
                                        },
                                        |row: &mut SettingsRow, color: ColorData| {
                                            if let Value::Gradient(Gradient::Vertical(_, v)) =
                                                &mut row.value
                                            {
                                                *v = color.0;
                                            }
                                        },
                                    )),
                                    1.0,
                                )
                                .padding(Insets::new(0.0, GRID_BORDER, 0.0, 0.0)),
                        )
                    }
                    Gradient::Horizontal(_, _) => Box::new(
                        Flex::row()
                            .with_flex_child(
                                color_editor().lens(Identity.map(
                                    |row: &SettingsRow| {
                                        ColorData(match row.value {
                                            Value::Gradient(Gradient::Horizontal(v, _)) => v,
                                            // TODO: What
                                            _ => livesplit_core::settings::Color::transparent(),
                                        })
                                    },
                                    |row: &mut SettingsRow, color: ColorData| {
                                        if let Value::Gradient(Gradient::Horizontal(v, _)) =
                                            &mut row.value
                                        {
                                            *v = color.0;
                                        }
                                    },
                                )),
                                1.0,
                            )
                            .with_spacer(GRID_BORDER)
                            .with_flex_child(
                                color_editor().lens(Identity.map(
                                    |row: &SettingsRow| {
                                        ColorData(match row.value {
                                            Value::Gradient(Gradient::Horizontal(_, v)) => v,
                                            // TODO: What
                                            _ => livesplit_core::settings::Color::transparent(),
                                        })
                                    },
                                    |row: &mut SettingsRow, color: ColorData| {
                                        if let Value::Gradient(Gradient::Horizontal(_, v)) =
                                            &mut row.value
                                        {
                                            *v = color.0;
                                        }
                                    },
                                )),
                                1.0,
                            )
                            .padding(Insets::new(0.0, GRID_BORDER, 0.0, 0.0)),
                    ),
                },
                _ => Box::new(Flex::column()),
            },
        ))
}

fn delta_gradient() -> impl Widget<SettingsRow> {
    Flex::column()
        .with_child(
            combo_box::static_list(&[
                "Transparent",
                "Plain",
                "Vertical",
                "Horizontal",
                "Plain (Delta)",
                "Vertical (Delta)",
                "Horizontal (Delta)",
            ])
            .lens(Identity.map(
                |row: &SettingsRow| match &row.value {
                    Value::DeltaGradient(v) => match v {
                        DeltaGradient::Gradient(Gradient::Transparent) => 0,
                        DeltaGradient::Gradient(Gradient::Plain(_)) => 1,
                        DeltaGradient::Gradient(Gradient::Vertical(_, _)) => 2,
                        DeltaGradient::Gradient(Gradient::Horizontal(_, _)) => 3,
                        DeltaGradient::DeltaPlain => 4,
                        DeltaGradient::DeltaVertical => 5,
                        DeltaGradient::DeltaHorizontal => 6,
                    },
                    // TODO: What
                    _ => 7,
                },
                |row: &mut SettingsRow, value: usize| {
                    if let Value::DeltaGradient(v) = &mut row.value {
                        let [a, b] = match *v {
                            DeltaGradient::Gradient(Gradient::Plain(v)) => [v; 2],
                            DeltaGradient::Gradient(
                                Gradient::Vertical(a, b) | Gradient::Horizontal(a, b),
                            ) => [a, b],
                            _ => [settings::Color::transparent(); 2],
                        };
                        *v = match value {
                            0 => DeltaGradient::Gradient(Gradient::Transparent),
                            1 => DeltaGradient::Gradient(Gradient::Plain(a)),
                            2 => DeltaGradient::Gradient(Gradient::Vertical(a, b)),
                            3 => DeltaGradient::Gradient(Gradient::Horizontal(a, b)),
                            4 => DeltaGradient::DeltaPlain,
                            5 => DeltaGradient::DeltaVertical,
                            6 => DeltaGradient::DeltaHorizontal,
                            _ => return,
                        };
                    }
                },
            ))
            .expand_width()
            .center(),
        )
        .with_child(ViewSwitcher::new(
            |row: &SettingsRow, _| match &row.value {
                Value::DeltaGradient(v) => match v {
                    DeltaGradient::Gradient(Gradient::Transparent) => 0,
                    DeltaGradient::Gradient(Gradient::Plain(_)) => 1,
                    DeltaGradient::Gradient(Gradient::Vertical(_, _)) => 2,
                    DeltaGradient::Gradient(Gradient::Horizontal(_, _)) => 3,
                    DeltaGradient::DeltaPlain => 4,
                    DeltaGradient::DeltaVertical => 5,
                    DeltaGradient::DeltaHorizontal => 6,
                },
                // TODO: What
                _ => 7,
            },
            |_, row, _| match &row.value {
                Value::DeltaGradient(v) => match v {
                    DeltaGradient::Gradient(Gradient::Plain(_)) => Box::new(
                        color_editor()
                            .padding(Insets::new(0.0, GRID_BORDER, 0.0, 0.0))
                            .lens(Identity.map(
                                |row: &SettingsRow| {
                                    ColorData(match row.value {
                                        Value::DeltaGradient(DeltaGradient::Gradient(
                                            Gradient::Plain(v),
                                        )) => v,
                                        // TODO: What
                                        _ => livesplit_core::settings::Color::transparent(),
                                    })
                                },
                                |row: &mut SettingsRow, color: ColorData| {
                                    if let Value::DeltaGradient(DeltaGradient::Gradient(
                                        Gradient::Plain(v),
                                    )) = &mut row.value
                                    {
                                        *v = color.0;
                                    }
                                },
                            )),
                    ),
                    DeltaGradient::Gradient(Gradient::Vertical(_, _)) => {
                        Box::new(
                            Flex::row()
                                .with_flex_child(
                                    color_editor().lens(Identity.map(
                                        |row: &SettingsRow| {
                                            ColorData(match row.value {
                                                Value::DeltaGradient(DeltaGradient::Gradient(Gradient::Vertical(v, _))) => v,
                                                // TODO: What
                                                _ => livesplit_core::settings::Color::transparent(),
                                            })
                                        },
                                        |row: &mut SettingsRow, color: ColorData| {
                                            if let Value::DeltaGradient(DeltaGradient::Gradient(Gradient::Vertical(v, _))) =
                                                &mut row.value
                                            {
                                                *v = color.0;
                                            }
                                        },
                                    )),
                                    1.0,
                                )
                                .with_spacer(GRID_BORDER)
                                .with_flex_child(
                                    color_editor().lens(Identity.map(
                                        |row: &SettingsRow| {
                                            ColorData(match row.value {
                                                Value::DeltaGradient(DeltaGradient::Gradient(Gradient::Vertical(_, v))) => v,
                                                // TODO: What
                                                _ => livesplit_core::settings::Color::transparent(),
                                            })
                                        },
                                        |row: &mut SettingsRow, color: ColorData| {
                                            if let Value::DeltaGradient(DeltaGradient::Gradient(Gradient::Vertical(_, v))) =
                                                &mut row.value
                                            {
                                                *v = color.0;
                                            }
                                        },
                                    )),
                                    1.0,
                                )
                                .padding(Insets::new(0.0, GRID_BORDER, 0.0, 0.0)),
                        )
                    }
                    DeltaGradient::Gradient(Gradient::Horizontal(_, _)) => Box::new(
                        Flex::row()
                            .with_flex_child(
                                color_editor().lens(Identity.map(
                                    |row: &SettingsRow| {
                                        ColorData(match row.value {
                                            Value::DeltaGradient(DeltaGradient::Gradient(Gradient::Horizontal(v, _))) => v,
                                            // TODO: What
                                            _ => livesplit_core::settings::Color::transparent(),
                                        })
                                    },
                                    |row: &mut SettingsRow, color: ColorData| {
                                        if let Value::DeltaGradient(DeltaGradient::Gradient(Gradient::Horizontal(v, _))) =
                                            &mut row.value
                                        {
                                            *v = color.0;
                                        }
                                    },
                                )),
                                1.0,
                            )
                            .with_spacer(GRID_BORDER)
                            .with_flex_child(
                                color_editor().lens(Identity.map(
                                    |row: &SettingsRow| {
                                        ColorData(match row.value {
                                            Value::DeltaGradient(DeltaGradient::Gradient(Gradient::Horizontal(_, v))) => v,
                                            // TODO: What
                                            _ => livesplit_core::settings::Color::transparent(),
                                        })
                                    },
                                    |row: &mut SettingsRow, color: ColorData| {
                                        if let Value::DeltaGradient(DeltaGradient::Gradient(Gradient::Horizontal(_, v))) =
                                            &mut row.value
                                        {
                                            *v = color.0;
                                        }
                                    },
                                )),
                                1.0,
                            )
                            .padding(Insets::new(0.0, GRID_BORDER, 0.0, 0.0)),
                    ),
                    _ => Box::new(Flex::column()),
                },
                _ => Box::new(Flex::column()),
            },
        ))
}

fn list_gradient() -> impl Widget<SettingsRow> {
    Flex::column()
        .with_child(
            combo_box::static_list(&[
                "Transparent",
                "Plain",
                "Vertical",
                "Horizontal",
                "Alternating",
            ])
            .lens(Identity.map(
                |row: &SettingsRow| match &row.value {
                    Value::ListGradient(v) => match v {
                        ListGradient::Same(Gradient::Transparent) => 0,
                        ListGradient::Same(Gradient::Plain(_)) => 1,
                        ListGradient::Same(Gradient::Vertical(_, _)) => 2,
                        ListGradient::Same(Gradient::Horizontal(_, _)) => 3,
                        ListGradient::Alternating(_, _) => 4,
                    },
                    // TODO: What
                    _ => 5,
                },
                |row: &mut SettingsRow, value: usize| {
                    if let Value::ListGradient(v) = &mut row.value {
                        let [a, b] = match *v {
                            ListGradient::Same(Gradient::Transparent) => {
                                [settings::Color::transparent(); 2]
                            }
                            ListGradient::Same(Gradient::Plain(v)) => [v; 2],
                            ListGradient::Same(
                                Gradient::Vertical(a, b) | Gradient::Horizontal(a, b),
                            )
                            | ListGradient::Alternating(a, b) => [a, b],
                        };
                        *v = match value {
                            0 => ListGradient::Same(Gradient::Transparent),
                            1 => ListGradient::Same(Gradient::Plain(a)),
                            2 => ListGradient::Same(Gradient::Vertical(a, b)),
                            3 => ListGradient::Same(Gradient::Horizontal(a, b)),
                            4 => ListGradient::Alternating(a, b),
                            _ => return,
                        };
                    }
                },
            ))
            .expand_width()
            .center(),
        )
        .with_child(ViewSwitcher::new(
            |row: &SettingsRow, _| match &row.value {
                Value::ListGradient(v) => match v {
                    ListGradient::Same(Gradient::Transparent) => 0,
                    ListGradient::Same(Gradient::Plain(_)) => 1,
                    ListGradient::Same(Gradient::Vertical(_, _)) => 2,
                    ListGradient::Same(Gradient::Horizontal(_, _)) => 3,
                    ListGradient::Alternating(_, _) => 4,
                },
                // TODO: What
                _ => 5,
            },
            |_, row, _| match &row.value {
                Value::ListGradient(v) => match v {
                    ListGradient::Same(Gradient::Transparent) => Box::new(Flex::column()),
                    ListGradient::Same(Gradient::Plain(_)) => Box::new(
                        color_editor()
                            .padding(Insets::new(0.0, GRID_BORDER, 0.0, 0.0))
                            .lens(Identity.map(
                                |row: &SettingsRow| {
                                    ColorData(match row.value {
                                        Value::ListGradient(ListGradient::Same(
                                            Gradient::Plain(v),
                                        )) => v,
                                        // TODO: What
                                        _ => livesplit_core::settings::Color::transparent(),
                                    })
                                },
                                |row: &mut SettingsRow, color: ColorData| {
                                    if let Value::ListGradient(ListGradient::Same(
                                        Gradient::Plain(v),
                                    )) = &mut row.value
                                    {
                                        *v = color.0;
                                    }
                                },
                            )),
                    ),
                    ListGradient::Same(Gradient::Vertical(_, _)) => {
                        Box::new(
                            Flex::row()
                                .with_flex_child(
                                    color_editor().lens(Identity.map(
                                        |row: &SettingsRow| {
                                            ColorData(match row.value {
                                                Value::ListGradient(ListGradient::Same(Gradient::Vertical(v, _))) => v,
                                                // TODO: What
                                                _ => livesplit_core::settings::Color::transparent(),
                                            })
                                        },
                                        |row: &mut SettingsRow, color: ColorData| {
                                            if let Value::ListGradient(ListGradient::Same(Gradient::Vertical(v, _))) =
                                                &mut row.value
                                            {
                                                *v = color.0;
                                            }
                                        },
                                    )),
                                    1.0,
                                )
                                .with_spacer(GRID_BORDER)
                                .with_flex_child(
                                    color_editor().lens(Identity.map(
                                        |row: &SettingsRow| {
                                            ColorData(match row.value {
                                                Value::ListGradient(ListGradient::Same(Gradient::Vertical(_, v))) => v,
                                                // TODO: What
                                                _ => livesplit_core::settings::Color::transparent(),
                                            })
                                        },
                                        |row: &mut SettingsRow, color: ColorData| {
                                            if let Value::ListGradient(ListGradient::Same(Gradient::Vertical(_, v))) =
                                                &mut row.value
                                            {
                                                *v = color.0;
                                            }
                                        },
                                    )),
                                    1.0,
                                )
                                .padding(Insets::new(0.0, GRID_BORDER, 0.0, 0.0)),
                        )
                    }
                    ListGradient::Same(Gradient::Horizontal(_, _)) => Box::new(
                        Flex::row()
                            .with_flex_child(
                                color_editor().lens(Identity.map(
                                    |row: &SettingsRow| {
                                        ColorData(match row.value {
                                            Value::ListGradient(ListGradient::Same(Gradient::Horizontal(v, _))) => v,
                                            // TODO: What
                                            _ => livesplit_core::settings::Color::transparent(),
                                        })
                                    },
                                    |row: &mut SettingsRow, color: ColorData| {
                                        if let Value::ListGradient(ListGradient::Same(Gradient::Horizontal(v, _))) =
                                            &mut row.value
                                        {
                                            *v = color.0;
                                        }
                                    },
                                )),
                                1.0,
                            )
                            .with_spacer(GRID_BORDER)
                            .with_flex_child(
                                color_editor().lens(Identity.map(
                                    |row: &SettingsRow| {
                                        ColorData(match row.value {
                                            Value::ListGradient(ListGradient::Same(Gradient::Horizontal(_, v))) => v,
                                            // TODO: What
                                            _ => livesplit_core::settings::Color::transparent(),
                                        })
                                    },
                                    |row: &mut SettingsRow, color: ColorData| {
                                        if let Value::ListGradient(ListGradient::Same(Gradient::Horizontal(_, v))) =
                                            &mut row.value
                                        {
                                            *v = color.0;
                                        }
                                    },
                                )),
                                1.0,
                            )
                            .padding(Insets::new(0.0, GRID_BORDER, 0.0, 0.0)),
                    ),
                    ListGradient::Alternating(_, _) => Box::new(
                        Flex::row()
                            .with_flex_child(
                                color_editor().lens(Identity.map(
                                    |row: &SettingsRow| {
                                        ColorData(match row.value {
                                            Value::ListGradient(ListGradient::Alternating(v, _)) => v,
                                            // TODO: What
                                            _ => livesplit_core::settings::Color::transparent(),
                                        })
                                    },
                                    |row: &mut SettingsRow, color: ColorData| {
                                        if let Value::ListGradient(ListGradient::Alternating(v, _)) =
                                            &mut row.value
                                        {
                                            *v = color.0;
                                        }
                                    },
                                )),
                                1.0,
                            )
                            .with_spacer(GRID_BORDER)
                            .with_flex_child(
                                color_editor().lens(Identity.map(
                                    |row: &SettingsRow| {
                                        ColorData(match row.value {
                                            Value::ListGradient(ListGradient::Alternating(_, v)) => v,
                                            // TODO: What
                                            _ => livesplit_core::settings::Color::transparent(),
                                        })
                                    },
                                    |row: &mut SettingsRow, color: ColorData| {
                                        if let Value::ListGradient(ListGradient::Alternating(_, v)) =
                                            &mut row.value
                                        {
                                            *v = color.0;
                                        }
                                    },
                                )),
                                1.0,
                            )
                            .padding(Insets::new(0.0, GRID_BORDER, 0.0, 0.0)),
                    ),
                },
                _ => Box::new(Flex::column()),
            },
        ))
}

fn color() -> impl Widget<SettingsRow> {
    color_editor().lens(Identity.map(
        |row: &SettingsRow| {
            ColorData(match row.value {
                Value::Color(v) => v,
                // TODO: What
                _ => livesplit_core::settings::Color::transparent(),
            })
        },
        |row: &mut SettingsRow, color: ColorData| {
            if let Value::Color(v) = &mut row.value {
                *v = color.0;
            }
        },
    ))
}

fn layout_background() -> impl Widget<SettingsRow> {
    color_editor().lens(Identity.map(
        |row: &SettingsRow| {
            ColorData(match &row.value {
                Value::LayoutBackground(v) => color_from_layout_background(v),
                Value::Color(v) => *v,
                // TODO: What
                _ => livesplit_core::settings::Color::transparent(),
            })
        },
        |row: &mut SettingsRow, color: ColorData| match &mut row.value {
            Value::LayoutBackground(v) => {
                *v = LayoutBackground::Gradient(Gradient::Plain(color.0));
            }
            Value::Color(v) => {
                *v = color.0;
            }
            _ => (),
        },
    ))
}

#[derive(Copy, Clone)]
struct ColorData(livesplit_core::settings::Color);

impl Data for ColorData {
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

fn color_editor() -> impl Widget<ColorData> {
    crate::map_scope::map(
        crate::color_button::widget().expand_width(),
        |rgba: &ColorData| {
            let [h, s, v, a] = rgba.0.to_hsva().map(Into::into);
            crate::color_button::ColorState::hsva(h, s, v, a)
        },
        |hsva| {
            ColorData(livesplit_core::settings::Color::hsva(
                hsva.hue as _,
                hsva.saturation as _,
                hsva.value as _,
                hsva.alpha as _,
            ))
        },
    )
}

fn font() -> impl Widget<SettingsRow> {
    const LABELS_WIDTH: f64 = 60.0;

    Flex::column()
        .with_child(
            Switch::new()
                .lens(Identity.map(
                    |row: &SettingsRow| match &row.value {
                        Value::Font(v) => v.is_some(),
                        // TODO: What
                        _ => false,
                    },
                    |row: &mut SettingsRow, value: bool| {
                        if let Value::Font(v) = &mut row.value {
                            if v.is_some() != value {
                                *v = value.then(Font::default);
                            }
                        }
                    },
                ))
                .env_scope(|env, _| switch_style(env))
                .center(),
        )
        .with_child(ViewSwitcher::new(
            |row: &SettingsRow, _| matches!(row.value, Value::Font(Some(_))),
            |_, row, _| match row.value {
                Value::Font(Some(_)) => Box::new(
                    Flex::column()
                        .with_default_spacer()
                        .with_child(
                            Flex::row()
                                .with_child(Label::new("Name").fix_width(LABELS_WIDTH))
                                .with_flex_child(
                                    combo_box::custom_user_string(FONT_FAMILIES.clone())
                                        .lens(Identity.map(
                                            |row: &SettingsRow| match &row.value {
                                                Value::Font(Some(v)) => v.family.clone(),
                                                // TODO: What
                                                _ => String::new(),
                                            },
                                            |row: &mut SettingsRow, value: String| {
                                                if let Value::Font(Some(v)) = &mut row.value {
                                                    v.family = value;
                                                }
                                            },
                                        ))
                                        .expand_width(),
                                    1.0,
                                ),
                        )
                        .with_default_spacer()
                        .with_child(
                            Flex::row()
                                .with_child(Label::new("Style").fix_width(LABELS_WIDTH))
                                .with_flex_child(
                                    combo_box::static_list(&["Normal", "Italic", "Oblique"])
                                        .lens(Identity.map(
                                            |row: &SettingsRow| match &row.value {
                                                Value::Font(Some(v)) => match v.style {
                                                    FontStyle::Normal => 0,
                                                    FontStyle::Italic => 1,
                                                    FontStyle::Oblique => 2,
                                                },
                                                // TODO: What
                                                _ => 3,
                                            },
                                            |row: &mut SettingsRow, value: usize| {
                                                if let Value::Font(Some(v)) = &mut row.value {
                                                    v.style = match value {
                                                        0 => FontStyle::Normal,
                                                        1 => FontStyle::Italic,
                                                        2 => FontStyle::Oblique,
                                                        _ => return,
                                                    };
                                                }
                                            },
                                        ))
                                        .expand_width(),
                                    1.0,
                                ),
                        )
                        .with_default_spacer()
                        .with_child(
                            Flex::row()
                                .with_child(Label::new("Weight").fix_width(LABELS_WIDTH))
                                .with_flex_child(
                                    combo_box::static_list(&[
                                        "Thin",
                                        "Extra Light",
                                        "Light",
                                        "Semi Light",
                                        "Normal",
                                        "Medium",
                                        "Semi Bold",
                                        "Bold",
                                        "Extra Bold",
                                        "Black",
                                        "Extra Black",
                                    ])
                                    .lens(Identity.map(
                                        |row: &SettingsRow| match &row.value {
                                            Value::Font(Some(v)) => match v.weight {
                                                FontWeight::Thin => 0,
                                                FontWeight::ExtraLight => 1,
                                                FontWeight::Light => 2,
                                                FontWeight::SemiLight => 3,
                                                FontWeight::Normal => 4,
                                                FontWeight::Medium => 5,
                                                FontWeight::SemiBold => 6,
                                                FontWeight::Bold => 7,
                                                FontWeight::ExtraBold => 8,
                                                FontWeight::Black => 9,
                                                FontWeight::ExtraBlack => 10,
                                            },
                                            // TODO: What
                                            _ => 11,
                                        },
                                        |row: &mut SettingsRow, value: usize| {
                                            if let Value::Font(Some(v)) = &mut row.value {
                                                v.weight = match value {
                                                    0 => FontWeight::Thin,
                                                    1 => FontWeight::ExtraLight,
                                                    2 => FontWeight::Light,
                                                    3 => FontWeight::SemiLight,
                                                    4 => FontWeight::Normal,
                                                    5 => FontWeight::Medium,
                                                    6 => FontWeight::SemiBold,
                                                    7 => FontWeight::Bold,
                                                    8 => FontWeight::ExtraBold,
                                                    9 => FontWeight::Black,
                                                    10 => FontWeight::ExtraBlack,
                                                    _ => return,
                                                };
                                            }
                                        },
                                    ))
                                    .expand_width(),
                                    1.0,
                                ),
                        )
                        .with_default_spacer()
                        .with_child(
                            Flex::row()
                                .with_child(Label::new("Stretch").fix_width(LABELS_WIDTH))
                                .with_flex_child(
                                    combo_box::static_list(&[
                                        "Ultra Condensed",
                                        "Extra Condensed",
                                        "Condensed",
                                        "Semi Condensed",
                                        "Normal",
                                        "Semi Expanded",
                                        "Expanded",
                                        "Extra Expanded",
                                        "Ultra Expanded",
                                    ])
                                    .lens(Identity.map(
                                        |row: &SettingsRow| match &row.value {
                                            Value::Font(Some(v)) => match v.stretch {
                                                FontStretch::UltraCondensed => 0,
                                                FontStretch::ExtraCondensed => 1,
                                                FontStretch::Condensed => 2,
                                                FontStretch::SemiCondensed => 3,
                                                FontStretch::Normal => 4,
                                                FontStretch::SemiExpanded => 5,
                                                FontStretch::Expanded => 6,
                                                FontStretch::ExtraExpanded => 7,
                                                FontStretch::UltraExpanded => 8,
                                            },
                                            // TODO: What
                                            _ => 9,
                                        },
                                        |row: &mut SettingsRow, value: usize| {
                                            if let Value::Font(Some(v)) = &mut row.value {
                                                v.stretch = match value {
                                                    0 => FontStretch::UltraCondensed,
                                                    1 => FontStretch::ExtraCondensed,
                                                    2 => FontStretch::Condensed,
                                                    3 => FontStretch::SemiCondensed,
                                                    4 => FontStretch::Normal,
                                                    5 => FontStretch::SemiExpanded,
                                                    6 => FontStretch::Expanded,
                                                    7 => FontStretch::ExtraExpanded,
                                                    8 => FontStretch::UltraExpanded,
                                                    _ => return,
                                                };
                                            }
                                        },
                                    ))
                                    .expand_width(),
                                    1.0,
                                ),
                        ),
                ),
                _ => Box::new(Flex::row()),
            },
        ))
}

fn digits_format() -> impl Widget<SettingsRow> {
    combo_box::static_list(&["1", "01", "0:01", "00:01", "0:00:01", "00:00:01"])
        .lens(Identity.map(
            |row: &SettingsRow| match &row.value {
                Value::DigitsFormat(v) => match v {
                    DigitsFormat::SingleDigitSeconds => 0,
                    DigitsFormat::DoubleDigitSeconds => 1,
                    DigitsFormat::SingleDigitMinutes => 2,
                    DigitsFormat::DoubleDigitMinutes => 3,
                    DigitsFormat::SingleDigitHours => 4,
                    DigitsFormat::DoubleDigitHours => 5,
                },
                // TODO: What
                _ => 6,
            },
            |row: &mut SettingsRow, value: usize| {
                if let Value::DigitsFormat(v) = &mut row.value {
                    *v = match value {
                        0 => DigitsFormat::SingleDigitSeconds,
                        1 => DigitsFormat::DoubleDigitSeconds,
                        2 => DigitsFormat::SingleDigitMinutes,
                        3 => DigitsFormat::DoubleDigitMinutes,
                        4 => DigitsFormat::SingleDigitHours,
                        5 => DigitsFormat::DoubleDigitHours,
                        _ => return,
                    };
                }
            },
        ))
        .expand_width()
}

fn accuracy() -> impl Widget<SettingsRow> {
    combo_box::static_list(&["Seconds", "Tenths", "Hundredths", "Milliseconds"])
        .lens(Identity.map(
            |row: &SettingsRow| match &row.value {
                Value::Accuracy(v) => match v {
                    Accuracy::Seconds => 0,
                    Accuracy::Tenths => 1,
                    Accuracy::Hundredths => 2,
                    Accuracy::Milliseconds => 3,
                },
                // TODO: What
                _ => 4,
            },
            |row: &mut SettingsRow, value: usize| {
                if let Value::Accuracy(v) = &mut row.value {
                    *v = match value {
                        0 => Accuracy::Seconds,
                        1 => Accuracy::Tenths,
                        2 => Accuracy::Hundredths,
                        3 => Accuracy::Milliseconds,
                        _ => return,
                    };
                }
            },
        ))
        .expand_width()
}

fn optional_color() -> impl Widget<SettingsRow> {
    Flex::row()
        .with_child(
            Switch::new()
                .lens(Identity.map(
                    |row: &SettingsRow| match &row.value {
                        Value::OptionalColor(v) => v.is_some(),
                        // TODO: What
                        _ => false,
                    },
                    |row: &mut SettingsRow, value: bool| {
                        if let Value::OptionalColor(v) = &mut row.value {
                            if v.is_some() != value {
                                *v = value.then(settings::Color::white);
                            }
                        }
                    },
                ))
                .env_scope(|env, _| switch_style(env)),
        )
        .with_spacer(GRID_BORDER)
        .with_flex_child(
            ViewSwitcher::new(
                |row: &SettingsRow, _| matches!(row.value, Value::OptionalColor(Some(_))),
                |_, row, _| match row.value {
                    Value::OptionalColor(Some(_)) => {
                        Box::new(
                            color_editor()
                                .lens(Identity.map(
                                    |row: &SettingsRow| {
                                        ColorData(match row.value {
                                            Value::OptionalColor(Some(v)) => v,
                                            // TODO: What
                                            _ => livesplit_core::settings::Color::transparent(),
                                        })
                                    },
                                    |row: &mut SettingsRow, color: ColorData| {
                                        if let Value::OptionalColor(Some(v)) = &mut row.value {
                                            *v = color.0;
                                        }
                                    },
                                ))
                                .expand_width(),
                        )
                    }
                    _ => Box::new(Flex::row()),
                },
            ),
            1.0,
        )
}

fn optional_string() -> impl Widget<SettingsRow> {
    Flex::row()
        .with_child(
            Switch::new()
                .lens(Identity.map(
                    |row: &SettingsRow| match &row.value {
                        Value::OptionalString(v) => v.is_some(),
                        // TODO: What
                        _ => false,
                    },
                    |row: &mut SettingsRow, value: bool| {
                        if let Value::OptionalString(v) = &mut row.value {
                            if v.is_some() != value {
                                *v = value.then(String::new);
                            }
                        }
                    },
                ))
                .env_scope(|env, _| switch_style(env)),
        )
        .with_spacer(GRID_BORDER)
        .with_flex_child(
            ViewSwitcher::new(
                |row: &SettingsRow, _| matches!(row.value, Value::OptionalString(Some(_))),
                |_, row, _| match row.value {
                    Value::OptionalString(Some(_)) => {
                        Box::new(
                            TextBox::new()
                                .lens(Identity.map(
                                    |row: &SettingsRow| match &row.value {
                                        Value::OptionalString(Some(v)) => v.clone(),
                                        // TODO: What
                                        _ => String::new(),
                                    },
                                    |row: &mut SettingsRow, value: String| {
                                        if let Value::OptionalString(Some(v)) = &mut row.value {
                                            *v = value;
                                        }
                                    },
                                ))
                                .expand_width(),
                        )
                    }
                    _ => Box::new(Flex::row()),
                },
            ),
            1.0,
        )
}

fn optional_timing_method() -> impl Widget<SettingsRow> {
    Flex::row()
        .with_child(
            Switch::new()
                .lens(Identity.map(
                    |row: &SettingsRow| match &row.value {
                        Value::OptionalTimingMethod(v) => v.is_some(),
                        // TODO: What
                        _ => false,
                    },
                    |row: &mut SettingsRow, value: bool| {
                        if let Value::OptionalTimingMethod(v) = &mut row.value {
                            if v.is_some() != value {
                                *v = value.then(|| TimingMethod::RealTime);
                            }
                        }
                    },
                ))
                .env_scope(|env, _| switch_style(env)),
        )
        .with_spacer(GRID_BORDER)
        .with_flex_child(
            ViewSwitcher::new(
                |row: &SettingsRow, _| matches!(row.value, Value::OptionalTimingMethod(Some(_))),
                |_, row, _| match row.value {
                    Value::OptionalTimingMethod(Some(_)) => {
                        Box::new(
                            combo_box::static_list(&["Real Time", "Game Time"])
                                .lens(Identity.map(
                                    |row: &SettingsRow| match &row.value {
                                        Value::OptionalTimingMethod(Some(v)) => match v {
                                            TimingMethod::RealTime => 0,
                                            TimingMethod::GameTime => 1,
                                        },
                                        // TODO: What
                                        _ => 2,
                                    },
                                    |row: &mut SettingsRow, value: usize| {
                                        if let Value::OptionalTimingMethod(Some(v)) = &mut row.value
                                        {
                                            *v = match value {
                                                0 => TimingMethod::RealTime,
                                                1 => TimingMethod::GameTime,
                                                _ => return,
                                            };
                                        }
                                    },
                                ))
                                .expand_width()
                                .center(),
                        )
                    }
                    _ => Box::new(Flex::row()),
                },
            ),
            1.0,
        )
}

fn color_from_layout_background(
    lb: &LayoutBackground<settings::ImageId>,
) -> livesplit_core::settings::Color {
    match lb {
        LayoutBackground::Gradient(g) => color_from_gradiant(g),
        // TODO: What
        _ => livesplit_core::settings::Color::transparent(),
    }
}

fn color_from_gradiant(g: &Gradient) -> livesplit_core::settings::Color {
    match g {
        Gradient::Transparent => livesplit_core::settings::Color::transparent(),
        Gradient::Plain(c) => *c,
        Gradient::Horizontal(c, _) => *c,
        Gradient::Vertical(c, _) => *c,
    }
}
