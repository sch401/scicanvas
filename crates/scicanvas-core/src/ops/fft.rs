//! FFT、IFFT、滤波、相关函数与寻峰。
use super::*;

impl Project {
    /// FFT 滤波：每条曲线的滤波结果写成新列；来源是图形时也加到图里。X 需要等间距。
    pub fn fft_filter(&mut self, src: &Source, kind: crate::fft::FilterKind) -> Result<Vec<String>, String> {
        let r = self.resolve(src)?;
        for item in &r.items {
            let book = self.book(item.book_id).unwrap();
            let (xs, _, _) = get_xy(book, item);
            if item.x.is_some() && xs.len() >= 2 {
                crate::fft::sample_interval(&xs).map_err(|e| format!("{}：{e}", name_of(book, &item.y)))?;
            }
        }
        let (r, made) = self.per_item_column(src, OpKind::FftFilter { kind }, |book, item, xs, ys| {
            let y = book.column(&item.y).unwrap();
            let dt = if item.x.is_some() {
                crate::fft::sample_interval(xs).unwrap_or(1.0)
            } else {
                1.0
            };
            (
                crate::fft::filter(ys, dt, kind).unwrap_or_else(|_| vec![f64::NAN; ys.len()]),
                format!(
                    "{} {}",
                    y.display_name(),
                    kind.label().split(' ').next().unwrap_or("滤波")
                ),
                y.units.clone(),
                format!("FFT 滤波：{}", kind.label()),
            )
        })?;
        if let Some(gid) = r.graph_id {
            for (item, key) in &made {
                self.bind_results(item, std::slice::from_ref(key), ResultLink::Layer(gid, r.layer_index))?;
            }
        }
        Ok(made.into_iter().map(|(_, k)| k).collect())
    }

    /// 自相关 / 互相关：选一列时求自相关，两列时求第二列相对第一列的互相关（滞后以数据点计）。
    /// 结果放进新工作簿（滞后、相关系数）并画成线图。返回 (工作簿, 图形) 编号。
    pub fn correlation_function(&mut self, book_id: u64, keys: &[String]) -> Result<(u64, u64), String> {
        self.correlation_function_with_visibility(book_id, keys, false)
    }

    pub fn correlation_function_with_visibility(
        &mut self,
        book_id: u64,
        keys: &[String],
        visible_only: bool,
    ) -> Result<(u64, u64), String> {
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        let (a, b) = match keys {
            [a] => (a, a),
            [a, b] => (a, b),
            _ => return Err("请选中一列（自相关）或两列（互相关）".into()),
        };
        let col = |k: &str| book.column(k).ok_or_else(|| format!("找不到列 {k}"));
        let (ca, cb) = (col(a)?, col(b)?);
        let (va, vb) = if visible_only {
            (book.visible_values(ca), book.visible_values(cb))
        } else {
            (ca.values(), cb.values())
        };
        let (lags, r) = crate::stats::cross_correlation(&va, &vb)?;
        let auto = a == b;
        let title = if auto {
            format!("{} 自相关", ca.display_name())
        } else {
            format!("{} 与 {} 互相关", ca.display_name(), cb.display_name())
        };
        let folder = self.folder_of(book_id);
        let id = self.alloc_id();
        let mut nb = Book::new(id, self.unique_name(&title));
        let mut ykey = String::new();
        for (role, name, values) in [
            (Role::X, "滞后", lags.iter().map(|&k| k as f64).collect::<Vec<_>>()),
            (Role::Y, "相关系数", r),
        ] {
            let k = nb.push_column(role);
            let c = nb.columns.iter_mut().find(|c| c.key == k).unwrap();
            c.long_name = name.into();
            c.comment = format!("{title}，{} 点，归一化", va.len());
            c.data = values.iter().map(|v| fmt_num(*v)).collect();
            ykey = k;
        }
        self.books.push(nb);
        self.move_project_item(id, folder)?;
        let (gid, _) = self.plot_from_selection(id, &[ykey], PlotType::Line)?;
        Ok((id, gid))
    }

    /// 对每条曲线做 FFT：各生成一个频谱工作簿（频率、实部、虚部、幅值、相位、功率）和一张幅值–频率图，
    /// 放在源工作簿所在的文件夹。X 必须等间距；没有 X 列时采样间隔按 1。返回 (工作簿, 图形) 编号。
    pub fn fft(&mut self, src: &Source, window: crate::fft::Window) -> Result<Vec<(u64, u64)>, String> {
        let r = self.resolve(src)?;
        let mut made = vec![];
        let mut problems = vec![];
        for item in &r.items {
            let book = self.book(item.book_id).unwrap();
            let name = r.name(book, item);
            let (xs, ys, _) = get_xy(book, item);
            let x_units = item
                .x
                .as_deref()
                .and_then(|k| book.column(k))
                .map(|c| c.units.clone())
                .unwrap_or_default();
            let y_units = book.column(&item.y).map(|c| c.units.clone()).unwrap_or_default();
            let dt = if item.x.is_some() {
                crate::fft::sample_interval(&xs)
            } else {
                Ok(1.0)
            };
            let spec = match dt.and_then(|dt| crate::fft::spectrum(&ys, dt, window)) {
                Ok(s) => s,
                Err(e) => {
                    problems.push(format!("{name}：{e}"));
                    continue;
                }
            };
            let f_units = match x_units.as_str() {
                "s" => "Hz".to_string(),
                "" => String::new(),
                u => format!("1/{u}"),
            };
            let p_units = if y_units.is_empty() {
                String::new()
            } else {
                format!("{y_units}²")
            };
            let id = self.alloc_id();
            let mut nb = Book::new(id, self.unique_name(&format!("{name} FFT")));
            let note = format!("{} 的 FFT，{} 点，窗函数 {}", name, ys.len(), window.label());
            let columns: [(Role, &str, &str, &[f64]); 6] = [
                (Role::X, "频率", &f_units, &spec.freq),
                (Role::Y, "实部", "", &spec.re),
                (Role::Y, "虚部", "", &spec.im),
                (Role::Y, "幅值", &y_units, &spec.amplitude),
                (Role::Y, "相位", "°", &spec.phase_deg),
                (Role::Y, "功率", &p_units, &spec.power),
            ];
            let mut amp_key = String::new();
            for (role, long_name, units, values) in columns {
                let k = nb.push_column(role);
                let c = nb.columns.iter_mut().find(|c| c.key == k).unwrap();
                c.long_name = long_name.into();
                c.units = units.into();
                c.comment = note.clone();
                c.data = values.iter().map(|v| fmt_num(*v)).collect();
                if long_name == "幅值" {
                    amp_key = k;
                }
            }
            self.books.push(nb);
            self.move_project_item(id, self.folder_of(item.book_id))?;
            let (gid, _) = self.plot_from_selection(id, &[amp_key], PlotType::Line)?;
            made.push((id, gid));
        }
        if made.is_empty() {
            return Err(problems.join("；"));
        }
        Ok(made)
    }

    /// 逆傅里叶变换：`re_key`/`im_key` 为同一工作簿里逐行对齐的实部、虚部（没有虚部按 0）。
    /// 实部列对应的 X（频率）给出频率间隔 df，时间间隔 dt = 1/(N·df)；没有 X 时 dt = 1。
    /// 生成新工作簿（时间、实部，双边谱另有虚部）和实部曲线图，放在源工作簿所在文件夹。返回 (工作簿, 图形)。
    pub fn ifft(
        &mut self,
        book_id: u64,
        re_key: &str,
        im_key: Option<&str>,
        input: crate::fft::InverseInput,
    ) -> Result<(u64, u64), String> {
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        let re_col = book.column(re_key).ok_or("找不到实部列")?;
        // 去掉末尾空行；中间有缺失值时报错
        let finite = |v: Vec<f64>| -> Result<Vec<f64>, String> {
            let end = v.iter().rposition(|x| x.is_finite()).map_or(0, |i| i + 1);
            let v = v[..end].to_vec();
            if v.iter().any(|x| !x.is_finite()) {
                return Err("频谱里有缺失值或非数值".into());
            }
            Ok(v)
        };
        let re = finite(re_col.values())?;
        let im = match im_key {
            Some(k) => finite(book.column(k).ok_or("找不到虚部列")?.values())?,
            None => vec![],
        };
        let m = re.len().max(im.len());
        if m < 2 {
            return Err("IFFT 至少需要 2 个频点".into());
        }
        let (mut re, mut im) = (re, im);
        re.resize(m, 0.0);
        im.resize(m, 0.0);
        let n = input.points(m);
        let x_key = x_for(book, re_col, &[]);
        let x_col = x_key.as_deref().and_then(|k| book.column(k));
        let dt = match x_col {
            Some(c) => {
                let f = c.values();
                let df = f.get(1).zip(f.first()).map_or(f64::NAN, |(b, a)| b - a);
                if !(df.is_finite() && df > 0.0) {
                    return Err("频率列前两行必须递增，用来确定频率间隔".into());
                }
                1.0 / (n as f64 * df)
            }
            None => 1.0,
        };
        let t_units = match x_col.map(|c| c.units.as_str()).unwrap_or("") {
            "Hz" => "s".to_string(),
            u => u.strip_prefix("1/").unwrap_or("").to_string(),
        };
        let (out_re, out_im) = match input {
            crate::fft::InverseInput::OneSided { .. } => (crate::fft::inverse_real(&re, &im, n), None),
            crate::fft::InverseInput::TwoSided => {
                let (r, i) = crate::fft::inverse(&re, &im);
                (r, Some(i))
            }
        };
        let source = re_col.display_name().to_string();
        let note = match input {
            crate::fft::InverseInput::OneSided { .. } => format!("{source} 的 IFFT（单边谱），{n} 点"),
            crate::fft::InverseInput::TwoSided => format!("{source} 的 IFFT（双边谱），{n} 点"),
        };
        let name = self.unique_name(&format!("{} IFFT", book.name));
        let folder = self.folder_of(book_id);
        let id = self.alloc_id();
        let mut nb = Book::new(id, name);
        let times: Vec<f64> = (0..n).map(|i| i as f64 * dt).collect();
        let mut columns: Vec<(Role, &str, &str, &[f64])> =
            vec![(Role::X, "时间", &t_units, &times), (Role::Y, "实部", "", &out_re)];
        if let Some(i) = &out_im {
            columns.push((Role::Y, "虚部", "", i));
        }
        let mut re_out_key = String::new();
        for (role, long_name, units, values) in columns {
            let k = nb.push_column(role);
            let c = nb.columns.iter_mut().find(|c| c.key == k).unwrap();
            c.long_name = long_name.into();
            c.units = units.into();
            c.comment = note.clone();
            c.data = values.iter().map(|v| fmt_num(*v)).collect();
            if long_name == "实部" {
                re_out_key = k;
            }
        }
        self.books.push(nb);
        self.move_project_item(id, folder)?;
        let (gid, _) = self.plot_from_selection(id, &[re_out_key], PlotType::Line)?;
        Ok((id, gid))
    }

    /// 寻峰与峰积分：每条曲线生成一个峰表工作簿（峰位、峰高、突出度、半峰宽、面积、质心、积分起止 X），
    /// 放在源工作簿所在的文件夹。`min_prominence` 是相对 Y 范围的比例（0.05 即 5%）。
    /// X 需要单调；递减的 X（如红外光谱的波数）会先反转。返回新工作簿编号。
    pub fn find_peaks(&mut self, src: &Source, min_prominence: f64) -> Result<Vec<u64>, String> {
        let r = self.resolve(src)?;
        let mut made = vec![];
        let mut problems = vec![];
        for item in &r.items {
            let book = self.book(item.book_id).unwrap();
            let name = r.name(book, item);
            let (mut xs, mut ys, _) = get_xy(book, item);
            let unit = |k: Option<&str>| {
                k.and_then(|k| book.column(k))
                    .map(|c| c.units.clone())
                    .unwrap_or_default()
            };
            let (x_units, y_units) = (unit(item.x.as_deref()), unit(Some(&item.y)));
            if xs.len() < 3 {
                problems.push(format!("{name}：数据点太少"));
                continue;
            }
            if xs.windows(2).all(|w| w[1] < w[0]) {
                xs.reverse();
                ys.reverse();
            } else if !xs.windows(2).all(|w| w[1] > w[0]) {
                problems.push(format!("{name}：X 需要单调递增或递减"));
                continue;
            }
            let (lo, hi) = ys
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), y| (a.min(*y), b.max(*y)));
            let threshold = min_prominence * (hi - lo);
            let peaks = crate::peaks::find_peaks(&xs, &ys, threshold);
            if peaks.is_empty() {
                problems.push(format!("{name}：没有突出度达到 {} 的峰", fmt_num(threshold)));
                continue;
            }
            let area_units = match (x_units.as_str(), y_units.as_str()) {
                ("", "") => String::new(),
                ("", u) | (u, "") => u.to_string(),
                (a, b) => format!("{b}·{a}"),
            };
            let id = self.alloc_id();
            let mut nb = Book::new(id, self.unique_name(&format!("{name} 峰")));
            let note = format!(
                "{} 的峰，最小突出度 {}（Y 范围的 {}%）",
                name,
                fmt_num(threshold),
                round_sig(min_prominence * 100.0, 4)
            );
            type Getter = fn(&crate::peaks::Peak, &[f64]) -> f64;
            let columns: [(Role, &str, &str, Getter); 8] = [
                (Role::X, "峰位", &x_units, |p, _| p.x),
                (Role::Y, "峰高", &y_units, |p, _| p.height),
                (Role::Y, "突出度", &y_units, |p, _| p.prominence),
                (Role::Y, "半峰宽", &x_units, |p, _| p.fwhm),
                (Role::Y, "面积", &area_units, |p, _| p.area),
                (Role::Y, "质心", &x_units, |p, _| p.centroid),
                (Role::Y, "积分起点", &x_units, |p, xs| xs[p.start]),
                (Role::Y, "积分终点", &x_units, |p, xs| xs[p.end]),
            ];
            for (role, long_name, units, get) in columns {
                let k = nb.push_column(role);
                let c = nb.columns.iter_mut().find(|c| c.key == k).unwrap();
                c.long_name = long_name.into();
                c.units = units.into();
                c.comment = note.clone();
                c.data = peaks.iter().map(|p| fmt_num(get(p, &xs))).collect();
            }
            self.books.push(nb);
            self.move_project_item(id, self.folder_of(item.book_id))?;
            made.push(id);
        }
        if made.is_empty() {
            return Err(problems.join("；"));
        }
        Ok(made)
    }
}
