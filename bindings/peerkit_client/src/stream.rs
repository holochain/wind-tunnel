//! Tokenizer for the `peerkit node` REPL stdout stream.
//!
//! The CLI writes its `peerkit> ` prompt without a newline after every
//! completed command and after every event it prints above the prompt, so a
//! line-oriented reader would never see a prompt that is not followed by more
//! output. This tokenizer works on raw bytes and reports prompts as they
//! arrive, alongside complete lines.

/// The readline prompt the CLI reprints after commands and events.
const PROMPT: &[u8] = b"peerkit> ";

/// Cursor-reset escape the CLI writes before every event line it prints
/// above the prompt.
const EVENT_MARK: &[u8] = b"\x1b[1G";

/// One unit of REPL output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// A `peerkit> ` prompt. One follows every completed command and one
    /// follows every event line.
    Prompt,
    /// A complete output line without its trailing newline.
    Line {
        /// Line content with the event mark removed.
        text: String,
        /// True when the line was printed by the CLI's event printer, that
        /// is, when it started with the cursor-reset escape.
        event: bool,
    },
}

/// Incremental tokenizer over the REPL stdout byte stream.
#[derive(Debug)]
pub(crate) struct ReplTokenizer {
    buf: Vec<u8>,
    /// Whether the next byte begins a line. Prompts can only appear there.
    at_line_start: bool,
}

impl ReplTokenizer {
    /// Create a tokenizer positioned at the start of the stream.
    pub(crate) fn new() -> Self {
        Self {
            buf: Vec::new(),
            at_line_start: true,
        }
    }

    /// Append `bytes` and return every token completed by them, in order.
    ///
    /// A prompt or line split across reads is held back until it completes.
    pub(crate) fn feed(&mut self, bytes: &[u8]) -> Vec<Token> {
        self.buf.extend_from_slice(bytes);
        let mut tokens = Vec::new();
        loop {
            if self.at_line_start {
                if self.buf.starts_with(PROMPT) {
                    self.buf.drain(..PROMPT.len());
                    tokens.push(Token::Prompt);
                    continue;
                }
                if !self.buf.is_empty() && PROMPT.starts_with(&self.buf) {
                    // Could still become a prompt: wait for more bytes.
                    break;
                }
            }
            let Some(end) = self.buf.iter().position(|b| *b == b'\n') else {
                if !self.buf.is_empty() {
                    self.at_line_start = false;
                }
                break;
            };
            let raw: Vec<u8> = self.buf.drain(..=end).collect();
            let line = &raw[..end];
            let (event, body) = match line.strip_prefix(EVENT_MARK) {
                Some(rest) => (true, rest),
                None => (false, line),
            };
            tokens.push(Token::Line {
                text: String::from_utf8_lossy(body).into_owned(),
                event,
            });
            self.at_line_start = true;
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str, event: bool) -> Token {
        Token::Line {
            text: text.to_string(),
            event,
        }
    }

    #[test]
    fn startup_banner_then_prompt() {
        let mut t = ReplTokenizer::new();
        let tokens = t.feed(b"\nNode session started with agent ID abc\n\npeerkit> ");
        assert_eq!(
            tokens,
            vec![
                line("", false),
                line("Node session started with agent ID abc", false),
                line("", false),
                Token::Prompt,
            ]
        );
    }

    #[test]
    fn event_line_is_marked_and_followed_by_redraw_prompt() {
        let mut t = ReplTokenizer::new();
        let tokens =
            t.feed(b"peerkit> \x1b[1G2026-09-14T09:05:04.931Z [Peer discovered]: aa\npeerkit> ");
        assert_eq!(
            tokens,
            vec![
                Token::Prompt,
                line("2026-09-14T09:05:04.931Z [Peer discovered]: aa", true),
                Token::Prompt,
            ]
        );
    }

    #[test]
    fn multi_line_event_marks_only_its_first_line() {
        let mut t = ReplTokenizer::new();
        let tokens = t.feed(
            b"\x1b[1G2026-09-14T09:05:04.989Z [Addresses changed]: [\n    \"/ip4/x\"\n]\npeerkit> ",
        );
        assert_eq!(
            tokens,
            vec![
                line("2026-09-14T09:05:04.989Z [Addresses changed]: [", true),
                line("    \"/ip4/x\"", false),
                line("]", false),
                Token::Prompt,
            ]
        );
    }

    #[test]
    fn consecutive_prompts_on_one_line_are_separate_tokens() {
        let mut t = ReplTokenizer::new();
        let tokens = t.feed(b"peerkit> peerkit> peerkit> Connecting to 1 failed: Error: x\n");
        assert_eq!(
            tokens,
            vec![
                Token::Prompt,
                Token::Prompt,
                Token::Prompt,
                line("Connecting to 1 failed: Error: x", false),
            ]
        );
    }

    #[test]
    fn prompt_split_across_reads_is_emitted_once_complete() {
        let mut t = ReplTokenizer::new();
        assert_eq!(t.feed(b"peerk"), vec![]);
        assert_eq!(t.feed(b"it> "), vec![Token::Prompt]);
    }

    #[test]
    fn partial_line_waits_for_newline_and_prompt_after_it_is_recognised() {
        let mut t = ReplTokenizer::new();
        assert_eq!(t.feed(b"Connected to 1"), vec![]);
        assert_eq!(
            t.feed(b"\npeerkit> "),
            vec![line("Connected to 1", false), Token::Prompt]
        );
    }

    #[test]
    fn prompt_text_inside_a_line_is_not_a_prompt() {
        let mut t = ReplTokenizer::new();
        let tokens = t.feed(b"\x1b[1G2026-09-14T09:05:20.217Z [Message from 1]: peerkit> hi\n");
        assert_eq!(
            tokens,
            vec![line(
                "2026-09-14T09:05:20.217Z [Message from 1]: peerkit> hi",
                true
            )]
        );
    }

    #[test]
    fn large_payload_line_survives_chunking() {
        let mut t = ReplTokenizer::new();
        let payload = "x".repeat(262_144);
        let full =
            format!("\x1b[1G2026-09-14T09:05:22.301Z [Message from 1]: {payload}\npeerkit> ");
        let bytes = full.as_bytes();
        let mut tokens = Vec::new();
        for chunk in bytes.chunks(4096) {
            tokens.extend(t.feed(chunk));
        }
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[1], Token::Prompt);
        match &tokens[0] {
            Token::Line { text, event } => {
                assert!(*event);
                assert!(text.ends_with(&payload));
            }
            other => panic!("unexpected token {other:?}"),
        }
    }
}
