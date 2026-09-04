//! Turning a byte stream into Server-Sent Events payloads.
//!
//! Both vendors stream with SSE, so the framing is shared even though what the
//! payloads *say* is not.
//!
//! This is a separate, pure type because the failure it exists to prevent is
//! invisible in an integration test that happens to deliver whole lines: a
//! network chunk can split anywhere, including the middle of a UTF-8 character
//! or between `dat` and `a:`. A decoder that assumes chunk boundaries are line
//! boundaries works perfectly against a local stub and drops text in
//! production.

/// Accumulates bytes and yields complete `data:` payloads.
#[derive(Debug, Default)]
pub struct SseDecoder {
    buffer: Vec<u8>,
}

impl SseDecoder {
    /// A decoder with nothing buffered.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds one network chunk and returns whatever payloads it completed.
    ///
    /// A partial line is kept for the next call rather than guessed at.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(chunk);

        let mut payloads = Vec::new();

        // Lines are split on \n; a trailing \r is stripped, so both CRLF and
        // LF framing work. Servers use both.
        while let Some(newline) = self.buffer.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.buffer.drain(..=newline).collect();
            let line = &line[..line.len() - 1];
            let line = line.strip_suffix(b"\r").unwrap_or(line);

            // Lossy rather than strict: a split multi-byte character would
            // otherwise fail the whole line. Splits land between lines by the
            // time we get here, so this is belt and braces.
            let line = String::from_utf8_lossy(line);

            // `event:` and `id:` lines are ignored: both vendors put
            // everything we need in the JSON payload, and reading the event
            // name as well would be two sources of truth for one fact.
            if let Some(payload) = line.strip_prefix("data:") {
                payloads.push(payload.trim().to_string());
            }
        }

        payloads
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_whole_event_yields_its_payload() {
        let mut d = SseDecoder::new();

        assert_eq!(d.push(b"data: {\"a\":1}\n\n"), vec!["{\"a\":1}"]);
    }

    /// The reason this type exists. A chunk boundary is not a line boundary,
    /// and a decoder that assumes it is loses text only under real network
    /// conditions.
    #[test]
    fn a_payload_split_across_chunks_is_reassembled() {
        let mut d = SseDecoder::new();

        assert!(d.push(b"data: {\"te").is_empty());
        assert!(d.push(b"xt\":\"hel").is_empty());
        assert_eq!(d.push(b"lo\"}\n"), vec!["{\"text\":\"hello\"}"]);
    }

    /// Including a split inside the field name itself.
    #[test]
    fn a_split_inside_the_field_name_survives() {
        let mut d = SseDecoder::new();

        assert!(d.push(b"dat").is_empty());
        assert_eq!(d.push(b"a: 1\n"), vec!["1"]);
    }

    #[test]
    fn several_events_in_one_chunk_all_come_out() {
        let mut d = SseDecoder::new();

        let out = d.push(b"data: one\n\ndata: two\n\ndata: three\n");

        assert_eq!(out, vec!["one", "two", "three"]);
    }

    #[test]
    fn crlf_framing_works_too() {
        let mut d = SseDecoder::new();

        assert_eq!(d.push(b"data: {\"a\":1}\r\n\r\n"), vec!["{\"a\":1}"]);
    }

    /// Event and id lines carry nothing the payload does not.
    #[test]
    fn non_data_lines_are_ignored() {
        let mut d = SseDecoder::new();

        let out = d.push(b"event: content_block_delta\nid: 7\ndata: payload\n\n");

        assert_eq!(out, vec!["payload"]);
    }

    /// A multi-byte character split across chunks must not corrupt the text.
    #[test]
    fn a_split_multibyte_character_is_preserved() {
        let mut d = SseDecoder::new();
        let em_dash = "—".as_bytes();

        assert!(d.push(b"data: a").is_empty());
        assert!(d.push(&em_dash[..1]).is_empty());
        assert!(d.push(&em_dash[1..]).is_empty());
        let out = d.push(b"b\n");

        assert_eq!(out, vec!["a—b"]);
    }

    #[test]
    fn nothing_is_emitted_until_a_line_is_complete() {
        let mut d = SseDecoder::new();

        assert!(d.push(b"data: incomplete").is_empty());
    }
}
