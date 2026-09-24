use crate::extract::PageText;

/// A retrieval unit: text pinned to the page it came from.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub page: usize,
    pub text: String,
}

/// Target size for one chunk, in characters. Large enough to carry a
/// complete thought (BM25 needs shared terms), small enough that a
/// retrieved passage is readable as a citation.
const MAX_CHUNK_CHARS: usize = 800;
/// Chunks shorter than this are merged with the next paragraph on the same
/// page instead of standing alone.
const MIN_CHUNK_CHARS: usize = 120;

/// Splits pages into paragraph-aware chunks. Chunks never cross pages — a
/// citation must point at a page the sentence actually sits on.
pub fn chunk_pages(pages: &[PageText]) -> Vec<Chunk> {
    let mut chunks: Vec<Chunk> = Vec::new();
    for page in pages {
        for paragraph in split_paragraphs(&page.text) {
            let paragraph = paragraph.trim();
            if paragraph.is_empty() {
                continue;
            }
            // Oversize paragraphs are split on sentence ends so no chunk
            // blows past the target size.
            for piece in split_long(&paragraph, MAX_CHUNK_CHARS) {
                // Merge tiny leftovers into the previous chunk when they
                // belong to the same page and the merge stays in budget.
                if let Some(last) = chunks.last_mut() {
                    if last.page == page.page
                        && last.text.chars().count() + piece.chars().count() + 1 <= MAX_CHUNK_CHARS
                        && piece.chars().count() < MIN_CHUNK_CHARS
                    {
                        last.text.push(' ');
                        last.text.push_str(&piece);
                        continue;
                    }
                }
                chunks.push(Chunk {
                    page: page.page,
                    text: piece,
                });
            }
        }
    }
    chunks
}

/// Splits on blank-line runs and on single newlines that look like
/// paragraph breaks (pdf-extract emits one `\n` per rendered line, so a
/// newline followed by a heading-ish line is not reliable; blank lines are
/// the strong signal, with single-newline joins kept inside a paragraph).
fn split_paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

/// Splits `text` into pieces of at most `max` chars, preferring to break on
/// sentence ends.
fn split_long(text: &str, max: usize) -> Vec<String> {
    let text = text.trim();
    if text.chars().count() <= max {
        return vec![text.to_string()];
    }
    let mut pieces = Vec::new();
    let mut current = String::new();
    for sentence in split_sentences(text) {
        if current.chars().count() + sentence.chars().count() + 1 > max && !current.is_empty() {
            pieces.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(&sentence);
    }
    if !current.is_empty() {
        pieces.push(current);
    }
    pieces
}

/// Splits on `.`, `!`, `?` followed by whitespace; keeps the terminator.
pub fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for i in 0..chars.len() {
        let (_, ch) = chars[i];
        if ch == '.' || ch == '!' || ch == '?' {
            match chars.get(i + 1).map(|(_, c)| *c) {
                Some(' ') | Some('\n') | None => {
                    let end = chars.get(i + 1).map(|(b, _)| *b).unwrap_or(text.len());
                    let s = text[start..end].trim();
                    if !s.is_empty() {
                        sentences.push(s.to_string());
                    }
                    start = chars.get(i + 1).map(|(b, _)| *b).unwrap_or(text.len());
                }
                _ => {}
            }
        }
    }
    let tail = text[start.min(text.len())..].trim();
    if !tail.is_empty() {
        sentences.push(tail.to_string());
    }
    sentences
}
