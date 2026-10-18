//! Run with `cargo bench -p scicanvas-core --bench data_pipeline`.
use scicanvas_core::{csv, scene, PlotType, Project};
use std::{fmt::Write, hint::black_box, time::Instant};

fn main() {
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
        let book = csv::book_from_table(1, "Benchmark", csv::parse_table(black_box(&input)));
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
