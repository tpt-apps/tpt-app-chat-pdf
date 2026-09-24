use crate::chunk::Chunk;
use std::collections::HashMap;

/// BM25 parameters (standard defaults).
const K1: f32 = 1.2;
const B: f32 = 0.75;

/// Terms too common to be worth indexing or matching. Deliberately short —
/// domain words like "maximum" or "below" carry real signal in technical
/// documents.
const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "from", "had", "has", "have",
    "he", "her", "his", "i", "if", "in", "is", "it", "its", "of", "on", "or", "she", "that", "the",
    "their", "them", "then", "there", "these", "they", "this", "to", "was", "were", "will", "with",
];

/// One scored retrieval hit.
pub struct Scored<'a> {
    pub chunk: &'a Chunk,
    pub score: f32,
}

/// A small BM25 lexical index over the document's chunks. Pure std —
/// compiles everywhere the app does, no model weights, sub-millisecond on
/// document-sized corpora.
#[derive(Clone)]
pub struct BM25Index {
    chunks: Vec<Chunk>,
    /// term -> (chunk index, term frequency in that chunk)
    postings: HashMap<String, Vec<(usize, u32)>>,
    /// document frequency per term
    doc_freq: HashMap<String, u32>,
    doc_lengths: Vec<usize>,
    avgdl: f32,
}

pub(crate) fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 1 && !STOPWORDS.contains(&t.to_ascii_lowercase().as_str()))
        .map(|t| t.to_ascii_lowercase())
        .collect()
}

impl BM25Index {
    pub fn build(chunks: Vec<Chunk>) -> Self {
        let mut postings: HashMap<String, Vec<(usize, u32)>> = HashMap::new();
        let mut doc_freq: HashMap<String, u32> = HashMap::new();
        let mut doc_lengths = Vec::with_capacity(chunks.len());

        for (idx, chunk) in chunks.iter().enumerate() {
            let mut counts: HashMap<String, u32> = HashMap::new();
            for term in tokenize(&chunk.text) {
                *counts.entry(term).or_insert(0) += 1;
            }
            let mut length = 0;
            for (term, freq) in counts {
                length += freq as usize;
                *doc_freq.entry(term.clone()).or_insert(0) += 1;
                postings.entry(term).or_default().push((idx, freq));
            }
            doc_lengths.push(length);
        }

        let avgdl = if doc_lengths.is_empty() {
            0.0
        } else {
            doc_lengths.iter().sum::<usize>() as f32 / doc_lengths.len() as f32
        };

        Self {
            chunks,
            postings,
            doc_freq,
            doc_lengths,
            avgdl,
        }
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    pub fn chunks(&self) -> &[Chunk] {
        &self.chunks
    }

    /// Inverse document frequency, BM25+ style smoothed; 0 when the term is
    /// unknown (so unknown query terms contribute nothing).
    fn idf(&self, term: &str) -> f32 {
        let n = self.chunks.len() as f32;
        match self.doc_freq.get(term) {
            Some(df) if *df > 0 => {
                (n - *df as f32 + 0.5) / (*df as f32 + 0.5) + 1.0
            }
            _ => 0.0,
        }
    }

    /// BM25 scores every chunk against `query` and returns the `top_k` best,
    /// in descending score order. Chunks scoring zero are dropped.
    pub fn search(&self, query: &str, top_k: usize) -> Vec<Scored<'_>> {
        let mut terms: HashMap<String, u32> = HashMap::new();
        for term in tokenize(query) {
            *terms.entry(term).or_insert(0) += 1;
        }

        let mut scores = vec![0.0f32; self.chunks.len()];
        for (term, qtf) in &terms {
            let idf = self.idf(term);
            if idf <= 0.0 {
                continue;
            }
            if let Some(hits) = self.postings.get(term) {
                for &(idx, freq) in hits {
                    let tf = freq as f32;
                    let dl = self.doc_lengths[idx] as f32;
                    let norm = dl / self.avgdl.max(1.0);
                    scores[idx] +=
                        idf * (tf * (K1 + 1.0)) / (tf + K1 * (1.0 - B + B * norm)) * (*qtf as f32);
                }
            }
        }

        let mut hits: Vec<(usize, f32)> = scores
            .into_iter()
            .enumerate()
            .filter(|(_, s)| *s > 0.0)
            .collect();
        hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        hits.truncate(top_k);
        hits.into_iter()
            .map(|(idx, score)| Scored {
                chunk: &self.chunks[idx],
                score,
            })
            .collect()
    }

    /// IDF of a single term — exposed for the sentence scorer.
    pub fn term_idf(&self, term: &str) -> f32 {
        self.idf(term)
    }
}
