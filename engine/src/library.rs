//! Pro-edition multi-document library: several indexed PDFs queried
//! together, answers citing document + page.

use crate::answer::{self, Statement};
use crate::error::EngineError;
use crate::ChatPdfEngine;

/// One document in the library, as the UI's library manager shows it.
#[derive(Clone)]
pub struct LibraryDoc {
    pub name: String,
    engine: ChatPdfEngine,
}

/// A named collection of documents asked as one corpus.
#[derive(Clone, Default)]
pub struct ChatPdfLibrary {
    docs: Vec<LibraryDoc>,
}

/// A [`ChatPdfLibrary::ask`] answer: statements carry the source document's
/// name alongside the page.
#[derive(Debug, Clone)]
pub struct LibraryAnswer {
    pub question: String,
    pub statements: Vec<LibraryStatement>,
    pub documents: Vec<String>,
}

/// A statement quoted from a named document.
#[derive(Debug, Clone)]
pub struct LibraryStatement {
    pub document: String,
    pub page: usize,
    pub text: String,
}

impl ChatPdfLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    /// Extracts and indexes another PDF into the library; returns
    /// (indexed pages, total pages in the file) so callers can report
    /// "first 40 of 214 pages".
    pub fn add_pdf(
        &mut self,
        name: &str,
        bytes: &[u8],
    ) -> Result<(usize, usize), EngineError> {
        self.add_pdf_capped(name, bytes, usize::MAX)
    }

    /// [`Self::add_pdf`] with a page cap (the free tier's 40-page limit).
    pub fn add_pdf_capped(
        &mut self,
        name: &str,
        bytes: &[u8],
        max_pages: usize,
    ) -> Result<(usize, usize), EngineError> {
        let (engine, total_pages) = ChatPdfEngine::from_pdf_bytes_capped(bytes, max_pages)?;
        let indexed = engine.page_count();
        self.docs.push(LibraryDoc {
            name: name.to_string(),
            engine,
        });
        Ok((indexed, total_pages))
    }

    pub fn len(&self) -> usize {
        self.docs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// Document names in library order (the library manager's listing).
    pub fn document_names(&self) -> Vec<String> {
        self.docs.iter().map(|d| d.name.clone()).collect()
    }

    /// Removes a document by name; returns whether anything was removed.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.docs.len();
        self.docs.retain(|d| d.name != name);
        self.docs.len() != before
    }

    /// Asks the whole library: every document is queried independently and
    /// the best statements across all of them win, each citing its document
    /// and page.
    pub fn ask(&self, question: &str, top_k_per_doc: usize) -> LibraryAnswer {
        let mut scored: Vec<(f32, LibraryStatement)> = Vec::new();
        for doc in &self.docs {
            // The per-doc statement scores are what answer() ranks
            // internally; re-derive them here by asking each engine and
            // re-scoring, so cross-document ordering is on one scale.
            let answer = doc.engine.ask(question, top_k_per_doc);
            for statement in answer.statements {
                let score = answer::score_statement(doc.engine.index(), question, &statement.text);
                scored.push((score, LibraryStatement {
                    document: doc.name.clone(),
                    page: statement.page,
                    text: statement.text,
                }));
            }
        }
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.dedup_by(|a, b| a.1.document == b.1.document && a.1.text == b.1.text);
        scored.truncate(5);

        let mut documents: Vec<String> = Vec::new();
        for (_, statement) in &scored {
            if !documents.contains(&statement.document) {
                documents.push(statement.document.clone());
            }
        }

        LibraryAnswer {
            question: question.to_string(),
            statements: scored.into_iter().map(|(_, s)| s).collect(),
            documents,
        }
    }

    /// Extractive summary of every document, grouped in library order —
    /// the payload behind the Pro "Export summary" action.
    pub fn summarize(&self, sentences_per_doc: usize) -> Vec<(String, Vec<Statement>)> {
        self.docs
            .iter()
            .map(|d| (d.name.clone(), d.engine.summary(sentences_per_doc)))
            .collect()
    }
}
