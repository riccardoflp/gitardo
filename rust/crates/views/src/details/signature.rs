//! The line **Signature** of the inspector and of the page of a commit (`details.signature`): whether the commit is
//! signed and whether the signature holds, with a shield in the color of what git said.

use model::details::{CommitSignature, SignatureVerdict};
use ui::layout::{Align, Area, place};
use ui::{Label, Sense, TextKind, Tone, Ui};

use crate::icons;
use crate::intent::Intent;
use crate::text::details as words;

/// What git said of the signature of the commit shown. While it is read there is none, and no line: the lines under
/// it would move down for it and up again for every commit that is not signed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignatureRead<'a>(pub &'a CommitSignature);

impl SignatureRead<'_> {
    /// Whether the line is drawn: not for a commit that is not signed (Q1091).
    #[must_use]
    pub fn is_shown(self) -> bool {
        self.0.verdict != SignatureVerdict::NotSigned
    }
}

/// What the line of a signature is drawn from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SignatureState<'a> {
    /// The signature, as git read it.
    pub read: SignatureRead<'a>,
    /// The width of the column of the names of the lines, as the other lines of its card have it.
    pub key_width: f32,
}

/// The tone of what git said of a signature: a good one in the color of an addition, one git doubts in the color of
/// a warning, a bad one in the color of a removal, and the rest muted.
fn tone(verdict: SignatureVerdict) -> Tone {
    match verdict {
        SignatureVerdict::Good => Tone::Success,
        SignatureVerdict::GoodUnknownValidity
        | SignatureVerdict::GoodExpired
        | SignatureVerdict::GoodExpiredKey
        | SignatureVerdict::GoodRevokedKey => Tone::Warning,
        SignatureVerdict::Bad => Tone::Error,
        SignatureVerdict::CannotCheck | SignatureVerdict::SignedNotChecked | SignatureVerdict::NotSigned => Tone::Muted,
    }
}

/// The line **Signature**, over the rectangle its caller clipped it to: the name of the line, then a shield and the
/// words of what git said, whose tooltip has the kind of signature, the key, its fingerprint and its trust. It asks
/// nothing.
///
/// Feature: details.signature
pub fn signature_view(state: &SignatureState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let mut parts = Area::new(rect);
    let _ = ui.label(
        parts.left(state.key_width),
        &Label::new(words::SIGNATURE).kind(TextKind::Detail).tone(Tone::Muted),
    );
    let SignatureRead(signature) = state.read;
    let said = words::signature_words(signature);
    let tooltip = words::signature_tooltip(signature);
    let tone = tone(signature.verdict);
    let size = theme.sizes.icon;
    let mut value = Area::with_gap(parts.rest(), theme.sizes.spacing.small);
    let icon = place(value.left(size), (size, size), (Align::Start, Align::Center));
    ui.icon_in(icon, icons::SIGNATURE, tone);
    let words_rect = value.rest();
    let _ = ui.label(words_rect, &Label::new(&said).tone(tone));
    let response = ui.interact(ui.id("signature"), words_rect, Sense::HOVER);
    ui.tooltip(&response, &tooltip);
    Vec::new()
}
