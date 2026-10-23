//! 界面测试共用驱动；保留各场景自己的绘制入口、时钟、尺寸与事件顺序。
use eframe::egui::{self, Context, Event, FullOutput, Modifiers, Pos2, Rect, Vec2};

pub(crate) struct Harness<F = ()> {
    pub ctx: Context,
    pub time: f64,
    pub modifiers: Modifiers,
    pub size: Option<Vec2>,
    step: Option<f64>,
    draw: F,
}

impl<F> Harness<F> {
    pub fn new(ctx: Context, size: Option<Vec2>, draw: F) -> Self {
        Self {
            ctx,
            size,
            draw,
            time: 0.0,
            modifiers: Modifiers::NONE,
            step: None,
        }
    }

    pub fn with_step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    fn input(&mut self, events: Vec<Event>) -> egui::RawInput {
        let time = self.step.map(|step| {
            self.time += step;
            self.time
        });
        egui::RawInput {
            time,
            modifiers: self.modifiers,
            screen_rect: self.size.map(|size| Rect::from_min_size(Pos2::ZERO, size)),
            events,
            ..Default::default()
        }
    }

    /// 固定的场景入口，返回这一帧的实际界面输出。
    pub fn frame<A, R>(&mut self, state: &mut A, events: Vec<Event>) -> FullOutput
    where
        F: FnMut(&mut A, &Context) -> R,
    {
        self.frame_result(state, events).0
    }

    /// 少数场景还需返回绘制所得的窗口范围或命令标识。
    pub fn frame_value<A, R>(&mut self, state: &mut A, events: Vec<Event>) -> R
    where
        F: FnMut(&mut A, &Context) -> R,
    {
        self.frame_result(state, events).1
    }

    fn frame_result<A, R>(&mut self, state: &mut A, events: Vec<Event>) -> (FullOutput, R)
    where
        F: FnMut(&mut A, &Context) -> R,
    {
        let input = self.input(events);
        render(&self.ctx, input, |ctx| (self.draw)(state, ctx))
    }

    /// 组件适配器可传入临时入口，仍沿用同一套输入与时间处理。
    pub fn run<R>(&mut self, events: Vec<Event>, draw: impl FnMut(&Context) -> R) -> (FullOutput, R) {
        let input = self.input(events);
        render(&self.ctx, input, draw)
    }

    pub fn click<A, R>(&mut self, state: &mut A, pos: Pos2) -> FullOutput
    where
        F: FnMut(&mut A, &Context) -> R,
    {
        self.frame(state, vec![Event::PointerMoved(pos)]);
        self.frame(state, vec![button(pos, true)]);
        self.frame(state, vec![button(pos, false)])
    }

    /// 从已有输出定位，避免为查找控件额外推进一帧。
    pub fn click_text<A, R>(&mut self, state: &mut A, output: &FullOutput, label: &str) -> FullOutput
    where
        F: FnMut(&mut A, &Context) -> R,
    {
        let pos = find_text(output, label).unwrap_or_else(|| panic!("找不到控件：{label}"));
        self.click(state, pos + egui::vec2(5.0, 5.0))
    }

    pub fn drag(&mut self, start: Pos2, via: Pos2, end: Pos2, mut draw: impl FnMut(&Context)) -> FullOutput {
        self.run(vec![Event::PointerMoved(start)], &mut draw);
        self.run(vec![button(start, true)], &mut draw);
        self.run(vec![Event::PointerMoved(via)], &mut draw);
        self.run(vec![Event::PointerMoved(end)], &mut draw);
        self.run(vec![button(end, false)], draw).0
    }

    pub fn key(&mut self, key: egui::Key, mut draw: impl FnMut(&Context)) -> FullOutput {
        self.run(vec![key_event(key, true)], &mut draw);
        self.run(vec![key_event(key, false)], draw).0
    }
}

fn render<R>(ctx: &Context, input: egui::RawInput, mut draw: impl FnMut(&Context) -> R) -> (FullOutput, R) {
    let mut value = None;
    let output = ctx.run(input, |ctx| value = Some(draw(ctx)));
    (output, value.expect("界面帧应执行绘制入口"))
}

pub(crate) fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn key_event(key: egui::Key, pressed: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

fn text_shapes(output: &FullOutput) -> impl Iterator<Item = &egui::epaint::TextShape> {
    output.shapes.iter().filter_map(|shape| match &shape.shape {
        egui::Shape::Text(text) => Some(text),
        _ => None,
    })
}

pub(crate) fn find_text(output: &FullOutput, label: &str) -> Option<Pos2> {
    text_shapes(output)
        .find(|text| text.galley.job.text == label)
        .map(|text| text.pos)
}

pub(crate) fn text_positions(output: &FullOutput, label: &str) -> Vec<Pos2> {
    text_shapes(output)
        .filter(|text| text.galley.job.text == label)
        .map(|text| text.pos)
        .collect()
}

pub(crate) fn text_rect(output: &FullOutput, label: &str) -> Option<Rect> {
    text_shapes(output)
        .find(|text| text.galley.job.text == label)
        .map(|text| text.galley.rect.translate(text.pos.to_vec2()))
}
