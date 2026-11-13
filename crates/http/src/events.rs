//! Server-sent events, read line by line as they arrive: how a language model streams its answer
//! (the HTML Living Standard, "Server-sent events", the event stream format).

use std::io::BufRead;

/// The most bytes one line of a stream of events may have.
const MOST_LINE_BYTES: usize = 1024 * 1024;

/// One event of a stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Its name: what its `event:` line says, `message` when it has none.
    pub name: String,
    /// Its data: its `data:` lines joined with new lines.
    pub data: String,
    /// What its `id:` line says, when it has one.
    pub id: Option<String>,
}

/// The events of a stream, one at a time as they arrive.
#[derive(Debug)]
pub struct Events<R> {
    reader: R,
}

impl<R: BufRead> Events<R> {
    /// The events of what a reader gives: the body of an answer, in a [`std::io::BufReader`].
    pub fn new(reader: R) -> Events<R> {
        Events { reader }
    }

    /// The next event, once its empty line has arrived; none when the stream ended. An event the end of the stream
    /// cuts short is left out, as the standard says.
    ///
    /// # Errors
    ///
    /// What reading the stream failed with: what [`crate::Body::error`] says in words.
    pub fn next_event(&mut self) -> std::io::Result<Option<Event>> {
        let mut name = None;
        let mut data: Option<String> = None;
        let mut id = None;
        loop {
            let mut line = Vec::new();
            let limit = u64::try_from(MOST_LINE_BYTES).unwrap_or(u64::MAX);
            let read = std::io::Read::take(&mut self.reader, limit).read_until(b'\n', &mut line)?;
            if read == 0 {
                return Ok(None);
            }
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_end_matches('\n').trim_end_matches('\r');
            if line.is_empty() {
                if let Some(data) = data.take() {
                    return Ok(Some(Event {
                        name: name.take().unwrap_or_else(|| String::from("message")),
                        data,
                        id: id.take(),
                    }));
                }
                name = None;
                continue;
            }
            if line.starts_with(':') {
                continue;
            }
            let (field, value) = line.split_once(':').unwrap_or((line, ""));
            let value = value.strip_prefix(' ').unwrap_or(value);
            match field {
                "event" => name = Some(value.to_owned()),
                "data" => match &mut data {
                    Some(joined) => {
                        joined.push('\n');
                        joined.push_str(value);
                    }
                    None => data = Some(value.to_owned()),
                },
                "id" => id = Some(value.to_owned()),
                // `retry` and fields the standard does not name are left out, as it says.
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, Events};

    fn events(text: &str) -> Vec<Event> {
        let mut stream = Events::new(text.as_bytes());
        let mut found = Vec::new();
        while let Some(event) = stream.next_event().unwrap() {
            found.push(event);
        }
        found
    }

    fn event(name: &str, data: &str, id: Option<&str>) -> Event {
        Event {
            name: name.to_owned(),
            data: data.to_owned(),
            id: id.map(str::to_owned),
        }
    }

    #[test]
    fn events_are_read_one_by_one() {
        let text = "data: one\n\nevent: delta\ndata: two\r\nid: 7\r\n\r\n: a comment\ndata:three\n\n";
        assert_eq!(
            events(text),
            [
                event("message", "one", None),
                event("delta", "two", Some("7")),
                event("message", "three", None)
            ]
        );
    }

    #[test]
    fn data_on_several_lines_is_joined_with_new_lines() {
        assert_eq!(
            events("data: a\ndata:\ndata: b\n\n"),
            [event("message", "a\n\nb", None)]
        );
    }

    #[test]
    fn an_event_with_no_data_and_one_cut_short_are_left_out() {
        assert_eq!(events("event: ping\n\ndata: cut"), []);
        assert_eq!(events("retry: 10\n\n"), []);
    }
}
