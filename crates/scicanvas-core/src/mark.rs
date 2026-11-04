//! Plot marks compose data series into the shared scene primitives.
use crate::coord::{AxisSet, Coord, YAxis};
use crate::model::{Book, PlotType, SeriesStyle};
use crate::page::Layer;
use crate::plotspec::BookLookup;
use crate::scene::{Item, Prim};
use crate::ticks::Axis;

pub(crate) mod annotations;
pub(crate) mod bars;
pub(crate) mod box_plot;
pub(crate) mod path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Line,
    Symbol,
    Bar,
    Box,
    Area,
    Drop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Path {
    Straight,
    Natural,
    StepHorizontal,
    StepVertical,
}

pub(crate) struct Context<'g, 'book, 'lookup, 'axis> {
    pub layer: &'g Layer,
    pub book: Option<&'book Book>,
    pub others: BookLookup<'book, 'lookup>,
    pub axes: &'axis AxisSet,
    pub rect: [f64; 4],
    pub horizontal: bool,
    pub dx: f64,
}

impl Context<'_, '_, '_, '_> {
    pub fn px(&self, x: f64) -> f64 {
        self.axes
            .cartesian(self.rect, YAxis::Left)
            .project([x, self.axes.left_y.lo])[0]
    }
    pub fn py(&self, y: f64, axis: YAxis) -> f64 {
        self.axes.cartesian(self.rect, axis).project([self.axes.x.lo, y])[1]
    }
    pub fn y_axis(&self, axis: YAxis) -> &Axis {
        self.axes.y(axis)
    }

    pub fn inside_x(&self, x: f64) -> bool {
        x >= self.rect[0] - 0.5 && x <= self.rect[0] + self.rect[2] + 0.5
    }

    pub fn inside_y(&self, y: f64) -> bool {
        y >= self.rect[1] - 0.5 && y <= self.rect[1] + self.rect[3] + 0.5
    }

    pub fn offset(&self, style: &SeriesStyle) -> f64 {
        if style.kind.is_bar() {
            crate::presets::recipe(style.kind)
                .position
                .offset(self.dx, style.bar_index, style.bar_count)
        } else {
            0.0
        }
    }

    pub fn push(&self, items: &mut Vec<Item>, prim: Prim, clip: bool) {
        items.push(Item {
            prim,
            clip: clip.then_some(self.rect),
        });
    }
}

pub(crate) fn draw(ctx: &Context<'_, '_, '_, '_>, plots: &[crate::page::PreparedPlot], items: &mut Vec<Item>) {
    let mut symbols = Vec::new();
    for plot in plots {
        if box_plot::draw(ctx, plot, items) {
            continue;
        }
        let (series, style, paths) = (&plot.series, &plot.style, &plot.paths);
        if bars::draw(ctx, series, style, items) {
            continue;
        }
        if let Some(symbol) = path::draw(ctx, series, style, plot.plot.style.as_ref(), paths, items) {
            symbols.push(symbol);
        }
    }
    if ctx.layer.error_bars.visible {
        annotations::errors(ctx, plots, items);
    }
    for symbol in symbols {
        ctx.push(items, symbol, false);
    }
    if ctx.layer.labels.visible {
        annotations::labels(ctx, plots, items);
    }
}

pub(crate) fn contains(kind: PlotType, mark: Kind) -> bool {
    crate::presets::recipe(kind).marks.contains(&mark)
}
