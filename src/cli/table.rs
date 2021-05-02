use num_traits::FromPrimitive;
use std::fmt;

use crate::lib::{
    summary::Summary,
    entry::Category,
};

pub struct Table<'d> {
    data: &'d [Summary],
}

struct BoxFmt {
    width: usize,
    text: String,
}

struct ColFmt {
    width: usize,
    label: BoxFmt,
    boxes: Vec<BoxFmt>,
}

struct GridFmt {
    labels: ColFmt,
    columns: Vec<ColFmt>,
}

impl<'d> Table<'d> {
    pub fn from(data: &'d [Summary]) -> Self {
        Self { data }
    }

    fn to_formatter(&self) -> GridFmt {
        let columns = (0..Category::COUNT)
            .map(|i| Category::from_usize(i).unwrap())
            .collect::<Vec<_>>();
        let cols = columns
            .iter()
            .map(|c| BoxFmt::category(*c))
            .map(|b| ColFmt::with_label(b))
            .collect::<Vec<_>>();
        let mut grid = GridFmt::with_columns(cols);
        for sum in self.data {
            grid.push_line(BoxFmt::period(sum.period()), sum.amounts().iter().map(|f| BoxFmt::amount(*f)).collect::<Vec<_>>());
        }
        grid
    }
}

impl BoxFmt {
    fn from(text: String) -> Self {
        let width = text.len();
        Self {
            text,
            width,
        }
    }

    fn amount(a: crate::lib::entry::Amount) -> Self {
        if a.nonzero() {
            let text = format!("{}", a);
            let width = text.len() - 2;
            Self {
                text,
                width,
            }
        } else {
            Self::from(String::new())
        }
    }

    fn period(p: crate::lib::date::Period) -> Self {
        Self::from(format!("{}", p))
    }

    fn category(c: crate::lib::entry::Category) -> Self {
        Self::from(format!("{:?}", c))
    }
}

impl ColFmt {
    fn with_label(label: BoxFmt) -> Self {
        Self {
            width: label.width + 3,
            label,
            boxes: Vec::new(),
        }
    }

    fn push(&mut self, b: BoxFmt) {
        self.width = self.width.max(b.width + 3);
        self.boxes.push(b);
    }
}

impl GridFmt {
    fn with_columns(columns: Vec<ColFmt>) -> Self {
        Self {
            labels: ColFmt::with_label(BoxFmt::from(String::new())),
            columns,
        }
    }

    fn push_line(&mut self, label: BoxFmt, boxes: Vec<BoxFmt>) {
        self.labels.push(label);
        for (i, b) in boxes.into_iter().enumerate() {
            self.columns[i].push(b);
        }
    }
}


impl fmt::Display for Table<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_formatter())
    }
}

impl fmt::Display for GridFmt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.labels.write_label(f)?;
        for c in &self.columns {
            c.write_label(f)?;
        }
        writeln!(f)?;
        for idx in 0..self.labels.len() {
            self.labels.write_item(f, idx, false)?;
            for c in &self.columns {
                c.write_item(f, idx, true)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl ColFmt {
    fn write_label(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.label.write(f, self.width, true)
    }

    fn write_item(&self, f: &mut fmt::Formatter, idx: usize, right: bool) -> fmt::Result {
        self.boxes[idx].write(f, self.width, right)
    }

    fn len(&self) -> usize {
        self.boxes.len()
    }
}

const PADDING: &str = "                                         ";
impl BoxFmt {
    fn write(&self, f: &mut fmt::Formatter, width: usize, right: bool) -> fmt::Result {
        if right {
            write!(f, " {}{} |", &PADDING[..width.saturating_sub(self.width)], self.text)
        } else {
            write!(f, " {}{} |", self.text, &PADDING[..width.saturating_sub(self.width)])
        }
    }
}
