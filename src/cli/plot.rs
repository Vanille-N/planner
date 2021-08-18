use crate::lib::{
    date::{Date, Period},
    entry::Amount,
    summary::Summary,
};

/// In charge of the public interface to the plotting devices
pub struct Plotter<'d> {
    data: &'d [Summary],
}

/// Recommended usage:
/// ```
/// let mut cal: Calendar = unimplemented!();
/// let lst: Vec<entry> = unimplemented!();
/// cal.register(&lst);
/// Plotter::from(cal.contents()).print_cumulative_plot()
/// ```
impl<'d> Plotter<'d> {
    /// Wrap data to plot
    pub fn from(data: &'d [Summary]) -> Self {
        Self { data }
    }

    /// Launch plotting
    pub fn print_cumulative_plot(&self, title: &str) {
        self.cumulative_plot()
            .to_range_group_drawer()
            .render(&format!("{}.svg", title))
    }

    /// Accumulate contained data into cumulative plot
    fn cumulative_plot(&self) -> Plot<Period, CumulativeEntry<Amount>> {
        let mut plot = Plot::new();
        for sum in self.data {
            plot.push(sum.period(), CumulativeEntry::cumul(sum.amounts().to_vec()));
        }
        plot
    }
}

/// Generic plotter
#[derive(Debug)]
pub struct Plot<X, Y> {
    /// (X, Y) generic descriptor of how to display the data
    data: Vec<(X, Y)>,
}

impl<X, Y> Plot<X, Y> {
    /// Empty plotter
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Add item
    fn push(&mut self, x: X, y: Y) {
        self.data.push((x, y));
    }
}

/// Describes how to format a collection of same-abscissa points
#[derive(Debug)]
struct CumulativeEntry<Y> {
    points: Vec<Y>,
}

impl<Y> CumulativeEntry<Y>
where
    Y: std::ops::AddAssign + Clone,
{
    /// Calculate cumulative values
    fn cumul(mut points: Vec<Y>) -> Self {
        for i in 1..points.len() {
            let prev = points[i - 1].clone();
            points[i] += prev;
        }
        Self { points }
    }
}

/// A plot item that can be converted to a value
/// (e.g. an amount or a date)
pub trait Scalar {
    fn to_scalar(&self) -> i64;
}

/// A plot item that can be converted to a pair of values
/// (e.g. a period)
pub trait ScalarRange {
    fn to_range(&self) -> (i64, i64);
}

/// A plot item that can be converted to a group of values
/// (e.g. a sequence of cumulative entries)
pub trait ScalarGroup {
    fn to_group(&self) -> Vec<i64>;
}

impl Scalar for Amount {
    fn to_scalar(&self) -> i64 {
        self.0 as i64
    }
}

impl Scalar for Date {
    fn to_scalar(&self) -> i64 {
        self.index() as i64
    }
}

impl ScalarRange for Period {
    fn to_range(&self) -> (i64, i64) {
        (self.0.to_scalar(), self.1.to_scalar())
    }
}

impl<T> ScalarRange for (T, T)
where
    T: Scalar,
{
    fn to_range(&self) -> (i64, i64) {
        (self.0.to_scalar(), self.1.to_scalar())
    }
}

impl<Y> ScalarGroup for CumulativeEntry<Y>
where
    Y: Scalar,
{
    fn to_group(&self) -> Vec<i64> {
        self.points
            .iter()
            .map(|p| p.to_scalar())
            .collect::<Vec<_>>()
    }
}

impl<X, Y> Plot<X, Y>
where
    X: ScalarRange,
    Y: ScalarGroup,
{
    fn to_range_group_drawer(&self) -> RangeGroupDrawer {
        RangeGroupDrawer {
            points: self
                .data
                .iter()
                .map(|(x, y)| (x.to_range(), y.to_group()))
                .collect::<Vec<_>>(),
        }
    }
}

struct Dimensions {
    min_x: i64,
    min_y: i64,
    max_x: i64,
    max_y: i64,
    delta_x: i64,
    delta_y: i64,
    view_height: f64,
    view_width: f64,
    stroke_width: f64,
    margin: f64,
    atomic_width: f64,
}

impl Dimensions {
    fn new() -> Self {
        Self {
            min_x: i64::MAX,
            min_y: i64::MAX,
            max_x: i64::MIN,
            max_y: i64::MIN,
            delta_y: 0,
            delta_x: 0,
            stroke_width: 2.0,
            margin: 20.0,
            view_height: 700.0,
            view_width: 1000.0,
            atomic_width: 0.0,
        }
        .update()
    }

    fn update(mut self) -> Self {
        self.delta_y = self.max_y.saturating_sub(self.min_y);
        self.delta_x = self.max_x.saturating_sub(self.min_x);
        self.atomic_width = 0.95 / self.delta_x.max(1) as f64 * self.view_width;
        self
    }

    fn with_data<'iter, Points, XSeq, YSeq>(mut self, data: Points) -> Self
    where
        Points: IntoIterator<Item = (XSeq, YSeq)>,
        XSeq: IntoIterator<Item = &'iter i64>,
        YSeq: IntoIterator<Item = &'iter i64>,
    {
        for (xs, ys) in data {
            for x in xs {
                self.max_x = self.max_x.max(*x);
                self.min_x = self.min_x.min(*x);
            }
            for y in ys {
                self.max_y = self.max_y.max(*y);
                self.min_y = self.min_y.min(*y);
            }
        }
        self.update()
    }

    fn resize_x(&self, x: i64) -> f64 {
        (x - self.min_x) as f64 / self.delta_x as f64 * self.view_width
    }

    fn resize_y(&self, y: i64) -> f64 {
        (self.max_y - y) as f64 / self.delta_y as f64 * self.view_height
    }
}

#[derive(Debug)]
struct RangeGroupDrawer {
    points: Vec<((i64, i64), Vec<i64>)>,
}

use svg::{
    node::element::{path::Data, Line, Path},
    Document,
};

impl RangeGroupDrawer {
    fn render(&self, file: &str) {
        // configure dimensions with extremal values
        let (xmin, ymin, width, height) = {
            let mut xmin = i64::MAX;
            let mut ymin = i64::MAX;
            let mut xmax = i64::MIN;
            let mut ymax = i64::MIN;
            for ((start, end), points) in &self.points {
                xmin = xmin.min(*start).min(*end);
                xmax = xmax.max(*start).max(*end);
                for pt in points {
                    ymin = ymin.min(*pt);
                    ymax = ymax.max(*pt);
                }
            }
            (
                xmin,
                ymin,
                xmax.saturating_sub(xmin),
                ymax.saturating_sub(ymin),
            )
        };
        // dimensions
        let fheight = 700.0;
        let fwidth = 1000.0;
        let stroke_width = 2.0;
        let margin = 20.0;
        let resize_x = |x: i64| (x - xmin) as f64 / width as f64 * fwidth;
        let resize_y = |y: i64| (height - (y - ymin)) as f64 / height as f64 * fheight;
        let atomic_width = 0.95 / width as f64 * fwidth;
        let dim = Dimensions::new().with_data(
            self.points
                .iter()
                .map(|((start, end), points)| ([start, end], points)),
        );
        // plot columns one by one
        if self.points.is_empty() {
            return;
        }
        let mut groups = Vec::new();
        let group_size = self.points[0].1.len();
        for i in 0..group_size - 1 {
            groups.push(Data::new().move_to((
                dim.resize_x(self.points[0].0 .0),
                dim.resize_y(self.points[0].1[i]),
            )));
        }
        // add lower data points
        let groups_inorder = self
            .points
            .iter()
            .fold(groups, |gr, ((start, end), points)| {
                gr.into_iter()
                    .enumerate()
                    .map(|(i, gr)| {
                        gr.line_to((dim.resize_x(*start), dim.resize_y(points[i])))
                            .line_to((
                                dim.resize_x(*end) + dim.atomic_width,
                                dim.resize_y(points[i]),
                            ))
                    })
                    .collect::<Vec<_>>()
            });
        // add upper data points
        let groups = self
            .points
            .iter()
            .rev()
            .fold(groups_inorder, |gr, ((start, end), points)| {
                gr.into_iter()
                    .enumerate()
                    .map(|(i, gr)| {
                        gr.line_to((
                            dim.resize_x(*end) + dim.atomic_width,
                            dim.resize_y(points[i + 1]),
                        ))
                        .line_to((dim.resize_x(*start), dim.resize_y(points[i + 1])))
                    })
                    .collect::<Vec<_>>()
            });
        // the two transformations above create
        //
        // (start,i+1) <-------- (end,i+1)
        //    |                     ^
        //    |                     |
        //    v                     |
        // (start,i)   --------> (end,i)
        let paths = groups
            .into_iter()
            .enumerate()
            .map(|(i, gr)| Path::new().set("fill", COLORS[i]).set("d", gr.close()));
        let yaxis = Line::new()
            .set("x1", dim.resize_x(dim.min_x))
            .set("x2", dim.resize_x(dim.min_x))
            .set("y1", dim.resize_y(dim.max_y) - dim.margin / 2.0)
            .set("y2", dim.resize_y(dim.min_y))
            .set("stroke", "black")
            .set("stroke-width", dim.stroke_width);
        let xaxis = Line::new()
            .set("x1", dim.resize_x(dim.min_x))
            .set("x2", dim.resize_x(dim.max_x) + dim.margin / 2.0)
            .set("y1", dim.resize_y(0))
            .set("y2", dim.resize_y(0))
            .set("stroke", "black")
            .set("stroke-width", dim.stroke_width);
        let document = paths
            .into_iter()
            .fold(Document::new(), |doc, path| doc.add(path))
            .add(yaxis)
            .add(xaxis)
            .set(
                "viewBox",
                (
                    -dim.margin,
                    -dim.margin,
                    dim.view_width + 2.0 * dim.margin,
                    dim.view_height + 2.0 * dim.margin,
                ),
            );
        svg::save(file, &document).unwrap();
    }
}

const COLORS: &[&str] = &["red", "green", "blue", "yellow", "orange", "purple", "cyan"];
