//! Incremental markdown rendering for text-mode stdout via `mdstream`.

use std::io::{self, Write};

use mdstream::{DocumentState, MdStream, Options};

/// Renders streaming markdown chunks to stdout without full-buffer reparsing.
pub struct MarkdownStreamRenderer {
    stream: MdStream,
    state: DocumentState,
    committed_count: usize,
}

impl MarkdownStreamRenderer {
    pub fn new() -> Self {
        Self {
            stream: MdStream::new(Options::default()),
            state: DocumentState::new(),
            committed_count: 0,
        }
    }

    /// Append a text delta and flush newly committed / pending blocks to stdout.
    pub fn append_and_render(&mut self, text: &str, out: &mut impl Write) -> io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let update = self.stream.append(text);
        let applied = self.state.apply(update);
        if applied.reset {
            self.committed_count = 0;
        }
        let committed = self.state.committed();
        while self.committed_count < committed.len() {
            let block = &committed[self.committed_count];
            writeln!(out, "{}", block.display_or_raw())?;
            self.committed_count += 1;
        }
        if let Some(pending) = self.state.pending() {
            write!(out, "{}", pending.display_or_raw())?;
        }
        out.flush()
    }

    pub fn finalize(&mut self, out: &mut impl Write) -> io::Result<()> {
        let update = self.stream.finalize();
        let applied = self.state.apply(update);
        if applied.reset {
            self.committed_count = 0;
        }
        let committed = self.state.committed();
        while self.committed_count < committed.len() {
            let block = &committed[self.committed_count];
            writeln!(out, "{}", block.display_or_raw())?;
            self.committed_count += 1;
        }
        writeln!(out)
    }
}

impl Default for MarkdownStreamRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_incremental_chunks() {
        let mut renderer = MarkdownStreamRenderer::new();
        let mut buf = Vec::new();
        renderer.append_and_render("Hello ", &mut buf).unwrap();
        renderer.append_and_render("**world**", &mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("Hello"));
    }
}
