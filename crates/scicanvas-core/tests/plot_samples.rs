//! 每种图型一个示例项目（`samples/*.scv`）：一个工作簿和一张用全部 Y 列绘制的图。
//! 注释行说明每列该放什么数据。修改数据或绘图后运行
//! `cargo test -p scicanvas-core --features serde --test plot_samples -- --ignored`，
//! 解压检查新生成的 CSV 和 SVG 后再提交。
#![cfg(feature = "serde")]

use std::{fs, path::PathBuf};

use scicanvas_core::{fmt_num, page::Layer, project_file, Book, Column, PlotType, Project, Role};

fn values(xs: impl IntoIterator<Item = f64>, decimals: i32) -> Vec<String> {
    let p = 10f64.powi(decimals);
    xs.into_iter().map(|x| fmt_num((x * p).round() / p)).collect()
}

fn column(key: &str, role: Role, name: &str, units: &str, comment: &str, data: Vec<String>) -> Column {
    let mut c = Column::new(key, role);
    c.long_name = name.into();
    c.units = units.into();
    c.comment = comment.into();
    c.data = data;
    c
}

/// 列依次为 A、B、C……；每项为（角色、长名称、单位、注释、数据）。
type Spec<'a> = (Role, &'a str, &'a str, &'a str, Vec<String>);

fn sample(kind: PlotType, book_name: &str, columns: Vec<Spec>) -> Project {
    let mut p = Project::default();
    let id = p.alloc_id();
    let mut book = Book::new(id, book_name);
    for (i, (role, name, units, comment, data)) in columns.into_iter().enumerate() {
        book.columns.push(column(
            &scicanvas_core::column_letter(i),
            role,
            name,
            units,
            comment,
            data,
        ));
    }
    p.books.push(book);
    let (graph, _) = p.plot_from_selection(id, &[], kind).unwrap();
    p.rename_project_item(graph, kind.label()).unwrap();
    p
}

/// 调整图层：轴标题概括多列数据；柱子占满坐标框时手动留出放图例的空白，
/// 阶梯图留出顶部余量，最高一段不和坐标框重合。
fn layer(mut p: Project, f: impl FnOnce(&mut Layer)) -> Project {
    f(p.graphs[0].first_layer_mut());
    p
}

fn range(layer: &mut Layer, x: Option<(f64, f64)>, y: Option<(f64, f64)>) {
    if let Some((min, max)) = x {
        (layer.x_auto, layer.x_min, layer.x_max) = (false, min, max);
    }
    if let Some((min, max)) = y {
        (layer.y_auto, layer.y_min, layer.y_max) = (false, min, max);
    }
}

fn samples() -> Vec<(String, Project)> {
    use PlotType::*;
    use Role::{YError, X, Y};
    let t: Vec<f64> = (0..=12).map(|i| f64::from(i) * 5.0).collect();
    let years = values((2020..=2025).map(f64::from), 0);
    let months = values((1..=12).map(f64::from), 0);
    let numbers = |n: i32| values((1..=n).map(f64::from), 0);
    let list = |xs: &[f64]| values(xs.iter().copied(), 3);
    let cases = vec![
        sample(
            Line,
            "反应釜升温",
            vec![
                (X, "时间", "min", "X 列：自变量，按从小到大排列", values(t.clone(), 0)),
                (
                    Y,
                    "釜内温度",
                    "°C",
                    "Y 列：每个 Y 列画成一条折线",
                    values(t.iter().map(|t| 25.0 + 55.0 * (1.0 - (-t / 15.0).exp())), 1),
                ),
                (
                    Y,
                    "夹套温度",
                    "°C",
                    "Y 列：第二条曲线",
                    values(t.iter().map(|t| 25.0 + 60.0 * (1.0 - (-t / 8.0).exp())), 1),
                ),
            ],
        ),
        sample(
            Scatter,
            "标准曲线",
            vec![
                (
                    X,
                    "浓度",
                    "mg/L",
                    "X 列：标准溶液浓度",
                    values((0..=10).map(|i| f64::from(i) * 2.0), 0),
                ),
                (
                    Y,
                    "吸光度",
                    "",
                    "Y 列：测量值，每行画一个点",
                    list(&[
                        0.004, 0.101, 0.196, 0.291, 0.389, 0.486, 0.577, 0.676, 0.772, 0.869, 0.962,
                    ]),
                ),
            ],
        ),
        sample(
            LineSymbol,
            "转化率",
            vec![
                (X, "时间", "h", "X 列", list(&[0.0, 1.0, 2.0, 4.0, 6.0, 8.0, 12.0])),
                (
                    Y,
                    "转化率",
                    "%",
                    "Y 列：三次平行实验的平均值",
                    list(&[0.0, 21.4, 38.2, 61.5, 75.3, 84.0, 92.6]),
                ),
                (
                    YError,
                    "标准差",
                    "%",
                    "Y 误差列：放在对应 Y 列右侧，自动画成误差棒",
                    list(&[0.0, 1.8, 2.4, 2.9, 2.2, 1.7, 1.1]),
                ),
            ],
        ),
        layer(
            sample(
                Bar,
                "催化剂产率",
                vec![
                    (
                        X,
                        "催化剂编号",
                        "",
                        "X 列：1 Pt/C，2 Pd/C，3 Ru/C，4 Ni/Al₂O₃，5 Cu/ZnO",
                        numbers(5),
                    ),
                    (
                        Y,
                        "第一批",
                        "%",
                        "Y 列：多个 Y 列并排成组",
                        list(&[62.5, 78.3, 85.1, 71.4, 90.2]),
                    ),
                    (
                        Y,
                        "第二批",
                        "%",
                        "Y 列：同一 X 的第二根柱",
                        list(&[60.1, 80.6, 83.7, 74.0, 88.9]),
                    ),
                ],
            ),
            |l| {
                l.y_title = "产率 (%)".into();
                range(l, None, Some((0.0, 120.0)));
            },
        ),
        layer(
            sample(
                StackedBar,
                "发电结构",
                vec![
                    (X, "年份", "", "X 列", years.clone()),
                    (
                        Y,
                        "火电",
                        "TWh",
                        "Y 列：从下往上依次堆叠",
                        list(&[520.0, 535.0, 528.0, 510.0, 492.0, 470.0]),
                    ),
                    (
                        Y,
                        "水电",
                        "TWh",
                        "Y 列：叠在上一列之上",
                        list(&[130.0, 128.0, 135.0, 140.0, 138.0, 142.0]),
                    ),
                    (
                        Y,
                        "新能源",
                        "TWh",
                        "Y 列：最上层",
                        list(&[60.0, 82.0, 105.0, 131.0, 162.0, 195.0]),
                    ),
                ],
            ),
            |l| {
                l.y_title = "发电量 (TWh)".into();
                range(l, Some((2019.0, 2030.0)), Some((0.0, 1000.0)));
            },
        ),
        layer(
            sample(
                PercentStackedBar,
                "废水组分",
                vec![
                    (X, "采样点", "", "X 列：采样点编号", numbers(5)),
                    (
                        Y,
                        "有机物",
                        "mg/L",
                        "Y 列：填原始浓度即可，程序按每个 X 的总量换算成百分比",
                        list(&[120.0, 95.0, 160.0, 80.0, 105.0]),
                    ),
                    (Y, "氨氮", "mg/L", "Y 列", list(&[35.0, 42.0, 28.0, 50.0, 31.0])),
                    (Y, "总磷", "mg/L", "Y 列", list(&[8.0, 6.5, 12.0, 5.0, 9.5])),
                ],
            ),
            |l| range(l, Some((0.0, 9.0)), None),
        ),
        layer(
            sample(
                FloatingBar,
                "月气温范围",
                vec![
                    (X, "月份", "", "X 列", months),
                    (
                        Y,
                        "最低气温",
                        "°C",
                        "Y 列（下端）：浮动柱按“下端、上端”两列一组",
                        list(&[-6.0, -3.5, 2.8, 9.4, 15.2, 19.8, 22.9, 21.7, 16.0, 8.9, 1.5, -4.2]),
                    ),
                    (
                        Y,
                        "最高气温",
                        "°C",
                        "Y 列（上端）：紧跟在下端列右侧",
                        list(&[2.1, 5.8, 12.6, 20.3, 26.4, 30.1, 31.5, 30.4, 26.2, 19.0, 10.2, 3.6]),
                    ),
                ],
            ),
            |l| l.y_title = "气温 (°C)".into(),
        ),
        sample(
            HorizontalBar,
            "处理方法去除率",
            vec![
                (
                    X,
                    "方法编号",
                    "",
                    "X 列：显示在纵轴。1 混凝，2 活性炭，3 臭氧，4 膜分离，5 生物法，6 芬顿",
                    numbers(6),
                ),
                (
                    Y,
                    "去除率",
                    "%",
                    "Y 列：显示为横向条的长度",
                    list(&[45.2, 68.7, 74.1, 91.3, 63.5, 82.8]),
                ),
            ],
        ),
        sample(
            Area,
            "净热流",
            vec![
                (X, "时间", "s", "X 列", values((0..=20).map(f64::from), 0)),
                (
                    Y,
                    "净热流",
                    "W",
                    "Y 列：填充到 0，负值填在基线下方",
                    values(
                        (0..=20).map(|t| {
                            let t = f64::from(t);
                            5.0 * (t / 2.5).sin() * (-t / 15.0).exp()
                        }),
                        2,
                    ),
                ),
            ],
        ),
        sample(
            StepHorizontal,
            "生存曲线",
            vec![
                (
                    X,
                    "时间",
                    "d",
                    "X 列：观察时间点",
                    list(&[0.0, 3.0, 7.0, 12.0, 18.0, 25.0, 33.0, 40.0]),
                ),
                (
                    Y,
                    "存活比例",
                    "",
                    "Y 列：先水平保持到下一个 X，再竖直变化",
                    list(&[1.0, 0.95, 0.85, 0.72, 0.6, 0.48, 0.4, 0.35]),
                ),
            ],
        ),
        layer(
            sample(
                StepVertical,
                "进料设定",
                vec![
                    (
                        X,
                        "时间",
                        "h",
                        "X 列：设定值切换的时刻",
                        list(&[0.0, 2.0, 5.0, 8.0, 12.0, 16.0, 20.0, 24.0]),
                    ),
                    (
                        Y,
                        "进料流量",
                        "L/h",
                        "Y 列：先竖直跳到新值，再水平保持",
                        list(&[0.0, 1.5, 3.0, 2.5, 4.0, 3.5, 2.0, 0.0]),
                    ),
                ],
            ),
            |l| range(l, None, Some((0.0, 5.0))),
        ),
        sample(
            Spline,
            "水的黏度",
            vec![
                (
                    X,
                    "温度",
                    "°C",
                    "X 列：测量点可以较稀疏",
                    values((0..=10).map(|i| f64::from(i) * 10.0), 0),
                ),
                (
                    Y,
                    "动力黏度",
                    "mPa·s",
                    "Y 列：用自然三次样条连成平滑曲线",
                    list(&[
                        1.792, 1.307, 1.002, 0.798, 0.653, 0.547, 0.467, 0.404, 0.355, 0.315, 0.282,
                    ]),
                ),
            ],
        ),
        sample(
            SplineSymbol,
            "吸附等温线",
            vec![
                (
                    X,
                    "平衡压力",
                    "kPa",
                    "X 列",
                    list(&[0.0, 5.0, 10.0, 20.0, 40.0, 60.0, 80.0, 100.0]),
                ),
                (
                    Y,
                    "吸附量",
                    "mmol/g",
                    "Y 列：实测点画符号，并用平滑样条相连",
                    values(
                        [0.0, 5.0, 10.0, 20.0, 40.0, 60.0, 80.0, 100.0].map(|p: f64| 4.2 * 0.08 * p / (1.0 + 0.08 * p)),
                        3,
                    ),
                ),
            ],
        ),
        sample(
            VerticalDrop,
            "正癸烷质谱",
            vec![
                (
                    X,
                    "质荷比",
                    "m/z",
                    "X 列：峰的位置",
                    list(&[27.0, 29.0, 41.0, 43.0, 57.0, 71.0, 85.0, 99.0, 113.0, 142.0]),
                ),
                (
                    Y,
                    "相对丰度",
                    "%",
                    "Y 列：从 0 画竖线到该值，适合谱峰",
                    list(&[15.0, 30.0, 55.0, 100.0, 85.0, 40.0, 25.0, 6.0, 3.0, 5.0]),
                ),
            ],
        ),
    ];
    cases
        .into_iter()
        .enumerate()
        .map(|(i, p)| (format!("{:02}-{}.scv", i + 1, p.graphs[0].name), p))
        .collect()
}

fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples")
}

#[test]
fn every_plot_type_has_a_sample_with_one_table_and_one_graph() {
    let samples = samples();
    let kinds: Vec<_> = samples
        .iter()
        .map(|(_, p)| p.graphs[0].first_layer().spec.plot_type)
        .collect();
    assert_eq!(kinds, PlotType::ALL);
    let readme = fs::read_to_string(directory().join("README.md")).unwrap();
    for (name, p) in &samples {
        assert_eq!((p.books.len(), p.graphs.len()), (1, 1), "{name}");
        let book = &p.books[0];
        assert!(book.columns.iter().all(|c| !c.comment.is_empty()), "{name}");
        assert!(book.columns.iter().all(|c| c.data.len() == book.nrows()), "{name}");
        let layer = p.graphs[0].first_layer();
        // 浮动柱的下端列记在 base_by_y 里，其余图型每个 Y 列都是一条曲线。
        let spec = &layer.spec;
        assert!(
            book.columns
                .iter()
                .filter(|c| c.role == Role::Y)
                .all(|c| spec.y_keys.contains(&c.key) || spec.base_by_y.values().any(|b| *b == c.key)),
            "{name} plots every Y column"
        );
        assert!(readme.contains(name.as_str()), "samples/README.md lists {name}");
    }
}

#[test]
fn committed_samples_match_generated_projects() {
    let samples = samples();
    let mut expected: Vec<_> = samples.iter().map(|(name, _)| name.clone()).collect();
    expected.sort();
    let mut actual: Vec<_> = fs::read_dir(directory())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".scv"))
        .collect();
    actual.sort();
    assert_eq!(actual, expected);
    for (name, project) in samples {
        let committed = fs::read(directory().join(&name)).unwrap();
        assert_eq!(project_file::from_bytes(&committed).unwrap(), project, "{name}");
        assert!(
            committed == project_file::to_scv(&project).unwrap(),
            "{name} is out of date; regenerate with the ignored update_plot_samples test"
        );
    }
}

#[test]
#[ignore = "Explicit maintainer action; unzip and review the generated samples before committing"]
fn update_plot_samples() {
    fs::create_dir_all(directory()).unwrap();
    for (name, project) in samples() {
        project_file::save(&directory().join(name), &project).unwrap();
    }
}
