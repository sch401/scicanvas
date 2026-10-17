#![cfg(feature = "serde")]

use std::io::{Cursor, Read, Write};

use scicanvas_core::{project_file, svg, Project};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const V1: &str = include_str!("fixtures/legacy-v1.scproj");

fn project() -> Project {
    let mut p = Project::sample();
    let folder = p.add_folder(None).unwrap();
    p.rename_project_item(folder, "实验 α").unwrap();
    p.move_project_item(2, Some(folder)).unwrap();
    p.rename_project_item(1, "电压/电流: 1?").unwrap();
    let book = &mut p.books[0];
    book.columns[0].long_name = "时间".into();
    book.columns[0].units = "s".into();
    book.columns[1].long_name = "备注,\"引号\"".into();
    book.columns[1].units.clear();
    book.columns[1].data[0] = " 前后空格 ".into();
    p
}

fn entries(bytes: &[u8]) -> Vec<(String, CompressionMethod, String)> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..zip.len())
        .map(|i| {
            let mut entry = zip.by_index(i).unwrap();
            let mut text = String::new();
            entry.read_to_string(&mut text).unwrap();
            (entry.name().unwrap().into_owned(), entry.compression(), text)
        })
        .collect()
}

fn rezip(files: &[(&str, &str)]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, text) in files {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(text.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

#[test]
fn scv_roundtrips_and_unzips_to_readable_tables_and_graphs() {
    let p = project();
    let bytes = project_file::to_scv(&p).unwrap();
    assert_eq!(project_file::from_bytes(&bytes).unwrap(), p);
    assert_eq!(
        project_file::to_scv(&p).unwrap(),
        bytes,
        "same project gives identical bytes"
    );

    let files = entries(&bytes);
    let names: Vec<_> = files.iter().map(|f| f.0.as_str()).collect();
    assert_eq!(
        names,
        [
            "mimetype",
            "manifest.json",
            "project.json",
            "tables/1-电压_电流_ 1_.csv",
            "graphs/2-Graph1.svg"
        ]
    );
    assert_eq!(files[0].1, CompressionMethod::Stored);
    assert_eq!(files[0].2, project_file::SCV_MIMETYPE);
    assert_eq!(files[2].2, project_file::to_json(&p).unwrap());
    assert_eq!(files[3].2, project_file::book_csv(&p.books[0]));
    assert!(files[3]
        .2
        .starts_with("\u{feff}时间 (s),\"备注,\"\"引号\"\"\",电压 B (mV)\r\n"));
    assert!(files[3].2.contains("\" 前后空格 \""));
    assert_eq!(files[4].2, svg::render_page(&p.graphs[0], &p));

    let manifest: serde_json::Value = serde_json::from_str(&files[1].2).unwrap();
    assert_eq!(manifest["scv_version"], project_file::SCV_VERSION);
    assert_eq!(manifest["project_version"], project_file::CURRENT_VERSION);
    assert_eq!(manifest["tables"][0]["path"], names[3]);
    assert_eq!(manifest["tables"][0]["folder"], "");
    assert_eq!(manifest["graphs"][0]["folder"], "实验 α");
}

#[test]
fn opening_reads_only_project_json_and_rejects_foreign_or_newer_archives() {
    let p = project();
    let json = project_file::to_json(&p).unwrap();
    // 预览被删改或缺少 mimetype / manifest 都不影响打开。
    let edited = rezip(&[("project.json", &json), ("tables/1-x.csv", "改过的内容")]);
    assert_eq!(project_file::from_bytes(&edited).unwrap(), p);

    for (files, message) in [
        (
            vec![("mimetype", "application/zip"), ("project.json", json.as_str())],
            "不是 SciCanvas",
        ),
        (vec![("mimetype", project_file::SCV_MIMETYPE)], "缺少 project.json"),
        (
            vec![
                ("manifest.json", r#"{"scv_version": 99}"#),
                ("project.json", json.as_str()),
            ],
            "高于当前支持",
        ),
        (
            vec![("manifest.json", "{}"), ("project.json", json.as_str())],
            "scv_version",
        ),
        (vec![("project.json", r#"{"version": 999}"#)], "高于当前支持"),
    ] {
        let error = project_file::from_bytes(&rezip(&files)).unwrap_err();
        assert!(error.contains(message), "{error}");
    }
    assert!(project_file::from_bytes(b"PK\x03\x04broken").is_err());
    let mut truncated = project_file::to_scv(&p).unwrap();
    truncated.truncate(truncated.len() / 2);
    assert!(project_file::from_bytes(&truncated).is_err());
}

#[test]
fn legacy_json_still_opens_and_save_format_follows_extension() {
    let legacy = project_file::from_json(V1).unwrap();
    assert_eq!(project_file::from_bytes(V1.as_bytes()).unwrap(), legacy);
    assert_eq!(
        project_file::from_bytes(format!("\u{feff}{V1}").as_bytes()).unwrap(),
        legacy
    );

    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/file-tests")
        .join(format!("scv-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let p = project();
    let scv = directory.join("项目.SCV");
    let json = directory.join("项目.scproj");
    project_file::save(&scv, &p).unwrap();
    project_file::save(&json, &p).unwrap();
    assert!(std::fs::read(&scv).unwrap().starts_with(b"PK\x03\x04"));
    assert_eq!(
        std::fs::read_to_string(&json).unwrap(),
        project_file::to_json(&p).unwrap()
    );
    assert_eq!(project_file::load(&scv).unwrap(), p);
    assert_eq!(project_file::load(&json).unwrap(), p);

    let original = std::fs::read(&scv).unwrap();
    let mut invalid = p.clone();
    invalid.graphs[0].first_layer_mut().line_width = f32::NAN;
    assert!(project_file::save(&scv, &invalid).is_err());
    assert_eq!(std::fs::read(&scv).unwrap(), original);
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 2);
    std::fs::remove_dir_all(directory).unwrap();
}
