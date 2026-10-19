use scicanvas_core::{csv, ColumnData, ColumnView, DataKind, Project};

fn cells(values: &[&str]) -> ColumnData {
    values.iter().map(|v| v.to_string()).collect()
}

#[test]
fn numeric_storage_preserves_spelling_and_distinguishes_missing_from_nan() {
    let data = cells(&["001", " 1,000 ", "1.0", "", "NaN", "-0", "1e6", " "]);
    assert_eq!(data.kind(), DataKind::Num);
    assert_eq!(data, vec!["001", " 1,000 ", "1.0", "", "NaN", "-0", "1e6", " "]);
    assert_eq!(data.f64_at(0), Some(1.0));
    assert_eq!(data.f64_at(1), Some(1000.0));
    assert_eq!(data.f64_at(3), None);
    assert!(data.f64_at(4).unwrap().is_nan());
    assert_eq!(data.f64_at(5).unwrap().to_bits(), (-0.0f64).to_bits());
    let view: &dyn ColumnView = &data;
    assert_eq!(view.text_at(6), "1e6");
    assert_eq!(view.len(), 8);
}

#[test]
fn import_checks_late_text_without_losing_numeric_rows_or_original_input() {
    let mut text = String::from("X,Y\n");
    for i in 0..1200 {
        text.push_str(&format!(
            "{i},{}\n",
            if i == 1199 {
                "not numeric".to_string()
            } else {
                i.to_string()
            }
        ));
    }
    let book = csv::book_from_table(1, "mixed", csv::parse_table(&text));
    assert_eq!(book.columns[0].data.kind(), DataKind::Num);
    assert_eq!(book.columns[1].data.kind(), DataKind::Text);
    assert_eq!(book.columns[1].data.f64_at(1001), Some(1001.0));
    assert_eq!(book.columns[1].data.text_at(1199), "not numeric");
}

#[test]
fn row_edits_and_reordering_keep_type_masks_lexemes_and_category_dictionary() {
    for data in [
        cells(&["01", "", "3"]),
        cells(&["a", "", "c"]),
        cells(&["2026-10-18T10:00:00+01:00", "", "2026-10-19"])
            .converted(DataKind::DateTime)
            .unwrap(),
        cells(&["group B", "", "group A"])
            .converted(DataKind::Category)
            .unwrap(),
    ] {
        let original = data.clone();
        let mut edited = data.clone();
        edited.insert_empty(1, 65);
        assert_eq!(edited.text_at(66), "");
        assert_eq!(edited.text_at(67), original.text_at(2));
        edited.validate().unwrap();
        edited.delete_rows(1, 65);
        assert_eq!(edited, original);
        let mut reordered = original.reordered(&[2, 0, 1, usize::MAX]);
        assert_eq!(reordered.kind(), original.kind());
        assert_eq!(reordered.text_at(0), original.text_at(2));
        reordered.trim_empty();
        assert_eq!(reordered.len(), 2);
        reordered.validate().unwrap();
    }
}

#[test]
fn conversions_are_atomic_and_dates_use_utc_milliseconds() {
    let data = cells(&["2026-10-18T10:00:00+01:00", "2026-10-18T09:00:00Z", "", "2026-10-19"]);
    let dates = data.converted(DataKind::DateTime).unwrap();
    assert_eq!(dates.f64_at(0), dates.f64_at(1));
    assert_eq!(dates.f64_at(2), None);
    assert_eq!(dates.text_at(0), "2026-10-18T10:00:00+01:00");
    assert!(cells(&["bad date"]).converted(DataKind::DateTime).is_err());
    let mut project = Project::sample();
    project.books[0].columns[1].data = cells(&["bad"]);
    let before = project.clone();
    assert!(project.books[0]
        .convert_columns(&["A".into(), "B".into()], DataKind::DateTime)
        .is_err());
    assert_eq!(project, before);
}

#[test]
fn common_date_spellings_and_years_parse_but_bare_integers_do_not() {
    let ms = |text: &str| {
        cells(&[text])
            .converted(DataKind::DateTime)
            .map(|d| d.f64_at(0).unwrap())
    };
    // 2026-10-18T00:00:00Z
    for text in [
        "2026-10-18",
        "2026/10/18",
        "2026.10.18",
        "2026年10月18日",
        "2026/10/18 00:00",
    ] {
        assert_eq!(ms(text), Ok(1_792_281_600_000.0), "{text}");
    }
    // 2026-10-18T14:30:00Z，无时区的写法按 UTC
    for text in [
        "2026-10-18 14:30",
        "2026-10-18T14:30:00",
        "2026/10/18 14:30:00",
        "2026年10月18日 14:30",
    ] {
        assert_eq!(ms(text), Ok(1_792_333_800_000.0), "{text}");
    }
    assert_eq!(ms("2026-10-18 14:30:15.5"), Ok(1_792_333_815_500.0));
    // 四位年份取 1 月 1 日：2020-01-01T00:00:00Z
    assert_eq!(ms("2020"), Ok(1_577_836_800_000.0));
    let years = cells(&["2020", "2021"]).converted(DataKind::DateTime).unwrap();
    assert_eq!(years.f64_at(1), Some(1_609_459_200_000.0));
    assert_eq!(years.text_at(0), "2020");
    // 纯整数、小数和日月顺序不确定的写法拒绝转换，原列保持不变
    for text in [
        "20261018",
        "1700000000000",
        "0",
        "0.5",
        "999",
        "18/10/2026",
        "10/18/2026",
        "2026-13-01",
        "2026-02-30",
    ] {
        assert!(ms(text).is_err(), "{text}");
    }
}

#[test]
fn dates_saved_as_millisecond_integers_by_the_first_v9_build_still_load() {
    let legacy = ColumnData::DateTime {
        values: vec![1_700_000_000_000],
        missing: vec![0],
        lexemes: [(0, "1700000000000".to_string())].into_iter().collect(),
    };
    legacy.validate().unwrap();
    assert_eq!(legacy.f64_at(0), Some(1_700_000_000_000.0));
    // 新输入的整数不再当作毫秒
    assert!(!legacy.accepts("1700000000000"));
}

#[test]
fn typed_sorting_is_stable_and_keeps_missing_values_last_in_both_directions() {
    let mut p = Project::sample();
    p.books[0].columns.truncate(2);
    p.books[0].columns[0].data = cells(&["2", "", "1", "1"]);
    p.books[0].columns[1].data = cells(&["a", "blank", "b", "c"]);
    p.sort_book(1, 0, true).unwrap();
    assert_eq!(p.books[0].columns[1].data, vec!["b", "c", "a", "blank"]);
    assert_eq!(p.books[0].columns[0].data.kind(), DataKind::Num);
    p.sort_book(1, 0, false).unwrap();
    assert_eq!(p.books[0].columns[1].data, vec!["a", "b", "c", "blank"]);
    p.books[0].columns[0].data = cells(&["late", "early", "late", ""])
        .converted(DataKind::Category)
        .unwrap();
    if let ColumnData::Category { ordered, .. } = &mut p.books[0].columns[0].data {
        *ordered = true;
    }
    p.sort_book(1, 0, true).unwrap();
    assert_eq!(p.books[0].columns[0].data, vec!["late", "late", "early"]);
    assert!(
        matches!(&p.books[0].columns[0].data,ColumnData::Category {levels,ordered:true,..} if levels==&["late","early"])
    );
}

#[test]
fn million_numeric_cells_use_compact_storage() {
    let data: ColumnData = (0..1_000_000).map(|i| i.to_string()).collect();
    assert_eq!(data.kind(), DataKind::Num);
    assert!(data.estimated_bytes() < 12 * 1024 * 1024);
    assert_eq!(data.f64_at(999_999), Some(999_999.0));
}

#[test]
fn calculations_preserve_explicit_target_types_and_failed_date_writes_are_atomic() {
    use scicanvas_core::stats::Sequence;
    let mut p = Project::sample();
    p.books[0].columns[0].data = ColumnData::Text(vec!["1".into(), "2".into()]);
    p.set_column_values(1, "A", "i*2").unwrap();
    assert_eq!(p.books[0].columns[0].data.kind(), DataKind::Text);
    p.books[0].columns[0].data = cells(&["2026-10-18", ""]).converted(DataKind::DateTime).unwrap();
    let before = p.clone();
    assert!(p
        .fill_sequence(1, "A", Sequence::Arithmetic { start: 0.5, step: 0.2 }, 3, 1)
        .is_err());
    assert_eq!(p, before);
    assert!(p.set_column_values(1, "A", "i/2").is_err());
    assert_eq!(p, before);
    let mut category = cells(&["B", "A"]).converted(DataKind::Category).unwrap();
    if let ColumnData::Category { ordered, .. } = &mut category {
        *ordered = true;
    }
    let replaced = category.replaced_texts(vec!["A".into(), "C".into()]).unwrap();
    assert!(matches!(&replaced,ColumnData::Category {levels,ordered:true,..} if levels==&["B","A","C"]));
    assert_eq!(replaced, vec!["A", "C"]);
}

#[cfg(feature = "serde")]
mod files {
    use super::*;
    use scicanvas_core::{project_file, svg};

    fn v8_json(p: &Project) -> serde_json::Value {
        let mut old = serde_json::to_value(p).unwrap();
        old["version"] = 8.into();
        for (book, wire) in p.books.iter().zip(old["books"].as_array_mut().unwrap()) {
            for (col, wire) in book.columns.iter().zip(wire["columns"].as_array_mut().unwrap()) {
                wire["data"] = serde_json::json!(col.data.iter().map(|v| v.into_owned()).collect::<Vec<_>>());
            }
        }
        old
    }

    #[test]
    fn v8_string_arrays_migrate_losslessly_and_keep_csv_and_svg_bytes() {
        let mut p = Project::sample();
        p.books[0].columns[0].data = cells(&["001", "2.0", " 3,000 ", "4e0", "NaN", "", "7"]);
        p.books[0].columns[1].data = cells(&["1", "2", "bad", "4", "5", "6", "7"]);
        let old = v8_json(&p);
        let restored = project_file::from_json(&old.to_string()).unwrap();
        assert_eq!(restored, p);
        assert_eq!(restored.version, project_file::CURRENT_VERSION);
        assert_eq!(
            svg::render_page(&restored.graphs[0], &restored),
            svg::render_page(&p.graphs[0], &p)
        );
        assert_eq!(
            project_file::book_csv(&restored.books[0]),
            project_file::book_csv(&p.books[0])
        );
        assert!(old["books"][0]["columns"][0]["data"].is_array());
        let current: serde_json::Value = serde_json::from_str(&project_file::to_json(&restored).unwrap()).unwrap();
        assert!(current["books"][0]["columns"][0]["data"]["values_le"].is_string());
        for version in 2..=8 {
            let mut legacy = old.clone();
            legacy["version"] = version.into();
            assert_eq!(project_file::from_json(&legacy.to_string()).unwrap(), p, "v{version}");
        }
    }

    #[test]
    fn v9_roundtrips_all_types_masks_and_exact_nonfinite_bits_in_json_and_scv() {
        let mut p = Project::sample();
        p.books[0].columns[0].data = cells(&["-0", "NaN", "", "inf"]);
        if let ColumnData::Num { values, .. } = &mut p.books[0].columns[0].data {
            values[1] = f64::from_bits(0x7ff8000000000123);
        }
        p.books[0].columns[1].data = cells(&["2026-10-18T10:00:00+01:00", "", "2026-10-19"])
            .converted(DataKind::DateTime)
            .unwrap();
        p.books[0].columns[2].data = cells(&["类别甲", "", "类别乙"]).converted(DataKind::Category).unwrap();
        let key = p.books[0].push_column(scicanvas_core::Role::Ignore);
        let index = p.books[0].column_index(&key).unwrap();
        p.books[0].columns[index].data = ColumnData::Text(vec!["literal".into(), "001".into(), "".into()]);
        let restored = project_file::from_json(&project_file::to_json(&p).unwrap()).unwrap();
        assert_eq!(restored, p);
        assert_eq!(project_file::from_bytes(&project_file::to_scv(&p).unwrap()).unwrap(), p);
        assert_eq!(
            restored.books[0].columns[0].data.f64_at(1).unwrap().to_bits(),
            0x7ff8000000000123
        );
    }

    #[test]
    fn malformed_typed_columns_are_rejected_instead_of_repairing_values() {
        let valid = serde_json::to_value(Project::sample()).unwrap();
        for data in [
            serde_json::json!({"kind":"num","values_le":"not base64","missing":[]}),
            serde_json::json!({"kind":"num","values_le":"AA==","missing":[0]}),
            serde_json::json!({"kind":"num","values_le":"AAAAAAAAAAA=","missing":[]}),
            serde_json::json!({"kind":"num","values_le":"AAAAAAAAAAA=","missing":[2]}),
            serde_json::json!({"kind":"num","values_le":"AAAAAAAAAAA=","missing":[0],"lexemes":[[0,"wrong"]]}),
            serde_json::json!({"kind":"num","values_le":"AAAAAAAAAAA=","missing":[0],"lexemes":[[0,"0"],[0,"0"]]}),
            serde_json::json!({"kind":"category","codes":[1],"levels":["only"],"ordered":false}),
            serde_json::json!({"kind":"category","codes":[0],"levels":["same","same"],"ordered":false}),
            serde_json::json!({"kind":"date_time","values_le":"/////////38=","missing":[0]}),
            serde_json::json!({"kind":"future_column","values":[]}),
            serde_json::json!(["old schema under v9"]),
        ] {
            let mut bad = valid.clone();
            bad["books"][0]["columns"][0]["data"] = data;
            assert!(project_file::from_json(&bad.to_string()).is_err(), "{bad}");
        }
        let mut old = v8_json(&Project::sample());
        old["books"][0]["columns"][0]["data"] = serde_json::json!(["1", 23]);
        assert!(project_file::from_json(&old.to_string()).is_err());
    }
}

#[test]
fn column_time_zones_keep_wall_time_for_naive_entries_and_instants_for_zoned_ones() {
    use scicanvas_core::{format::DateFormat, Column, Role};
    let ms = |t: &str| chrono::DateTime::parse_from_rfc3339(t).unwrap().timestamp_millis() as f64;
    let mut c = Column::new("A", Role::X);
    c.data = cells(&["2026-10-18 08:00", "2026-10-18T00:00:00Z"]);
    c.data = c.data.converted(DataKind::DateTime).unwrap();
    c.format.date = DateFormat::DateMinute;
    // UTC+8：不带时区的 08:00 是当地时间（UTC 00:00）；带 Z 的保持原时刻，两者显示相同
    c.set_utc_offset(480);
    assert_eq!(c.data.f64_at(0), Some(ms("2026-10-18T00:00:00Z")));
    assert_eq!(c.data.f64_at(1), Some(ms("2026-10-18T00:00:00Z")));
    assert_eq!(
        (c.display_at(0), c.display_at(1)),
        ("2026-10-18 08:00".into(), "2026-10-18 08:00".into())
    );
    // 新输入按列时区理解；原文保留
    c.set(2, "2026-10-19 12:00".into());
    assert_eq!(c.data.f64_at(2), Some(ms("2026-10-19T04:00:00Z")));
    assert_eq!(c.data.text_at(2), "2026-10-19 12:00");
    // 改回 UTC：墙上时间不变
    c.set_utc_offset(0);
    assert_eq!(c.data.f64_at(0), Some(ms("2026-10-18T08:00:00Z")));
    assert_eq!(c.data.f64_at(1), Some(ms("2026-10-18T00:00:00Z")));
    assert_eq!(scicanvas_core::format::utc_offset_label(330), "UTC+5:30");
    assert_eq!(scicanvas_core::format::utc_offset_label(-180), "UTC−3");
}

#[test]
fn date_axes_align_to_local_midnight_and_save_the_zone() {
    use scicanvas_core::{scene, PlotType};
    let mut p = Project::default();
    p.new_book();
    let b = p.book_mut(1).unwrap();
    b.columns[0].data = cells(&["2026-10-18", "2026-10-19", "2026-10-20"]);
    b.columns[1].data = cells(&["1", "3", "2"]);
    b.convert_columns(&["A".into()], DataKind::DateTime).unwrap();
    b.columns[0].set_utc_offset(480);
    let (id, _) = p.plot_from_selection(1, &[], PlotType::LineSymbol).unwrap();
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    // 刻度在北京时间零点（UTC 前一天 16:00），标签仍是当地日期
    let first = chrono::DateTime::parse_from_rfc3339("2026-10-17T16:00:00Z")
        .unwrap()
        .timestamp_millis() as f64;
    assert_eq!(sc.xa.ticks.first(), Some(&first));
    let texts: Vec<String> = sc
        .items
        .iter()
        .filter_map(|it| match &it.prim {
            scene::Prim::Text { text, .. } if it.clip.is_none() => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t == "10-18 00:00"), "{texts:?}");
    #[cfg(feature = "serde")]
    {
        let json = scicanvas_core::project_file::to_json(&p).unwrap();
        assert!(json.contains("utc_offset"));
        let back = scicanvas_core::project_file::from_json(&json).unwrap();
        assert_eq!(back.book(1).unwrap().columns[0].format.utc_offset, 480);
    }
}
