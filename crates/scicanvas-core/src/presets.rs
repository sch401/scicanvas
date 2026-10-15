//! 数据文件定义的基础图库。新图型仅在渲染部件可用后加入。
use crate::{csv, Book, PlotType};

pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: PlotType,
}

pub fn all() -> Vec<Preset> {
    include_str!("../presets/basic.csv")
        .lines()
        .skip(1)
        .map(|line| {
            let mut columns = line.split(',');
            let id = columns.next().unwrap();
            let name = columns.next().unwrap();
            let kind = match columns.next().unwrap().trim() {
                "Line" => PlotType::Line,
                "Scatter" => PlotType::Scatter,
                "LineSymbol" => PlotType::LineSymbol,
                "Bar" => PlotType::Bar,
                "StackedBar" => PlotType::StackedBar,
                "PercentStackedBar" => PlotType::PercentStackedBar,
                "FloatingBar" => PlotType::FloatingBar,
                "StepHorizontal" => PlotType::StepHorizontal,
                "StepVertical" => PlotType::StepVertical,
                _ => unreachable!("内置预设使用已实现的图元"),
            };
            Preset { id, name, kind }
        })
        .collect()
}

pub fn example() -> Book {
    csv::book_from_table(1, "图库示例", csv::parse_table(include_str!("../presets/example.csv")))
}
