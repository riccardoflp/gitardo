//! Painting: the shapes a frame declares, kept by layer, and drawn at the end of the frame into the parts of the window
//! that changed.

mod display;
mod tiles;

pub(crate) use display::{Command, Shape};
pub(crate) use tiles::Tiles;

use render::{Canvas, Rect};

use crate::layer::Layer;

/// The shapes of a frame, by layer, each layer in the order its shapes were declared.
#[derive(Clone, Debug, Default)]
pub(crate) struct DisplayList {
    /// The commands of each layer, by [`Layer::index`].
    layers: [Vec<Command>; Layer::COUNT],
}

impl DisplayList {
    /// Adds a command to a layer.
    pub(crate) fn push(&mut self, layer: Layer, command: Command) {
        if let Some(commands) = self.layers.get_mut(layer.index()) {
            commands.push(command);
        }
    }

    /// Every command, the lowest layer first: the order they are drawn in.
    pub(crate) fn in_order(&self) -> impl Iterator<Item = &Command> {
        self.layers.iter().flatten()
    }

    /// How many commands the frame declared.
    pub(crate) fn len(&self) -> usize {
        self.layers.iter().map(Vec::len).sum()
    }

    /// The hashes of the tiles of a window of a size, from every command in the order they are drawn.
    pub(crate) fn tiles(&self, width: f32, height: f32) -> Tiles {
        let mut tiles = Tiles::new(width, height);
        for command in self.in_order() {
            tiles.add(command.bounds, command.hash_value());
        }
        tiles
    }

    /// Draws every command that meets each region, inside that region, in order.
    pub(crate) fn draw(&self, canvas: &mut Canvas<'_>, regions: &[Rect]) {
        for region in regions {
            for command in self.in_order() {
                if !command.bounds.intersection(*region).is_empty() {
                    command.draw(canvas, *region);
                }
            }
        }
    }
}
