//! Data-defined basic presets. PlotType remains the compatible file/UI identifier.
use crate::{
    coord::Orientation,
    csv,
    mark::{Kind, Path},
    position::Position,
    Book, PlotType,
};

#[derive(Clone, Copy)]
pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: PlotType,
}

#[derive(Clone, Debug)]
pub(crate) struct Recipe {
    pub marks: Vec<Kind>,
    pub position: Position,
    pub orientation: Orientation,
    pub path: Path,
}
impl Recipe {
    pub fn is_bar(&self) -> bool {
        self.marks.contains(&Kind::Bar)
    }
    pub fn has_line(&self) -> bool {
        self.marks.contains(&Kind::Line) || self.marks.contains(&Kind::Area)
    }
    pub fn has_symbol(&self) -> bool {
        self.marks.contains(&Kind::Symbol)
    }
}

fn entries() -> &'static [(Preset, Recipe)] {
    static ENTRIES: std::sync::OnceLock<Vec<(Preset, Recipe)>> = std::sync::OnceLock::new();
    ENTRIES.get_or_init(|| {
        include_str!("../presets/basic.csv")
            .lines()
            .skip(1)
            .map(|line| {
                let mut columns = line.split(',');
                let id = columns.next().unwrap();
                let name = columns.next().unwrap();
                let kind = match columns.next().unwrap() {
                    "Line" => PlotType::Line,
                    "Scatter" => PlotType::Scatter,
                    "LineSymbol" => PlotType::LineSymbol,
                    "Bar" => PlotType::Bar,
                    "StackedBar" => PlotType::StackedBar,
                    "PercentStackedBar" => PlotType::PercentStackedBar,
                    "FloatingBar" => PlotType::FloatingBar,
                    "HorizontalBar" => PlotType::HorizontalBar,
                    "Area" => PlotType::Area,
                    "Spline" => PlotType::Spline,
                    "SplineSymbol" => PlotType::SplineSymbol,
                    "VerticalDrop" => PlotType::VerticalDrop,
                    "StepHorizontal" => PlotType::StepHorizontal,
                    "StepVertical" => PlotType::StepVertical,
                    "Histogram" => PlotType::Histogram,
                    _ => unreachable!("built-in preset type"),
                };
                let marks = columns
                    .next()
                    .unwrap()
                    .split('+')
                    .map(|name| match name {
                        "Line" => Kind::Line,
                        "Symbol" => Kind::Symbol,
                        "Bar" => Kind::Bar,
                        "Area" => Kind::Area,
                        "Drop" => Kind::Drop,
                        _ => unreachable!("built-in mark"),
                    })
                    .collect();
                let position = match columns.next().unwrap() {
                    "Identity" => Position::Identity,
                    "Dodge" => Position::Dodge,
                    "Stack" => Position::Stack,
                    "PercentStack" => Position::PercentStack,
                    "Interval" => Position::Interval,
                    "Bin" => Position::Bin,
                    _ => unreachable!("built-in position"),
                };
                let orientation = match columns.next().unwrap() {
                    "Vertical" => Orientation::Vertical,
                    "Horizontal" => Orientation::Horizontal,
                    _ => unreachable!("built-in orientation"),
                };
                let path = match columns.next().unwrap() {
                    "Straight" => Path::Straight,
                    "Natural" => Path::Natural,
                    "StepHorizontal" => Path::StepHorizontal,
                    "StepVertical" => Path::StepVertical,
                    _ => unreachable!("built-in path"),
                };
                (
                    Preset { id, name, kind },
                    Recipe {
                        marks,
                        position,
                        orientation,
                        path,
                    },
                )
            })
            .collect()
    })
}

pub fn all() -> Vec<Preset> {
    entries().iter().map(|(preset, _)| *preset).collect()
}
pub(crate) fn recipe(kind: PlotType) -> &'static Recipe {
    &entries()
        .iter()
        .find(|(preset, _)| preset.kind == kind)
        .expect("preset for every persisted PlotType")
        .1
}

pub fn example() -> Book {
    csv::book_from_table(1, "图库示例", csv::parse_table(include_str!("../presets/example.csv")))
}
