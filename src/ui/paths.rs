#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SvgPoint {
    pub x: f64,
    pub y: f64,
}

impl SvgPoint {
    pub fn new(x: f64, y: f64) -> Self {
        SvgPoint { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCommand {
    MoveTo(SvgPoint),
    LineTo(SvgPoint),
    QuadTo(SvgPoint, SvgPoint),
    CubicTo(SvgPoint, SvgPoint, SvgPoint),
    Close,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SvgPath {
    pub commands: Vec<PathCommand>,
}

impl SvgPath {
    pub fn new(commands: Vec<PathCommand>) -> Self {
        SvgPath { commands }
    }

    pub fn closed(mut self, close: bool) -> Self {
        if close && !matches!(self.commands.last(), Some(PathCommand::Close)) {
            self.commands.push(PathCommand::Close);
        }
        self
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn start(&self) -> Option<SvgPoint> {
        self.commands.iter().find_map(|command| match command {
            PathCommand::MoveTo(point) => Some(*point),
            _ => None,
        })
    }

    pub fn end(&self) -> Option<SvgPoint> {
        self.commands
            .iter()
            .rev()
            .find_map(|command| match command {
                PathCommand::MoveTo(point) | PathCommand::LineTo(point) => Some(*point),
                PathCommand::QuadTo(_, point) => Some(*point),
                PathCommand::CubicTo(_, _, point) => Some(*point),
                PathCommand::Close => None,
            })
    }

    pub fn translate(mut self, dx: f64, dy: f64) -> Self {
        for command in self.commands.iter_mut() {
            match command {
                PathCommand::MoveTo(point) | PathCommand::LineTo(point) => {
                    point.x += dx;
                    point.y += dy;
                }
                PathCommand::QuadTo(control, point) => {
                    control.x += dx;
                    control.y += dy;
                    point.x += dx;
                    point.y += dy;
                }
                PathCommand::CubicTo(a, b, point) => {
                    a.x += dx;
                    a.y += dy;
                    b.x += dx;
                    b.y += dy;
                    point.x += dx;
                    point.y += dy;
                }
                PathCommand::Close => {}
            }
        }
        self
    }

    pub fn scale(mut self, factor: f64) -> Self {
        for command in self.commands.iter_mut() {
            match command {
                PathCommand::MoveTo(point) | PathCommand::LineTo(point) => {
                    point.x *= factor;
                    point.y *= factor;
                }
                PathCommand::QuadTo(control, point) => {
                    control.x *= factor;
                    control.y *= factor;
                    point.x *= factor;
                    point.y *= factor;
                }
                PathCommand::CubicTo(a, b, point) => {
                    a.x *= factor;
                    a.y *= factor;
                    b.x *= factor;
                    b.y *= factor;
                    point.x *= factor;
                    point.y *= factor;
                }
                PathCommand::Close => {}
            }
        }
        self
    }

    pub fn sample(&self, steps: usize) -> Vec<SvgPoint> {
        let mut points = Vec::new();
        let mut current = SvgPoint::new(0.0, 0.0);

        for command in &self.commands {
            match *command {
                PathCommand::MoveTo(point) => {
                    current = point;
                    points.push(point);
                }
                PathCommand::LineTo(point) => {
                    current = point;
                    points.push(point);
                }
                PathCommand::QuadTo(control, point) => {
                    let steps = steps.max(2);
                    for index in 1..=steps {
                        let t = index as f64 / steps as f64;
                        points.push(quad_at(current, control, point, t));
                    }
                    current = point;
                }
                PathCommand::CubicTo(a, b, point) => {
                    let steps = steps.max(2);
                    for index in 1..=steps {
                        let t = index as f64 / steps as f64;
                        points.push(cubic_at(current, a, b, point, t));
                    }
                    current = point;
                }
                PathCommand::Close => {}
            }
        }

        points
    }

    pub fn to_string(&self) -> String {
        let mut out = String::new();
        for command in &self.commands {
            match *command {
                PathCommand::MoveTo(point) => {
                    out.push_str(&format!("M{},{} ", trim(point.x), trim(point.y)));
                }
                PathCommand::LineTo(point) => {
                    out.push_str(&format!("L{},{} ", trim(point.x), trim(point.y)));
                }
                PathCommand::QuadTo(control, point) => {
                    out.push_str(&format!(
                        "Q{},{} {},{} ",
                        trim(control.x),
                        trim(control.y),
                        trim(point.x),
                        trim(point.y)
                    ));
                }
                PathCommand::CubicTo(a, b, point) => {
                    out.push_str(&format!(
                        "C{},{} {},{} {},{} ",
                        trim(a.x),
                        trim(a.y),
                        trim(b.x),
                        trim(b.y),
                        trim(point.x),
                        trim(point.y)
                    ));
                }
                PathCommand::Close => out.push_str("Z "),
            }
        }
        out.trim_end().to_string()
    }
}

fn trim(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    if rounded.fract() == 0.0 {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded}")
    }
}

fn quad_at(p0: SvgPoint, p1: SvgPoint, p2: SvgPoint, t: f64) -> SvgPoint {
    let mt = 1.0 - t;
    SvgPoint::new(
        mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x,
        mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y,
    )
}

fn cubic_at(p0: SvgPoint, p1: SvgPoint, p2: SvgPoint, p3: SvgPoint, t: f64) -> SvgPoint {
    let mt = 1.0 - t;
    let a = mt * mt * mt;
    let b = 3.0 * mt * mt * t;
    let c = 3.0 * mt * t * t;
    let d = t * t * t;
    SvgPoint::new(
        a * p0.x + b * p1.x + c * p2.x + d * p3.x,
        a * p0.y + b * p1.y + c * p2.y + d * p3.y,
    )
}

pub fn parse_svg_path(input: &str) -> SvgPath {
    let mut commands = Vec::new();
    let mut numbers: Vec<f64> = Vec::new();
    let mut current_command = 'M';
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_ascii_alphabetic() {
            if current_command != ch
                || matches!(current_command, 'M' | 'm' | 'L' | 'l' | 'C' | 'c' | 'Q' | 'q')
            {
                if !numbers.is_empty() {
                    flush_command(current_command, &mut numbers, &mut commands);
                }
            }
            current_command = ch;
            continue;
        }

        if ch.is_ascii_digit() || ch == '-' || ch == '+' || ch == '.' || ch == 'e' || ch == 'E' {
            let mut buffer = String::new();
            buffer.push(ch);
            while let Some(next) = chars.peek() {
                if next.is_ascii_digit()
                    || *next == '.'
                    || *next == '-'
                    || *next == '+'
                    || *next == 'e'
                    || *next == 'E'
                {
                    buffer.push(*next);
                    chars.next();
                } else {
                    break;
                }
            }
            if let Ok(value) = buffer.parse::<f64>() {
                numbers.push(value);
            }
            continue;
        }

        if ch == ',' || ch.is_whitespace() {
            continue;
        }

        if ch == 'Z' || ch == 'z' {
            commands.push(PathCommand::Close);
        }
    }

    if !numbers.is_empty() {
        flush_command(current_command, &mut numbers, &mut commands);
    }

    SvgPath::new(commands)
}

fn flush_command(command: char, numbers: &mut Vec<f64>, commands: &mut Vec<PathCommand>) {
    let args: Vec<f64> = std::mem::take(numbers);

    match command {
        'M' | 'm' => {
            let mut index = 0;
            let mut first = true;
            let mut previous = SvgPoint::new(0.0, 0.0);
            while index + 1 < args.len() {
                let mut x = args[index];
                let mut y = args[index + 1];
                if command == 'm' {
                    x += previous.x;
                    y += previous.y;
                }
                let point = SvgPoint::new(x, y);
                if first {
                    commands.push(PathCommand::MoveTo(point));
                    first = false;
                } else {
                    commands.push(PathCommand::LineTo(point));
                }
                previous = point;
                index += 2;
            }
        }
        'L' | 'l' => {
            let mut index = 0;
            let mut previous = last_point(commands);
            while index + 1 < args.len() {
                let mut x = args[index];
                let mut y = args[index + 1];
                if command == 'l' {
                    x += previous.x;
                    y += previous.y;
                }
                let point = SvgPoint::new(x, y);
                commands.push(PathCommand::LineTo(point));
                previous = point;
                index += 2;
            }
        }
        'C' | 'c' => {
            let mut index = 0;
            let mut previous = last_point(commands);
            while index + 5 < args.len() {
                let mut a = SvgPoint::new(args[index], args[index + 1]);
                let mut b = SvgPoint::new(args[index + 2], args[index + 3]);
                let mut p = SvgPoint::new(args[index + 4], args[index + 5]);
                if command == 'c' {
                    a.x += previous.x;
                    a.y += previous.y;
                    b.x += previous.x;
                    b.y += previous.y;
                    p.x += previous.x;
                    p.y += previous.y;
                }
                commands.push(PathCommand::CubicTo(a, b, p));
                previous = p;
                index += 6;
            }
        }
        'Q' | 'q' => {
            let mut index = 0;
            let mut previous = last_point(commands);
            while index + 3 < args.len() {
                let mut control = SvgPoint::new(args[index], args[index + 1]);
                let mut p = SvgPoint::new(args[index + 2], args[index + 3]);
                if command == 'q' {
                    control.x += previous.x;
                    control.y += previous.y;
                    p.x += previous.x;
                    p.y += previous.y;
                }
                commands.push(PathCommand::QuadTo(control, p));
                previous = p;
                index += 4;
            }
        }
        _ => {}
    }
}

fn last_point(commands: &[PathCommand]) -> SvgPoint {
    for command in commands.iter().rev() {
        match *command {
            PathCommand::MoveTo(point) | PathCommand::LineTo(point) => return point,
            PathCommand::QuadTo(_, point) => return point,
            PathCommand::CubicTo(_, _, point) => return point,
            PathCommand::Close => {}
        }
    }

    SvgPoint::new(0.0, 0.0)
}

pub fn polyline(points: &[SvgPoint]) -> SvgPath {
    if points.is_empty() {
        return SvgPath::default();
    }

    let mut commands = vec![PathCommand::MoveTo(points[0])];
    for point in points.iter().skip(1) {
        commands.push(PathCommand::LineTo(*point));
    }
    SvgPath::new(commands)
}
