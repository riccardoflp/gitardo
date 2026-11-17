//! What the screen reader says of a diagram: the text of the `title` and the `desc` Mermaid writes into its SVG from
//! `accTitle` and `accDescr`.

/// The text of the first element of a name in an SVG, its entities read; none when it has none, or only spaces.
pub(super) fn text_of(svg: &str, name: &str) -> Option<String> {
    let opening = format!("<{name}");
    let closing = format!("</{name}>");
    let mut rest = svg;
    loop {
        let start = rest.find(&opening)?;
        let after = rest.get(start + opening.len()..)?;
        // `<title` must be the whole name: `<titles` is another element.
        if after.starts_with(['>', ' ', '\t', '\n', '\r']) {
            let inside = after.get(after.find('>')? + 1..)?;
            let text = unescaped(inside.get(..inside.find(&closing)?)?);
            let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            return (!text.is_empty()).then_some(text);
        }
        rest = after;
    }
}

/// Text of XML with its entities read: `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;` and the numbered ones.
fn unescaped(text: &str) -> String {
    let mut read = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        read.push_str(rest.get(..start).unwrap_or_default());
        let after = rest.get(start..).unwrap_or_default();
        let entity = after
            .find(';')
            .and_then(|end| after.get(1..end).map(|name| (name, end)));
        if let Some((character, end)) = entity.and_then(|(name, end)| character(name).map(|character| (character, end)))
        {
            read.push(character);
            rest = after.get(end + 1..).unwrap_or_default();
        } else {
            read.push('&');
            rest = after.get(1..).unwrap_or_default();
        }
    }
    read.push_str(rest);
    read
}

/// The character an entity names, without its `&` and `;`.
fn character(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => {
            let number = name.strip_prefix('#')?;
            let value = match number.strip_prefix(['x', 'X']) {
                Some(hexadecimal) => u32::from_str_radix(hexadecimal, 16).ok()?,
                None => number.parse().ok()?,
            };
            char::from_u32(value)
        }
    }
}
