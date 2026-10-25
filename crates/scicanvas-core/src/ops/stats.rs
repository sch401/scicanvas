//! 行/列统计与频数结果。
use super::*;

impl Project {
    /// 行统计：对选中的列逐行求均值、标准差、标准误和 N（如平行样），写在最后一列右边。
    /// 均值为 Y 列、标准差为 Y 误差列，可直接画带误差棒的图。返回新列标识。
    pub fn row_stats(&mut self, book_id: u64, keys: &[String]) -> Result<Vec<String>, String> {
        if keys.len() < 2 {
            return Err("行统计至少需要选中两列".into());
        }
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        let cols: Vec<Vec<f64>> = keys
            .iter()
            .map(|k| {
                book.column(k)
                    .map(|c| c.values())
                    .ok_or_else(|| format!("找不到列 {k}"))
            })
            .collect::<Result<_, _>>()?;
        let units = book.column(&keys[0]).unwrap().units.clone();
        let names = format!("{}–{}", name_of(book, &keys[0]), name_of(book, keys.last().unwrap()));
        let rows = cols.iter().map(Vec::len).max().unwrap_or(0);
        let mut out: [Vec<String>; 4] = Default::default();
        for r in 0..rows {
            let v: Vec<f64> = cols
                .iter()
                .filter_map(|c| c.get(r).copied())
                .filter(|v| v.is_finite())
                .collect();
            let n = v.len();
            let mean = v.iter().sum::<f64>() / n as f64;
            let sd = if n > 1 {
                (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt()
            } else {
                f64::NAN
            };
            let cell = |x: f64| if x.is_finite() { fmt_num(x) } else { String::new() };
            out[0].push(cell(mean));
            out[1].push(cell(sd));
            out[2].push(cell(sd / (n as f64).sqrt()));
            out[3].push(n.to_string());
        }
        let defs = [
            ("均值", Role::Y, units.clone()),
            ("标准差", Role::YError, units.clone()),
            ("标准误", Role::Ignore, units),
            ("N", Role::Ignore, String::new()),
        ];
        let mut made = vec![];
        for ((label, role, units), mut data) in defs.into_iter().zip(out) {
            while data.last().is_some_and(|s| s.is_empty()) {
                data.pop();
            }
            let mut c = Column::new("", role);
            c.long_name = format!("行{label}");
            c.units = units;
            c.comment = format!("{names} 共 {} 列逐行统计", keys.len());
            c.data = data.into();
            made.push(self.insert_result_column(book_id, keys.last().unwrap(), c).unwrap());
        }
        Ok(made)
    }

    pub fn stats_on_columns(&mut self, src: &Source) -> Result<u64, String> {
        let columns = self.analysis_columns(src)?;
        let first = columns.first().ok_or("请选择数据曲线")?;
        let book_id = first.source.book;
        let rows = first.rows;
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        let src_name = book.name.clone();
        let stats: Vec<(String, Describe)> = columns
            .iter()
            .filter_map(|c| analysis::describe(&c.values).map(|d| (format!("{} · {}", c.source.col, c.name), d)))
            .collect();
        if stats.is_empty() {
            return Err("选中的数据里没有数值".into());
        }
        let id = self.alloc_id();
        let mut nb = Book::new(id, self.unique_name("列统计"));
        let note = match rows {
            Some((a, b)) => format!("来自 {} 第 {}–{} 行", src_name, a + 1, b + 1),
            None => format!("来自 {}", src_name),
        };
        type Getter = fn(&Describe) -> f64;
        let defs: [(&str, Getter); 10] = [
            ("N", |d| d.n as f64),
            ("均值", |d| d.mean),
            ("标准差", |d| d.sd),
            ("标准误", |d| d.se),
            ("最小值", |d| d.min),
            ("下四分位数", |d| d.q1),
            ("中位数", |d| d.median),
            ("上四分位数", |d| d.q3),
            ("最大值", |d| d.max),
            ("总和", |d| d.sum),
        ];
        let k = nb.push_column(Role::Ignore);
        let c = nb.columns.iter_mut().find(|c| c.key == k).unwrap();
        c.long_name = "数据列".into();
        c.comment = note.clone();
        c.data = stats.iter().map(|(n, _)| n.clone()).collect();
        for (label, get) in defs {
            let k = nb.push_column(Role::Y);
            let c = nb.columns.iter_mut().find(|c| c.key == k).unwrap();
            c.long_name = label.into();
            c.comment = note.clone();
            c.data = stats.iter().map(|(_, d)| fmt_num(get(d))).collect();
        }
        self.books.push(nb);
        self.move_project_item(id, self.folder_of(book_id))?;
        Ok(id)
    }

    /// 频数统计：每个选中的列生成一个结果工作簿，列出区间下限、中点、上限、频数、累积频数、
    /// 相对频率和累积相对频率；`bins` 为空时按 Sturges 规则，`range` 为空时用数据范围。
    pub fn frequency_counts(
        &mut self,
        src: &Source,
        bins: Option<usize>,
        range: Option<(f64, f64)>,
    ) -> Result<Vec<u64>, String> {
        let columns = self.analysis_columns(src)?;
        let mut results = vec![];
        let mut problems = vec![];
        for input in &columns {
            let book = self.book(input.source.book).unwrap();
            let c = book.column(&input.source.col).unwrap();
            let v = &input.values;
            let n = v.iter().filter(|x| x.is_finite()).count();
            let k = bins.unwrap_or_else(|| crate::stats::sturges_bins(n));
            match crate::stats::histogram(v, k, range) {
                Ok(b) => results.push((
                    input.source.book,
                    input.rows,
                    book.name.clone(),
                    input.name.clone(),
                    c.units.clone(),
                    b,
                )),
                Err(e) => problems.push(format!("{}：{e}", c.display_name())),
            }
        }
        if results.is_empty() {
            return Err(if problems.is_empty() {
                "选中的数据里没有数值".into()
            } else {
                problems.join("；")
            });
        }
        let mut ids = vec![];
        for (book_id, rows, src_name, name, units, bins) in results {
            let folder = self.folder_of(book_id);
            let id = self.alloc_id();
            let mut nb = Book::new(id, self.unique_name(&format!("{name} 频数")));
            let total: usize = bins.iter().map(|b| b.count).sum();
            let note = match rows {
                Some((a, b)) => format!("来自 {src_name} 的 {name}，第 {}–{} 行", a + 1, b + 1),
                None => format!("来自 {src_name} 的 {name}"),
            };
            let mut cumulative = 0;
            let cum: Vec<usize> = bins
                .iter()
                .map(|b| {
                    cumulative += b.count;
                    cumulative
                })
                .collect();
            let share = |c: usize| if total == 0 { f64::NAN } else { c as f64 / total as f64 };
            let columns: [(Role, &str, &str, Vec<f64>); 7] = [
                (Role::Ignore, "区间下限", &units, bins.iter().map(|b| b.lo).collect()),
                (
                    Role::X,
                    "区间中点",
                    &units,
                    bins.iter().map(|b| 0.5 * (b.lo + b.hi)).collect(),
                ),
                (Role::Ignore, "区间上限", &units, bins.iter().map(|b| b.hi).collect()),
                (Role::Y, "频数", "", bins.iter().map(|b| b.count as f64).collect()),
                (Role::Y, "累积频数", "", cum.iter().map(|c| *c as f64).collect()),
                (Role::Y, "相对频率", "", bins.iter().map(|b| share(b.count)).collect()),
                (Role::Y, "累积相对频率", "", cum.iter().map(|c| share(*c)).collect()),
            ];
            for (role, long_name, units, values) in columns {
                let k = nb.push_column(role);
                let c = nb.columns.iter_mut().find(|c| c.key == k).unwrap();
                c.long_name = long_name.into();
                c.units = units.into();
                c.comment = note.clone();
                c.data = values.into_iter().map(fmt_num).collect();
            }
            self.books.push(nb);
            self.move_project_item(id, folder)?;
            ids.push(id);
        }
        Ok(ids)
    }
}
