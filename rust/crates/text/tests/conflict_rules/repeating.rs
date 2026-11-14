//! Files made mostly of lines that repeat (closing braces, empty lines), changed at random on two sides: the pairings
//! of their lines with the base are many, which is where the rules must not guess.

use testkit::Random;

use crate::check::ended;
use crate::merges::Case;

/// The lines files are made of, most of them more than once.
const LINES: [&str; 10] = [
    "}",
    "}",
    "}",
    "",
    "",
    "{",
    "    }",
    "        return;",
    "    x += 1;",
    "        return;",
];

/// A file of 3 to 30 lines that repeat, changed by both sides.
pub fn repeating(random: &mut Random) -> Case {
    loop {
        let length = 3 + random.below(28);
        let base: Vec<String> = (0..length).map(|_| (*random.pick(&LINES)).to_owned()).collect();
        let current = edited(random, &base, "main");
        let incoming = edited(random, &base, "incoming");
        if current != base && incoming != base {
            return Case {
                base: Some(ended(&base)),
                current: ended(&current),
                incoming: ended(&incoming),
            };
        }
    }
}

/// The lines of the base, some removed, changed or followed by lines added, at random.
fn edited(random: &mut Random, base: &[String], side: &str) -> Vec<String> {
    let mut lines = Vec::new();
    if random.chance(1, 10) {
        lines.push((*random.pick(&LINES)).to_owned());
    }
    for (place, line) in base.iter().enumerate() {
        match random.below(14) {
            0 => {}
            1 => lines.push((*random.pick(&LINES)).to_owned()),
            2 => lines.push(format!("line {place} on {side}")),
            _ => lines.push(line.clone()),
        }
        if random.chance(1, 9) {
            lines.push((*random.pick(&LINES)).to_owned());
        }
    }
    lines
}
