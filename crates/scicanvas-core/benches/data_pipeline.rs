//! Run with `cargo bench -p scicanvas-core --bench data_pipeline`.
use scicanvas_core::{csv, scene, PlotType, Project};
use std::{fmt::Write, hint::black_box, time::Instant};

fn main() {
    if std::env::args().any(|a| a == "--continuous-review") {
        continuous_review();
        return;
    }
    #[cfg(feature = "serde")]
    if std::env::args().any(|a| a == "--recovery-review") {
        recovery_review();
        return;
    }
    if std::env::args().any(|a| a == "--history-only") {
        history_benchmark();
        return;
    }
    println!("rows,columns,csv_bytes,import_ms,layout_ms,sort_ms,scene_items");
    for rows in [100_000, 1_000_000] {
        // Generate input outside the timed import; descending X exercises actual sorting.
        let mut input = String::from("X,Y1,Y2,Y3,Y4,Y5,Y6,Y7,Y8,Y9\n");
        for i in (0..rows).rev() {
            writeln!(
                input,
                "{i},{},{},{},{},{},{},{},{},{}",
                i % 97,
                i % 89,
                i % 83,
                i % 79,
                i % 73,
                i % 71,
                i % 67,
                i % 61,
                i % 59
            )
            .unwrap();
        }
        let start = Instant::now();
        let book = csv::book_from_text(1, "Benchmark", black_box(&input));
        let import = start.elapsed();
        assert_eq!(book.nrows(), rows);
        assert_eq!(book.columns.len(), 10);
        let mut project = Project::default();
        let book_id = project.new_book();
        *project.book_mut(book_id).unwrap() = book;
        let (id, _) = project
            .plot_from_selection(1, &["B".into()], PlotType::Scatter)
            .unwrap();
        let start = Instant::now();
        let layout = scene::layout_page(project.graph(id).unwrap(), black_box(&project), &[]);
        black_box(&layout);
        let render = start.elapsed();
        let items = layout.items.len();
        drop(layout);
        let start = Instant::now();
        project.sort_book(1, 0, true).unwrap();
        let sort = start.elapsed();
        let book = black_box(&project.books[0]);
        assert_eq!(book.columns[0].data.text_at(0), "0");
        assert_eq!(book.columns[0].data.text_at(rows - 1), (rows - 1).to_string());
        println!(
            "{rows},10,{},{:.3},{:.3},{:.3},{items}",
            input.len(),
            import.as_secs_f64() * 1000.0,
            render.as_secs_f64() * 1000.0,
            sort.as_secs_f64() * 1000.0
        );
    }
}

fn history_benchmark() {
    use scicanvas_core::{history::History, Column, ColumnData, Role};
    let mut base = Project::default();
    let id = base.new_book();
    let book = base.book_mut(id).unwrap();
    book.columns.clear();
    for index in 0..10 {
        let mut c = Column::new(format!("C{index}"), Role::Y);
        c.data = ColumnData::Num {
            values: (0..1_000_000).map(|i| i as f64).collect(),
            missing: vec![0; 1_000_000_usize.div_ceil(64)],
            lexemes: Default::default(),
        }
        .into();
        book.columns.push(c);
    }
    let mut deep = base.clone();
    let start = Instant::now();
    for i in 0..3 {
        let mut snapshot = deep.clone();
        for c in &mut snapshot.books[0].columns {
            c.data = (*c.data).clone().into();
        }
        deep.books[0].columns[0].set(i, (i + 99).to_string());
        black_box(snapshot != deep);
        black_box(snapshot);
    }
    let baseline = start.elapsed();
    let mut shared = base.clone();
    let mut history = History::default();
    let start = Instant::now();
    for i in 0..3 {
        history.begin(&shared, "edit");
        shared.books[0].columns[0].set(i, (i + 99).to_string());
        history.changed();
        history.commit(&shared);
    }
    let optimized = start.elapsed();
    println!(
        "history_rows=1000000 columns=10 edits=3 deep_copy_control_ms={:.3} shared_history_ms={:.3}",
        baseline.as_secs_f64() * 1000.,
        optimized.as_secs_f64() * 1000.
    );
    let start = Instant::now();
    for _ in 0..100 {
        let snapshot = shared.books.clone();
        black_box(snapshot == shared.books);
    }
    println!(
        "shared_book_snapshot_and_equality_100_ms={:.3}",
        start.elapsed().as_secs_f64() * 1000.
    );
}

fn review_project() -> Project {
    use scicanvas_core::{Column, ColumnData, Role};
    let mut p = Project::default();
    let id = p.new_book();
    let book = p.book_mut(id).unwrap();
    book.columns.clear();
    for index in 0..10 {
        let mut c = Column::new(format!("C{index}"), Role::Y);
        c.data = ColumnData::Num {
            values: (0..1_000_000).map(|i| i as f64).collect(),
            missing: vec![0; 1_000_000_usize.div_ceil(64)],
            lexemes: Default::default(),
        }
        .into();
        book.columns.push(c);
    }
    p
}
fn continuous_review() {
    let mut p = review_project();
    let mut h = scicanvas_core::history::History::default();
    h.begin(&p, "continuous");
    let signature = |p: &Project| {
        p.books
            .iter()
            .map(|b| (b.id, b.columns.iter().map(|c| c.data.version()).collect::<Vec<_>>()))
            .collect::<Vec<_>>()
    };
    let mut cache = signature(&p);
    let mut copies = 0;
    let start = Instant::now();
    for i in 0..60 {
        let before = p.books[0].columns[0].data.identity();
        p.books[0].columns[0].set(999_999, (2_000_000 + i).to_string());
        copies += usize::from(before != p.books[0].columns[0].data.identity());
        let current = signature(&p);
        if cache != current {
            cache = current;
        }
        black_box(&cache);
    }
    h.changed();
    h.commit(&p);
    println!(
        "continuous_60_ms={:.3} changed_allocations={copies}",
        start.elapsed().as_secs_f64() * 1000.
    );
}
#[cfg(feature = "serde")]
fn recovery_review() {
    let p = review_project();
    let path = std::env::temp_dir().join(format!("scicanvas-recovery-bench-{}.scproj", std::process::id()));
    let before = peak_memory();
    let start = Instant::now();
    scicanvas_core::project_file::session::save_recovery(&path, &p, None).unwrap();
    let elapsed = start.elapsed();
    let after = peak_memory();
    println!(
        "recovery_ms={:.3} initial_working_set={} peak_working_set={} bytes={}",
        elapsed.as_secs_f64() * 1000.,
        before.0,
        after.1,
        std::fs::metadata(&path).unwrap().len()
    );
    std::fs::remove_file(&path).unwrap();
    let _ = std::fs::remove_file(path.with_extension("source"));
}
#[cfg(feature = "serde")]
fn peak_memory() -> (usize, usize) {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct Counters {
            cb: u32,
            faults: u32,
            peak: usize,
            current: usize,
            peak_paged: usize,
            paged: usize,
            peak_nonpaged: usize,
            nonpaged: usize,
            pagefile: usize,
            peak_pagefile: usize,
        }
        #[link(name = "psapi")]
        unsafe extern "system" {
            fn GetProcessMemoryInfo(handle: *mut std::ffi::c_void, data: *mut Counters, size: u32) -> i32;
        }
        let mut c = Counters {
            cb: std::mem::size_of::<Counters>() as u32,
            faults: 0,
            peak: 0,
            current: 0,
            peak_paged: 0,
            paged: 0,
            peak_nonpaged: 0,
            nonpaged: 0,
            pagefile: 0,
            peak_pagefile: 0,
        };
        // 伪句柄 -1 表示当前进程，结构使用 Win32 PROCESS_MEMORY_COUNTERS 的布局。
        if unsafe { GetProcessMemoryInfo(-1isize as *mut _, &mut c, std::mem::size_of::<Counters>() as u32) } != 0 {
            return (c.current, c.peak);
        }
    }
    (0, 0)
}
