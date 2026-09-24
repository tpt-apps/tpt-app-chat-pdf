use crate::chunk::split_sentences;
use crate::index::{tokenize, BM25Index};
use serde::{Deserialize, Serialize};

/// One extractive sentence with the page it sits on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statement {
    pub text: String,
    pub page: usize,
}

/// A retrieved passage kept for "show your sources" display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub page: usize,
    pub text: String,
}

/// The response to a question. Every `statement` is copied verbatim from
/// the document — the engine cannot invent content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    pub question: String,
    /// Best matching sentences, best first (usually 1-3).
    pub statements: Vec<Statement>,
    /// The passages the statements were drawn from.
    pub passages: Vec<Citation>,
}

/// Answers `question` extractively: BM25 retrieves the top passages, then
/// the individual sentences inside them are re-ranked by idf-weighted term
/// overlap so the reply is the tightest matching sentences, not whole
/// paragraphs.
pub fn answer(index: &BM25Index, question: &str, top_k: usize) -> Answer {
    let hits = index.search(question, top_k.max(1));
    let max_hit = hits.iter().map(|h| h.score).fold(f32::MIN, f32::max);
    let mut passages = Vec::new();
    let mut best: Vec<(f32, Statement)> = Vec::new();

    for hit in hits {
        // Retrieval strength and sentence overlap are independent signals:
        // scale each sentence by how strongly its passage matched, so a
        // good sentence from a great passage beats an equal one from a
        // weaker passage.
        let passage_weight = 1.0 + hit.score / max_hit.max(1.0);
        passages.push(Citation {
            page: hit.chunk.page,
            text: hit.chunk.text.clone(),
        });
        for sentence in split_sentences(&hit.chunk.text) {
            let score = sentence_score(index, question, &sentence) * passage_weight;
            if score <= 0.0 {
                continue;
            }
            best.push((score, Statement {
                text: sentence,
                page: hit.chunk.page,
            }));
        }
    }

    // Best sentences first; drop near-duplicates (same page, same first 40
    // chars can appear in overlapping chunks) and keep at most three.
    best.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut statements: Vec<Statement> = Vec::new();
    for (_, stmt) in best {
        let key = stmt.text.chars().take(40).collect::<String>().to_ascii_lowercase();
        let dup = statements.iter().any(|s| {
            s.page == stmt.page
                && s.text.chars().take(40).collect::<String>().to_ascii_lowercase() == key
        });
        if !dup {
            statements.push(stmt);
        }
        if statements.len() == 3 {
            break;
        }
    }

    Answer {
        question: question.to_string(),
        statements,
        passages,
    }
}

/// IDF-weighted overlap between query terms and sentence terms, lightly
/// length-normalised. Exposed for the Pro library's cross-document ranking
/// (per-doc statement scores must share one scale).
pub fn score_statement(index: &BM25Index, question: &str, sentence: &str) -> f32 {
    sentence_score(index, question, sentence)
}

/// IDF-weighted overlap between query terms and sentence terms, lightly
/// length-normalised.
fn sentence_score(index: &BM25Index, question: &str, sentence: &str) -> f32 {
    let query_terms = tokenize(question);
    if query_terms.is_empty() {
        return 0.0;
    }
    let sentence_terms = tokenize(sentence);
    let mut uniq: Vec<&str> = query_terms.iter().map(|s| s.as_str()).collect();
    uniq.sort_unstable();
    uniq.dedup();

    let mut score = 0.0;
    for term in uniq {
        let tf = sentence_terms.iter().filter(|t| *t == term).count() as f32;
        if tf > 0.0 {
            score += index.term_idf(term) * (1.0 + tf.ln());
        }
    }
    // Dampen very long sentences so a whole paragraph doesn't beat the one
    // sentence that actually answers the question.
    score / (sentence_terms.len() as f32).sqrt().max(1.0)
}

/// Extractive document overview: highest-idf-weighted sentences across the
/// whole document, in document order.
pub fn summarize(index: &BM25Index, max_sentences: usize) -> Vec<Statement> {
    let mut scored: Vec<(f32, Statement)> = Vec::new();
    for chunk in index.chunks() {
        for sentence in split_sentences(&chunk.text) {
            let terms = tokenize(&sentence);
            if terms.len() < 4 {
                continue;
            }
            let mut uniq = terms.clone();
            uniq.sort();
            uniq.dedup();
            let mut score = 0.0;
            for term in uniq {
                score += index.term_idf(&term);
            }
            score /= (terms.len() as f32).sqrt().max(1.0);
            scored.push((score, Statement {
                text: sentence,
                page: chunk.page,
            }));
        }
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(max_sentences);
    scored.sort_by_key(|(_, s)| s.page);
    scored.into_iter().map(|(_, s)| s).collect()
}
