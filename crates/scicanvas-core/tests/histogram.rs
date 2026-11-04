#![cfg(feature = "serde")]
//! 直方图：共同区间计数、轴标题、柱子相连、项目保存（v25）。
use scicanvas_core::{
    project_file,
    scene::{self, Prim},
    stats, Book, Column, PlotType, Project, Role,
};

const A: [f64; 10] = [1.2, 2.5, 2.7, 3.1, 3.3, 3.8, 4.4, 4.9, 5.6, 7.0];
const B: [f64; 6] = [0.5, 2.2, 3.4, 3.6, 4.1, 6.3];

/// 只有两个 Y 列（不需要 X 列）的工作簿，用全部 Y 列画直方图。
fn project() -> Project {
    let mut p = Project::default();
    let id = p.alloc_id();
    let mut book = Book::new(id, "粒径");
    for (key, name, values) in [("A", "样品甲", &A[..]), ("B", "样品乙", &B[..])] {
        let mut column = Column::new(key, Role::Y);
        column.long_name = name.into();
        column.units = "µm".into();
        column.data = values.iter().map(ToString::to_string).collect::<Vec<_>>().into();
        book.columns.push(column);
    }
    p.books.push(book);
    p.plot_from_selection(id, &[], PlotType::Histogram).unwrap();
    p
}

#[test]
fn histogram_counts_every_y_column_on_shared_sturges_bins() {
    let p = project();
    let (_, plots) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
    assert_eq!(plots.len(), 2);
    // 共同范围 0.5–7.0；最长的一列 10 个值，Sturges 规则 ⌈log₂10⌉ + 1 = 5 个区间。
    assert_eq!(stats::sturges_bins(10), 5);
    for (plot, values) in plots.iter().zip([&A[..], &B[..]]) {
        let expected: Vec<[f64; 2]> = stats::histogram(values, 5, Some((0.5, 7.0)))
            .unwrap()
            .iter()
            .map(|b| [(b.lo + b.hi) / 2.0, b.count as f64])
            .collect();
        assert_eq!(plot.series.points, expected);
        assert!(plot.series.bases.iter().all(|b| *b == 0.0));
    }
}

#[test]
fn histogram_axis_titles_describe_values_and_counts() {
    let p = project();
    let layer = p.graphs[0].first_layer();
    assert_eq!(layer.x_axis_title_in(Some(&p.books[0]), &|_| None), "样品甲 (µm)");
    assert_eq!(layer.y_axis_title_in(Some(&p.books[0]), &|_| None), "计数");
}

#[test]
fn histogram_bars_fill_each_bin_without_gaps() {
    let mut p = project();
    // 不画图例，裁剪在绘图区内的矩形只有直方图的柱子。
    p.graphs[0].first_layer_mut().legend = false;
    let sc = scene::build_page(&p.graphs[0], &p, 800.0, 600.0, None);
    let bars: Vec<[f64; 2]> = sc
        .items
        .iter()
        .filter(|item| item.clip.is_some())
        .filter_map(|item| match item.prim {
            Prim::Rect { x, w, .. } => Some([x, x + w]),
            _ => None,
        })
        .collect();
    // 两列各 5 个区间（计数为 0 的区间也画一个高度为 0 的柱子）。
    assert_eq!(bars.len(), 10);
    for pair in bars[..5].windows(2) {
        assert!((pair[0][1] - pair[1][0]).abs() < 1e-6, "相邻区间的柱子应相接：{pair:?}");
    }
}

#[test]
fn histogram_is_saved_in_v25() {
    let p = project();
    let json = project_file::to_json(&p).unwrap();
    assert!(json.contains("Histogram"));
    let loaded = project_file::from_json(&json).unwrap();
    assert_eq!(loaded, p);
    assert_eq!(loaded.version, scicanvas_core::model::PROJECT_VERSION);
}
