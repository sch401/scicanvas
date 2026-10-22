//! Plot marks compose data series into the shared scene primitives.
use crate::coord::{Cartesian, Coord};
use crate::model::{Book, PlotType, Series, SeriesStyle};
use crate::page::Layer;
use crate::plotspec::BookLookup;
use crate::scene::{Item, Prim};
use crate::ticks::Axis;

pub(crate) mod annotations;
pub(crate) mod bars;
pub(crate) mod path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Line,
    Symbol,
    Bar,
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
    pub xa: &'axis Axis,
    pub ya: &'axis Axis,
    pub ya2: &'axis Axis,
    pub rect: [f64; 4],
    pub horizontal: bool,
    pub dx: f64,
}

impl Context<'_, '_, '_, '_> {
    pub fn px(&self, x: f64) -> f64 {
        let coord = Cartesian {
            x: self.xa,
            y: self.ya,
            rect: self.rect,
        };
        coord.project([x, self.ya.lo])[0]
    }

    pub fn py(&self, y: f64, right: bool) -> f64 {
        let coord = Cartesian {
            x: self.xa,
            y: if right { self.ya2 } else { self.ya },
            rect: self.rect,
        };
        coord.project([self.xa.lo, y])[1]
    }

    pub fn y_axis(&self, right: bool) -> &Axis {
        if right {
            self.ya2
        } else {
            self.ya
        }
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

pub(crate) fn draw(
    ctx: &Context<'_, '_, '_, '_>,
    series: &[Series],
    styles: &[SeriesStyle],
    splines: &[Vec<Vec<[f64; 2]>>],
    items: &mut Vec<Item>,
) {
    let mut symbols = Vec::new();
    for ((series, style), paths) in series.iter().zip(styles).zip(splines) {
        if bars::draw(ctx, series, style, items) {
            continue;
        }
        if let Some(symbol) = path::draw(ctx, series, style, paths, items) {
            symbols.push(symbol);
        }
    }
    if ctx.layer.error_bars.visible {
        annotations::errors(ctx, series, styles, items);
    }
    for symbol in symbols {
        ctx.push(items, symbol, false);
    }
    if ctx.layer.labels.visible {
        annotations::labels(ctx, series, styles, items);
    }
}

pub(crate) fn contains(kind: PlotType, mark: Kind) -> bool {
    crate::presets::recipe(kind).marks.contains(&mark)
}
