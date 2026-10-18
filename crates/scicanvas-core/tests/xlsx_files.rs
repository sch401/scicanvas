#![cfg(feature = "xlsx")]

use std::io::{Cursor, Write};

use scicanvas_core::xlsx::{read, Sheet};

fn rows(cells: &[&[&str]]) -> Vec<Vec<String>> {
    cells
        .iter()
        .map(|r| r.iter().map(|s| s.to_string()).collect())
        .collect()
}

#[test]
fn reads_an_openpyxl_workbook_with_inline_strings_and_tidy_numbers() {
    let sheets = read(include_bytes!("fixtures/sample.xlsx")).unwrap();
    let names: Vec<_> = sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["测量", "空表", "对照"]);
    assert_eq!(
        sheets[0].rows,
        rows(&[
            &["时间 (s)", "电压 (mV)", "备注"],
            &["0", "10", "第1次"],
            &["0.5", "8", ""],
            &["1", "6.4", "第3次"],
            &["1.5", "5.12", ""],
            &["2", "4.096", "第5次"],
            // 0.1 + 0.2 在 Excel 里存为 0.30000000000000004，按工作表显示格式整理
            &["", "0.3", ""],
        ])
    );
    assert!(sheets[1].rows.is_empty());
    assert_eq!(
        sheets[2].rows,
        rows(&[
            &["浓度", "吸光度", ""],
            &["1", "0.12", ""],
            &["", "", ""],
            &["3", "", "TRUE"]
        ])
    );
}

fn xlsx(files: &[(&str, &str)]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, text) in files {
        zip.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(text.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

const NS: &str = r#"xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#;

#[test]
fn shared_and_rich_strings_formulas_and_relationship_targets() {
    let workbook = format!(
        r#"<workbook {NS}><sheets><sheet name="结果" sheetId="2" r:id="rId9"/><sheet name="原始" sheetId="1" r:id="rId3"/></sheets></workbook>"#
    );
    let rels = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
        <Relationship Id="rId3" Type="worksheet" Target="worksheets/sheet1.xml"/>
        <Relationship Id="rId9" Type="worksheet" Target="/xl/worksheets/data.xml"/></Relationships>"#;
    let shared = format!(
        r#"<sst {NS}><si><t>温度</t></si><si><r><t>电</t></r><r><rPr><b/></rPr><t>导率</t></r><rPh><t>でんどう</t></rPh></si><si><t xml:space="preserve"> 备注 </t></si></sst>"#
    );
    let raw = format!(
        r#"<worksheet {NS}><sheetData>
        <row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c></row>
        <row r="2"><c r="A2"><v>25</v></c><c r="B2"><f>A2*2</f><v>50.000000000000007</v></c></row>
        <row><c><v>1e-3</v></c><c t="b"><v>0</v></c><c t="e"><v>#DIV/0!</v></c></row>
        </sheetData></worksheet>"#
    );
    let result = format!(
        r#"<worksheet {NS}><sheetData><row r="2"><c r="C2" t="str"><f>CONCAT(A1,"x")</f><v>温度x</v></c><c r="D2" t="inlineStr"><is><t>内联</t></is></c><c r="AA2" t="s"><v>2</v></c></row></sheetData></worksheet>"#
    );
    let bytes = xlsx(&[
        ("xl/workbook.xml", &workbook),
        ("xl/_rels/workbook.xml.rels", rels),
        ("xl/sharedStrings.xml", &shared),
        ("xl/worksheets/sheet1.xml", &raw),
        ("xl/worksheets/data.xml", &result),
    ]);
    let sheets = read(&bytes).unwrap();
    assert_eq!(sheets[0].name, "结果", "sheets follow the workbook tab order");
    let r = &sheets[0].rows;
    assert_eq!((r.len(), r[1].len()), (2, 27));
    assert_eq!(
        (r[1][2].as_str(), r[1][3].as_str(), r[1][26].as_str()),
        ("温度x", "内联", " 备注 ")
    );
    assert_eq!(
        sheets[1],
        Sheet {
            name: "原始".into(),
            rows: rows(&[
                &["温度", "电导率", ""],
                &["25", "50", ""],
                &["0.001", "FALSE", "#DIV/0!"]
            ]),
        }
    );
}

#[test]
fn legacy_and_broken_files_are_reported() {
    let mut xls = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    xls.extend([0; 64]);
    assert!(read(&xls).unwrap_err().contains(".xls"));
    assert!(read(b"not a zip").is_err());
    assert!(read(&xlsx(&[("docProps/app.xml", "<x/>")]))
        .unwrap_err()
        .contains("workbook"));
    let workbook = format!(r#"<workbook {NS}><sheets><sheet name="缺失" r:id="rId1"/></sheets></workbook>"#);
    assert!(read(&xlsx(&[("xl/workbook.xml", &workbook)]))
        .unwrap_err()
        .contains("缺失"));
}
