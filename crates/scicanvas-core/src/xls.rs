//! 读取旧版 Excel 97–2003 工作簿（.xls，BIFF8）：OLE2 复合文档里的 Workbook 流。
//! 只取单元格的值（公式取缓存结果）；日期按单元格数字格式识别，合并单元格用左上角的值填满。

use crate::xlsx::{excel_date_text, excel_number, fill_merged, is_date_format, Sheet};

const FREE: u32 = 0xFFFF_FFFF;
const END: u32 = 0xFFFF_FFFE;

fn u16_at(b: &[u8], i: usize) -> Result<u16, String> {
    b.get(i..i + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or_else(|| "文件内容不完整".into())
}

fn u32_at(b: &[u8], i: usize) -> Result<u32, String> {
    b.get(i..i + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| "文件内容不完整".into())
}

fn f64_at(b: &[u8], i: usize) -> Result<f64, String> {
    b.get(i..i + 8)
        .map(|s| f64::from_le_bytes(s.try_into().unwrap()))
        .ok_or_else(|| "文件内容不完整".into())
}

/// OLE2 复合文档：按名字取出一个流。
fn cfb_stream(bytes: &[u8], wanted: &[&str]) -> Result<Vec<u8>, String> {
    if bytes.len() < 512 || !bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        return Err("不是有效的 .xls 文件".into());
    }
    let shift = u16_at(bytes, 0x1E)?;
    let mini_shift = u16_at(bytes, 0x20)?;
    if !(7..=16).contains(&shift) || mini_shift >= shift {
        return Err("不是有效的 .xls 文件（扇区大小错误）".into());
    }
    let size = 1usize << shift;
    let sector = |n: u32| -> Result<&[u8], String> {
        let start = (n as usize + 1) << shift;
        bytes.get(start..start + size).ok_or_else(|| "文件内容不完整".into())
    };
    // FAT 扇区列表：头部 109 个，其余在 DIFAT 链里
    let n_fat = u32_at(bytes, 0x2C)? as usize;
    let mut fat_sectors = vec![];
    for i in 0..109.min(n_fat) {
        fat_sectors.push(u32_at(bytes, 0x4C + 4 * i)?);
    }
    let mut difat = u32_at(bytes, 0x44)?;
    let mut guard = 0;
    while fat_sectors.len() < n_fat && difat != END && difat != FREE {
        let s = sector(difat)?;
        for i in 0..size / 4 - 1 {
            if fat_sectors.len() < n_fat {
                fat_sectors.push(u32_at(s, 4 * i)?);
            }
        }
        difat = u32_at(s, size - 4)?;
        guard += 1;
        if guard > 1 << 20 {
            return Err("文件结构损坏（DIFAT 循环）".into());
        }
    }
    let mut fat = vec![];
    for &n in &fat_sectors {
        let s = sector(n)?;
        for i in 0..size / 4 {
            fat.push(u32_at(s, 4 * i)?);
        }
    }
    let chain = |start: u32, table: &[u32]| -> Result<Vec<u32>, String> {
        let mut out = vec![];
        let mut n = start;
        while n != END && n != FREE {
            out.push(n);
            if out.len() > table.len() {
                return Err("文件结构损坏（扇区链循环）".into());
            }
            n = *table.get(n as usize).ok_or("文件结构损坏（扇区越界）")?;
        }
        Ok(out)
    };
    let read_chain = |start: u32| -> Result<Vec<u8>, String> {
        let mut out = vec![];
        for n in chain(start, &fat)? {
            out.extend_from_slice(sector(n)?);
        }
        Ok(out)
    };
    let dir = read_chain(u32_at(bytes, 0x30)?)?;
    let entry = |i: usize| dir.get(i * 128..i * 128 + 128);
    let name_of = |e: &[u8]| {
        let len = (u16_at(e, 64).unwrap_or(0) as usize).min(64).saturating_sub(2);
        let units: Vec<u16> = e[..len].chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&units)
    };
    let root = entry(0).ok_or("文件结构损坏（没有根目录）")?;
    let mut found = None;
    for i in 0..dir.len() / 128 {
        let e = entry(i).unwrap();
        if e[66] == 2 && wanted.contains(&name_of(e).as_str()) {
            found = Some(e);
            break;
        }
    }
    let Some(e) = found else {
        let names: Vec<String> = (0..dir.len() / 128).filter_map(entry).map(name_of).collect();
        return Err(if names.iter().any(|n| n == "Book") {
            "这是 Excel 5.0/95 格式，请在 Excel 中另存为 .xlsx 后再导入".into()
        } else {
            "文件里没有 Excel 工作簿数据".into()
        });
    };
    let start = u32_at(e, 116)?;
    let len = u32_at(e, 120)? as usize;
    let cutoff = u32_at(bytes, 0x38)? as usize;
    let mut data = if len < cutoff {
        // 小流在迷你流里：迷你 FAT 链，64 字节一个迷你扇区
        let mini_fat_bytes = read_chain(u32_at(bytes, 0x3C)?)?;
        let mini_fat: Vec<u32> = (0..mini_fat_bytes.len() / 4)
            .map(|i| u32_at(&mini_fat_bytes, 4 * i).unwrap())
            .collect();
        let mini_stream = read_chain(u32_at(root, 116)?)?;
        let mini = 1usize << mini_shift;
        let mut out = vec![];
        for n in chain(start, &mini_fat)? {
            let at = n as usize * mini;
            out.extend_from_slice(mini_stream.get(at..at + mini).ok_or("文件内容不完整")?);
        }
        out
    } else {
        read_chain(start)?
    };
    if data.len() < len {
        return Err("文件内容不完整".into());
    }
    data.truncate(len);
    Ok(data)
}

/// BIFF 记录：(类型, 数据)。
fn records(stream: &[u8], from: usize) -> impl Iterator<Item = (u16, &[u8])> {
    let mut at = from;
    std::iter::from_fn(move || {
        let kind = u16_at(stream, at).ok()?;
        let len = u16_at(stream, at + 2).ok()? as usize;
        let data = stream.get(at + 4..at + 4 + len)?;
        at += 4 + len;
        Some((kind, data))
    })
}

/// 跨 CONTINUE 记录读取的字节流（共享字符串表会被拆开）。
struct Chunks<'a> {
    parts: Vec<&'a [u8]>,
    part: usize,
    pos: usize,
}

impl<'a> Chunks<'a> {
    fn byte(&mut self) -> Result<u8, String> {
        while self.part < self.parts.len() && self.pos >= self.parts[self.part].len() {
            self.part += 1;
            self.pos = 0;
        }
        let b = *self
            .parts
            .get(self.part)
            .and_then(|p| p.get(self.pos))
            .ok_or("共享字符串表不完整")?;
        self.pos += 1;
        Ok(b)
    }

    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes([self.byte()?, self.byte()?]))
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes([
            self.byte()?,
            self.byte()?,
            self.byte()?,
            self.byte()?,
        ]))
    }

    /// XLUnicodeRichExtendedString：字符跨记录时，新记录开头另有一个选项字节决定宽度。
    fn string(&mut self) -> Result<String, String> {
        let mut left = self.u16()? as usize;
        let flags = self.byte()?;
        let runs = if flags & 0x08 != 0 { self.u16()? as usize } else { 0 };
        let ext = if flags & 0x04 != 0 { self.u32()? as usize } else { 0 };
        let mut wide = flags & 0x01 != 0;
        let mut units: Vec<u16> = Vec::with_capacity(left);
        while left > 0 {
            if self.part < self.parts.len() && self.pos >= self.parts[self.part].len() {
                self.part += 1;
                self.pos = 0;
                wide = self.byte()? & 0x01 != 0;
            }
            units.push(if wide { self.u16()? } else { u16::from(self.byte()?) });
            left -= 1;
        }
        for _ in 0..runs * 4 + ext {
            self.byte()?;
        }
        Ok(String::from_utf16_lossy(&units))
    }
}

/// 单条记录里的 XLUnicodeString（cch 为 u16 或 u8）。
fn short_string(data: &[u8], at: usize, wide_len: bool) -> Result<String, String> {
    let (len, at) = if wide_len {
        (u16_at(data, at)? as usize, at + 2)
    } else {
        (*data.get(at).ok_or("文件内容不完整")? as usize, at + 1)
    };
    let flags = *data.get(at).ok_or("文件内容不完整")?;
    let at = at + 1;
    if flags & 0x01 != 0 {
        let raw = data.get(at..at + 2 * len).ok_or("文件内容不完整")?;
        let units: Vec<u16> = raw.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        Ok(String::from_utf16_lossy(&units))
    } else {
        let raw = data.get(at..at + len).ok_or("文件内容不完整")?;
        Ok(raw.iter().map(|&b| char::from(b)).collect())
    }
}

/// RK 数值：整数或截断的双精度，可选除以 100。
fn rk(v: u32) -> f64 {
    let x = if v & 0x02 != 0 {
        f64::from((v as i32) >> 2)
    } else {
        f64::from_bits(u64::from(v & 0xFFFF_FFFC) << 32)
    };
    if v & 0x01 != 0 {
        x / 100.0
    } else {
        x
    }
}

fn error_text(code: u8) -> String {
    match code {
        0x00 => "#NULL!",
        0x07 => "#DIV/0!",
        0x0F => "#VALUE!",
        0x17 => "#REF!",
        0x1D => "#NAME?",
        0x24 => "#NUM!",
        _ => "#N/A",
    }
    .into()
}

struct WorkbookGlobals {
    sheets: Vec<(String, usize)>,
    sst: Vec<String>,
    formats: std::collections::HashMap<u16, String>,
    xf_format: Vec<u16>,
    date1904: bool,
}

fn workbook_globals(stream: &[u8]) -> Result<WorkbookGlobals, String> {
    let mut sheets: Vec<(String, usize)> = vec![];
    let mut sst: Vec<String> = vec![];
    let mut formats: std::collections::HashMap<u16, String> = Default::default();
    let mut xf_format: Vec<u16> = vec![];
    let mut date1904 = false;
    // 工作簿全局区：直到第一个 EOF
    let mut globals = records(stream, 0).peekable();
    let Some((0x0809, bof)) = globals.next() else {
        return Err("不是有效的 .xls 文件（缺少 BOF）".into());
    };
    if u16_at(bof, 0)? != 0x0600 {
        return Err("只支持 Excel 97–2003（BIFF8）格式的 .xls，请在 Excel 中另存为 .xlsx 后再导入".into());
    }
    while let Some((kind, data)) = globals.next() {
        match kind {
            0x000A => break,
            0x002F => return Err("文件已加密，请在 Excel 中去掉密码后再导入".into()),
            0x0022 => date1904 = u16_at(data, 0)? == 1,
            0x0085 => {
                // BOUNDSHEET：流内位置、可见性、类型（0 为工作表）、表名
                if data.get(5) == Some(&0) {
                    sheets.push((short_string(data, 6, false)?, u32_at(data, 0)? as usize));
                }
            }
            0x041E => {
                formats.insert(u16_at(data, 0)?, short_string(data, 2, true)?);
            }
            0x00E0 => xf_format.push(u16_at(data, 2)?),
            0x00FC => {
                let mut parts = vec![data.get(8..).ok_or("共享字符串表不完整")?];
                while let Some((0x003C, more)) = globals.peek() {
                    parts.push(more);
                    globals.next();
                }
                let unique = u32_at(data, 4)? as usize;
                let mut reader = Chunks { parts, part: 0, pos: 0 };
                sst.reserve(unique.min(1 << 20));
                for _ in 0..unique {
                    sst.push(reader.string()?);
                }
            }
            _ => {}
        }
    }
    Ok(WorkbookGlobals {
        sheets,
        sst,
        formats,
        xf_format,
        date1904,
    })
}

/// 读取 .xls 里的全部工作表（只取普通工作表，顺序与 Excel 中的标签相同）。
pub fn read(bytes: &[u8]) -> Result<Vec<Sheet>, String> {
    let stream = cfb_stream(bytes, &["Workbook"])?;
    let WorkbookGlobals {
        sheets,
        sst,
        formats,
        xf_format,
        date1904,
    } = workbook_globals(&stream)?;
    let mut out = vec![];
    for (name, at) in sheets {
        let mut cells: Vec<(usize, usize, String)> = vec![];
        let mut merged: Vec<[usize; 4]> = vec![];
        let mut pending_formula: Option<(usize, usize)> = None;
        let mut it = records(&stream, at);
        if !matches!(it.next(), Some((0x0809, _))) {
            return Err(format!("工作表 {name} 的位置错误"));
        }
        for (kind, data) in it {
            let cell = || -> Result<(usize, usize, u16), String> {
                Ok((u16_at(data, 0)? as usize, u16_at(data, 2)? as usize, u16_at(data, 4)?))
            };
            match kind {
                0x000A => break,
                0x0203 => {
                    let (r, c, x) = cell()?;
                    cells.push((
                        r,
                        c,
                        formatted_number(f64_at(data, 6)?, x, date1904, &xf_format, &formats),
                    ));
                }
                0x027E => {
                    let (r, c, x) = cell()?;
                    cells.push((
                        r,
                        c,
                        formatted_number(rk(u32_at(data, 6)?), x, date1904, &xf_format, &formats),
                    ));
                }
                0x00BD => {
                    let r = u16_at(data, 0)? as usize;
                    let first = u16_at(data, 2)? as usize;
                    let n = (data.len().saturating_sub(6)) / 6;
                    for k in 0..n {
                        let x = u16_at(data, 4 + 6 * k)?;
                        cells.push((
                            r,
                            first + k,
                            formatted_number(rk(u32_at(data, 6 + 6 * k)?), x, date1904, &xf_format, &formats),
                        ));
                    }
                }
                0x00FD => {
                    let (r, c, _) = cell()?;
                    let i = u32_at(data, 6)? as usize;
                    cells.push((r, c, sst.get(i).cloned().unwrap_or_default()));
                }
                0x0204 => {
                    let (r, c, _) = cell()?;
                    cells.push((r, c, short_string(data, 6, true)?));
                }
                0x0205 => {
                    let (r, c, _) = cell()?;
                    let (v, is_error) = (*data.get(6).ok_or("文件内容不完整")?, data.get(7) == Some(&1));
                    cells.push((
                        r,
                        c,
                        if is_error {
                            error_text(v)
                        } else if v != 0 {
                            "TRUE".into()
                        } else {
                            "FALSE".into()
                        },
                    ));
                }
                0x0006 => {
                    // FORMULA：缓存结果为双精度，或用 0xFFFF 标记的字符串/布尔/错误/空
                    let (r, c, x) = cell()?;
                    let res = data.get(6..14).ok_or("文件内容不完整")?;
                    if res[6] == 0xFF && res[7] == 0xFF {
                        match res[0] {
                            0 => pending_formula = Some((r, c)),
                            1 => cells.push((r, c, if res[2] != 0 { "TRUE".into() } else { "FALSE".into() })),
                            2 => cells.push((r, c, error_text(res[2]))),
                            _ => {}
                        }
                    } else {
                        cells.push((
                            r,
                            c,
                            formatted_number(f64_at(data, 6)?, x, date1904, &xf_format, &formats),
                        ));
                    }
                }
                0x0207 => {
                    if let Some((r, c)) = pending_formula.take() {
                        cells.push((r, c, short_string(data, 0, true)?));
                    }
                }
                0x00E5 => {
                    let n = u16_at(data, 0)? as usize;
                    for k in 0..n {
                        let at = 2 + 8 * k;
                        merged.push([
                            u16_at(data, at)? as usize,
                            u16_at(data, at + 2)? as usize,
                            u16_at(data, at + 4)? as usize,
                            u16_at(data, at + 6)? as usize,
                        ]);
                    }
                }
                _ => {}
            }
        }
        out.push(sheet(name, cells, merged)?);
    }
    if out.is_empty() {
        return Err("工作簿里没有工作表".into());
    }
    Ok(out)
}

fn formatted_number(
    value: f64,
    ixfe: u16,
    date1904: bool,
    xf_format: &[u16],
    formats: &std::collections::HashMap<u16, String>,
) -> String {
    let date = xf_format
        .get(ixfe as usize)
        .is_some_and(|&f| is_date_format(f, formats.get(&f).map(String::as_str)));
    if date {
        excel_date_text(value, date1904).unwrap_or_else(|| excel_number(value))
    } else {
        excel_number(value)
    }
}

fn sheet(name: String, mut cells: Vec<(usize, usize, String)>, merged: Vec<[usize; 4]>) -> Result<Sheet, String> {
    cells.retain(|c| !c.2.is_empty());
    let nrows = cells.iter().map(|c| c.0 + 1).max().unwrap_or(0);
    let ncols = cells.iter().map(|c| c.1 + 1).max().unwrap_or(0);
    crate::xlsx::check_sheet_size(nrows, ncols)?;
    let mut rows = vec![vec![String::new(); ncols]; nrows];
    for (r, c, text) in cells {
        rows[r][c] = text;
    }
    fill_merged(&mut rows, &merged);
    Ok(Sheet { name, rows })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rk_numbers_decode_integers_floats_and_hundredths() {
        // 整数 5（左移 2 位加整数标记）、1.0 的高 32 位、-123 / 100
        assert_eq!(rk((5 << 2) | 0x02), 5.0);
        assert_eq!(rk((1.0f64.to_bits() >> 32) as u32), 1.0);
        assert_eq!(rk((((-123i32) << 2) as u32) | 0x03), -1.23);
    }

    #[test]
    fn rejects_non_compound_files() {
        assert!(read(b"not an xls").unwrap_err().contains("不是有效"));
    }
}
