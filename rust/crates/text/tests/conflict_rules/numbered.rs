//! Files of numbered lines, each line kept, changed or removed by one side, by both the same way or, when collisions
//! are asked for, by both differently, and each place between lines given a line added by one side, by both the same
//! way, or by both differently. Since every line is unique, the merge expected is known wherever the sides do not
//! collide.

use testkit::Random;

use crate::check::ended;
use crate::merges::Case;

/// What a side does to a line of the base.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Fate {
    /// Keeps it.
    Kept,
    /// Changes it to another line.
    Changed(String),
    /// Removes it.
    Removed,
}

/// A file made at random: what each side does to each line of the base and adds at each place between lines.
#[derive(Clone, Debug)]
pub struct Numbered {
    /// What the current side and the incoming side do to each line of the base.
    fates: Vec<(Fate, Fate)>,
    /// What the current side and the incoming side add before each line of the base, and at the end.
    added: Vec<(Vec<String>, Vec<String>)>,
}

/// One of the two sides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Which {
    /// The current side.
    Current,
    /// The incoming side.
    Incoming,
}

/// Whether the sides may change the same lines differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Collisions {
    /// Never: the merge expected is known everywhere.
    Never,
    /// Sometimes.
    Sometimes,
}

/// The text of a line of the base.
fn base_line(line: usize) -> String {
    format!("line {line}")
}

impl Numbered {
    /// A file made at random, of 4 to 40 lines, that both sides change.
    pub fn new(random: &mut Random, collisions: Collisions) -> Numbered {
        loop {
            let length = 4 + random.below(37);
            let mut fates: Vec<(Fate, Fate)> = (0..length).map(|line| fates(random, line, collisions)).collect();
            // A line both sides write the same way beside a collision is a line they have alike there, and the rules
            // may read one side's change as part of the other's, as when one side added more of the same: such a
            // collision is not one.
            let colliding: Vec<usize> = (0..length).filter(|line| is_collision(&fates[*line])).collect();
            for line in colliding {
                for beside in [line.wrapping_sub(1), line + 1] {
                    if let Some(fate) = fates.get_mut(beside)
                        && matches!(fate, (Fate::Changed(current), Fate::Changed(incoming)) if current == incoming)
                    {
                        *fate = (Fate::Kept, Fate::Kept);
                    }
                }
            }
            let added = (0..=length)
                .map(|place| {
                    let beside = [place.checked_sub(1), Some(place)].map(|line| line.and_then(|line| fates.get(line)));
                    // Lines both sides add at the same place collide only between lines both keep: a side that adds a
                    // line beside one it changes or removes has changed those lines, as a diff sees it.
                    let kept = beside.iter().flatten().all(|fate| **fate == (Fate::Kept, Fate::Kept));
                    let near_collision = beside.iter().flatten().any(|fate| is_collision(fate));
                    let allowed = Allowed {
                        collision: kept && collisions == Collisions::Sometimes,
                        both: !near_collision,
                    };
                    additions(random, place, allowed)
                })
                .collect();
            let numbered = Numbered { fates, added };
            let case = numbered.case();
            if Some(&case.current) != case.base.as_ref() && Some(&case.incoming) != case.base.as_ref() {
                return numbered;
            }
        }
    }

    /// The base and the two sides, one line each, ended by a line feed.
    pub fn case(&self) -> Case {
        Case {
            base: Some(ended(&(0..self.fates.len()).map(base_line).collect::<Vec<_>>())),
            current: self.side(Which::Current),
            incoming: self.side(Which::Incoming),
        }
    }

    /// The text of one side.
    fn side(&self, which: Which) -> String {
        let pick = |pair: &(Fate, Fate)| -> Fate {
            match which {
                Which::Current => pair.0.clone(),
                Which::Incoming => pair.1.clone(),
            }
        };
        let added = |place: usize| -> Vec<String> {
            let (current, incoming) = &self.added[place];
            match which {
                Which::Current => current.clone(),
                Which::Incoming => incoming.clone(),
            }
        };
        let mut lines = Vec::new();
        for (line, fate) in self.fates.iter().enumerate() {
            lines.extend(added(line));
            match pick(fate) {
                Fate::Kept => lines.push(base_line(line)),
                Fate::Changed(changed) => lines.push(changed),
                Fate::Removed => {}
            }
        }
        lines.extend(added(self.fates.len()));
        ended(&lines)
    }

    /// The merge expected from the place before the line `start` of the base to the place before the line `end`, both
    /// included; none when the sides collide there.
    pub fn expected(&self, start: usize, end: usize) -> Option<Vec<String>> {
        let mut lines = Vec::new();
        for place in start..=end {
            let (current, incoming) = &self.added[place];
            lines.extend(merged(current, incoming)?.iter().cloned());
            if place == end {
                break;
            }
            let fate = match &self.fates[place] {
                (current, Fate::Kept) => current,
                (Fate::Kept, incoming) => incoming,
                (current, incoming) if current == incoming => current,
                _ => return None,
            };
            match fate {
                Fate::Kept => lines.push(base_line(place)),
                Fate::Changed(changed) => lines.push(changed.clone()),
                Fate::Removed => {}
            }
        }
        Some(lines)
    }

    /// The merge expected of the whole file; none when the sides collide somewhere.
    pub fn expected_file(&self) -> Option<String> {
        Some(ended(&self.expected(0, self.fates.len())?))
    }

    /// The same file with a line of its own before each line of the base and at the end, kept by both sides, so that
    /// changes to lines next to each other are apart.
    pub fn spread(&self) -> Case {
        let mut spread = self.clone();
        for (place, (current, incoming)) in spread.added.iter_mut().enumerate() {
            current.insert(0, spacer(place));
            incoming.insert(0, spacer(place));
        }
        let mut case = spread.case();
        let lines: Vec<String> = (0..self.fates.len())
            .flat_map(|line| [spacer(line), base_line(line)])
            .chain(std::iter::once(spacer(self.fates.len())))
            .collect();
        case.base = Some(ended(&lines));
        case
    }
}

/// A line that keeps changes apart.
fn spacer(place: usize) -> String {
    format!("spacer {place}")
}

/// A file merged from its spread version, without the lines that kept the changes apart.
pub fn without_spacers(text: &str) -> String {
    ended(
        &text
            .lines()
            .filter(|line| !line.starts_with("spacer "))
            .collect::<Vec<_>>(),
    )
}

/// The lines both sides added at a place, merged; none when they added different lines.
fn merged<'l>(current: &'l [String], incoming: &'l [String]) -> Option<&'l [String]> {
    if incoming.is_empty() || current == incoming {
        Some(current)
    } else if current.is_empty() {
        Some(incoming)
    } else {
        None
    }
}

/// What each side does to a line of the base, at random.
fn fates(random: &mut Random, line: usize, collisions: Collisions) -> (Fate, Fate) {
    let changed = |side: &str| Fate::Changed(format!("line {line} changed on {side}"));
    let choices = if collisions == Collisions::Sometimes { 13 } else { 10 };
    match random.below(choices) {
        0 => (changed("main"), Fate::Kept),
        1 => (Fate::Kept, changed("incoming")),
        2 => (changed("both"), changed("both")),
        3 => (Fate::Removed, Fate::Kept),
        4 => (Fate::Kept, Fate::Removed),
        5 => (Fate::Removed, Fate::Removed),
        10 => (changed("main"), changed("incoming")),
        11 => (Fate::Removed, changed("incoming")),
        12 => (changed("main"), Fate::Removed),
        _ => (Fate::Kept, Fate::Kept),
    }
}

/// Whether the sides change a line differently.
fn is_collision(fate: &(Fate, Fate)) -> bool {
    !matches!(fate, (_, Fate::Kept) | (Fate::Kept, _)) && fate.0 != fate.1
}

/// What the sides may add at a place.
#[derive(Clone, Copy, Debug)]
struct Allowed {
    /// Different lines on each side.
    collision: bool,
    /// The same line on both sides.
    both: bool,
}

/// What each side adds at a place between lines, at random.
fn additions(random: &mut Random, place: usize, allowed: Allowed) -> (Vec<String>, Vec<String>) {
    let added = |side: &str| vec![format!("added at {place} on {side}")];
    match random.below(9) {
        0 => (added("main"), Vec::new()),
        1 => (Vec::new(), added("incoming")),
        2 if allowed.both => (added("both"), added("both")),
        8 if allowed.collision => (added("main"), added("incoming")),
        _ => (Vec::new(), Vec::new()),
    }
}
