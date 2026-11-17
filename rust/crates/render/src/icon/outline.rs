//! The outlines of Lucide's icons, read from their SVG files: the elements Lucide draws with (`path`, `circle`,
//! `ellipse`, `rect`, `line`, `polyline` and `polygon`) turned into paths on its grid of 24 by 24, to be stroked, and
//! filled too where an element says `fill="currentColor"`.
//!
//! Lucide's files are written by a program in one plain form: one element on each line or over a few, its attributes
//! as `name="value"`, no styles, no transforms, no groups. That form is all this reads; `svgtypes` parses the path data
//! and the lists of points, arcs and short forms included.

use svgtypes::{PointsParser, SimplePathSegment, SimplifyingPathParser};

use super::Design;
use crate::geometry::{Point, Rect};
use crate::number;
use crate::path::Path;

/// What went wrong in reading the outline of an icon: the element and what was wrong with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OutlineError {
    /// An element this does not know how to draw.
    UnknownElement(String),
    /// An attribute an element needs is missing, or is not a number.
    Attribute {
        /// The element.
        element: String,
        /// The attribute.
        name: &'static str,
    },
    /// The data of a path does not follow the grammar of SVG.
    PathData(String),
    /// A tag is opened and never closed.
    UnclosedTag,
}

/// The paths of an SVG file of Lucide: what each element draws, on the grid of the file.
pub(crate) fn outline(svg: &str) -> Result<Design, OutlineError> {
    let mut design = Design::default();
    for tag in tags(svg)? {
        let Some(element) = Element::parse(tag) else {
            continue;
        };
        let path = element.path()?;
        if element.attribute("fill") == Some("currentColor") {
            let _ = design.fills.extend(&path);
        }
        let _ = design.strokes.extend(&path);
    }
    Ok(design)
}

/// The text inside each `<…>` of a file, comments and the closing tags left out.
fn tags(svg: &str) -> Result<Vec<&str>, OutlineError> {
    let mut found = Vec::new();
    let mut rest = svg;
    while let Some(start) = rest.find('<') {
        let after = rest.get(start + 1..).unwrap_or_default();
        if let Some(comment) = after.strip_prefix("!--") {
            let end = comment.find("-->").ok_or(OutlineError::UnclosedTag)?;
            rest = comment.get(end + 3..).unwrap_or_default();
            continue;
        }
        let end = after.find('>').ok_or(OutlineError::UnclosedTag)?;
        let tag = after.get(..end).unwrap_or_default();
        if !tag.starts_with('/') {
            found.push(tag.trim_end_matches('/').trim());
        }
        rest = after.get(end + 1..).unwrap_or_default();
    }
    Ok(found)
}

/// An element of a file: its name and its attributes, in the order written.
struct Element<'source> {
    /// The name, such as `circle`.
    name: &'source str,
    /// Each attribute's name and value.
    attributes: Vec<(&'source str, &'source str)>,
}

impl<'source> Element<'source> {
    /// The element a tag opens; nothing for the `svg` element itself, whose attributes are the same in every file of
    /// Lucide.
    fn parse(tag: &'source str) -> Option<Element<'source>> {
        let (name, mut rest) = tag.split_once(char::is_whitespace).unwrap_or((tag, ""));
        if name == "svg" {
            return None;
        }
        let mut attributes = Vec::new();
        while let Some((key, after)) = rest.split_once("=\"") {
            let Some((value, next)) = after.split_once('"') else {
                break;
            };
            attributes.push((key.trim(), value));
            rest = next;
        }
        Some(Element { name, attributes })
    }

    /// The value of an attribute, if the element has it.
    fn attribute(&self, name: &str) -> Option<&'source str> {
        self.attributes
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| *value)
    }

    /// The value of an attribute that is a number.
    fn number(&self, name: &'static str) -> Result<f32, OutlineError> {
        self.attribute(name)
            .and_then(|value| value.trim().parse::<f32>().ok())
            .ok_or_else(|| OutlineError::Attribute {
                element: self.name.to_owned(),
                name,
            })
    }

    /// The value of an attribute that is a number, or zero when the element does not have it.
    fn number_or_zero(&self, name: &'static str) -> Result<f32, OutlineError> {
        if self.attribute(name).is_none() {
            return Ok(0.0);
        }
        self.number(name)
    }

    /// What the element draws.
    fn path(&self) -> Result<Path, OutlineError> {
        match self.name {
            "path" => path_data(self.attribute("d").unwrap_or_default()),
            "circle" => Ok(Path::circle(
                Point::new(self.number("cx")?, self.number("cy")?),
                self.number("r")?,
            )),
            "ellipse" => Ok(Path::ellipse(
                Point::new(self.number("cx")?, self.number("cy")?),
                Point::new(self.number("rx")?, self.number("ry")?),
            )),
            "rect" => {
                let rect = Rect::new(
                    self.number_or_zero("x")?,
                    self.number_or_zero("y")?,
                    self.number("width")?,
                    self.number("height")?,
                );
                Ok(Path::rounded_rect(rect, self.number_or_zero("rx")?))
            }
            "line" => {
                let mut path = Path::new();
                let _ = path
                    .move_to(Point::new(self.number("x1")?, self.number("y1")?))
                    .line_to(Point::new(self.number("x2")?, self.number("y2")?));
                Ok(path)
            }
            "polyline" => Ok(polyline(self.attribute("points").unwrap_or_default(), false)),
            "polygon" => Ok(polyline(self.attribute("points").unwrap_or_default(), true)),
            other => Err(OutlineError::UnknownElement(other.to_owned())),
        }
    }
}

/// The path of the data of a `path` element, its arcs turned into cubic curves.
fn path_data(data: &str) -> Result<Path, OutlineError> {
    let mut path = Path::new();
    let at = |x: f64, y: f64| Point::new(number::narrowed(x), number::narrowed(y));
    for segment in SimplifyingPathParser::from(data) {
        let segment = segment.map_err(|error| OutlineError::PathData(error.to_string()))?;
        let _ = match segment {
            SimplePathSegment::MoveTo { x, y } => path.move_to(at(x, y)),
            SimplePathSegment::LineTo { x, y } => path.line_to(at(x, y)),
            SimplePathSegment::Quadratic { x1, y1, x, y } => path.quadratic_to(at(x1, y1), at(x, y)),
            SimplePathSegment::CurveTo { x1, y1, x2, y2, x, y } => path.cubic_to(at(x1, y1), at(x2, y2), at(x, y)),
            SimplePathSegment::ClosePath => path.close(),
        };
    }
    Ok(path)
}

/// The path through a list of points, closed for a polygon.
fn polyline(points: &str, is_closed: bool) -> Path {
    let mut path = Path::new();
    for (index, (x, y)) in PointsParser::from(points).enumerate() {
        let point = Point::new(number::narrowed(x), number::narrowed(y));
        let _ = if index == 0 {
            path.move_to(point)
        } else {
            path.line_to(point)
        };
    }
    if is_closed && !path.is_empty() {
        let _ = path.close();
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::Command;

    #[test]
    fn the_elements_of_lucide_become_paths() {
        let svg = r#"<!-- @license lucide-static - ISC -->
<svg
  class="lucide"
  viewBox="0 0 24 24"
  stroke-width="2"
>
  <path d="M18 6 6 18" />
  <line x1="2" x2="22" y1="12" y2="12" />
  <circle cx="7.5" cy="7.5" r=".5" fill="currentColor" />
</svg>"#;
        let design = outline(svg).unwrap_or_default();
        let commands = design.strokes.commands();
        assert_eq!(commands.first(), Some(&Command::MoveTo(Point::new(18.0, 6.0))));
        assert_eq!(commands.get(1), Some(&Command::LineTo(Point::new(6.0, 18.0))));
        assert_eq!(commands.get(3), Some(&Command::LineTo(Point::new(22.0, 12.0))));
        assert!(!design.fills.is_empty(), "the dot is filled too");
    }

    #[test]
    fn an_arc_becomes_curves_that_end_where_it_ends() {
        let path = path_data("M15 6a9 9 0 0 0-9 9V3").unwrap_or_default();
        let ends: Vec<Point> = path
            .commands()
            .iter()
            .filter_map(|command| match *command {
                Command::CubicTo(_, _, end) | Command::LineTo(end) => Some(end),
                _ => None,
            })
            .collect();
        assert!(
            ends.iter()
                .any(|end| (end.x - 6.0).abs() < 1e-4 && (end.y - 15.0).abs() < 1e-4)
        );
        assert_eq!(ends.last(), Some(&Point::new(6.0, 3.0)));
    }

    #[test]
    fn what_cannot_be_drawn_is_said() {
        assert_eq!(
            outline(r#"<svg><text x="1" /></svg>"#).err(),
            Some(OutlineError::UnknownElement("text".to_owned()))
        );
        assert_eq!(
            outline(r#"<svg><circle cx="1" cy="1" /></svg>"#).err(),
            Some(OutlineError::Attribute {
                element: "circle".to_owned(),
                name: "r"
            })
        );
        assert_eq!(outline("<svg><path d=\"M1 1").err(), Some(OutlineError::UnclosedTag));
    }
}
