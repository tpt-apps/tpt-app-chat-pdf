//! Known-answer validation: questions against the committed sample PDF
//! (see gen-sample.mjs) must return grounded answers — the right fact from
//! the right page — and never content that isn't in the document.

use tpt_chatpdf_engine::{ChatPdfEngine, EngineError};

const SAMPLE: &[u8] = include_bytes!("fixtures/sample.pdf");

/// Ground truth for the no-hallucination check: the document's text as the
/// same parser the engine uses extracts it. Statements are sentence
/// substrings of whitespace-normalised chunk text, so compare with
/// whitespace collapsed.
fn document_text() -> String {
    let pages = pdf_extract::extract_text_from_mem_by_pages(SAMPLE)
        .expect("fixture extracts as ground truth");
    pages.join("\n")
}

fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase()
}

#[test]
fn indexes_the_sample_pdf() {
    let engine = ChatPdfEngine::from_pdf_bytes(SAMPLE).expect("sample pdf indexes");
    assert_eq!(engine.page_count(), 2);
    assert!(engine.chunk_count() >= 2, "expected multiple chunks");
}

#[test]
fn known_answer_max_cable_run() {
    let engine = ChatPdfEngine::from_pdf_bytes(SAMPLE).unwrap();
    let answer = engine.ask("What is the maximum cable run from the house switchboard to the shed?", 4);
    assert!(!answer.statements.is_empty(), "expected at least one statement");
    let top = &answer.statements[0];
    assert!(
        norm(&top.text).contains("40 metres"),
        "top statement should contain the known fact, got: {top:?}"
    );
    assert_eq!(top.page, 1, "the fact is on page 1");
    assert!(
        answer.passages.iter().any(|p| p.page == 1),
        "page 1 passage should be cited among the passages"
    );
}

#[test]
fn known_answer_earth_loop_impedance() {
    let engine = ChatPdfEngine::from_pdf_bytes(SAMPLE).unwrap();
    let answer = engine.ask("What must the earth loop impedance be below?", 4);
    assert!(!answer.statements.is_empty(), "expected at least one statement");
    let top = &answer.statements[0];
    assert!(
        norm(&top.text).contains("1.5 ohms"),
        "top statement should contain the known fact, got: {top:?}"
    );
    assert_eq!(top.page, 2, "the fact is on page 2");
}

#[test]
fn known_answer_rcbo_protection() {
    let engine = ChatPdfEngine::from_pdf_bytes(SAMPLE).unwrap();
    let answer = engine.ask("What device protects the shed circuit?", 4);
    assert!(!answer.statements.is_empty(), "expected at least one statement");
    let top = &answer.statements[0];
    assert!(
        norm(&top.text).contains("32 a rcbo"),
        "top statement should contain the known fact, got: {top:?}"
    );
}

#[test]
fn answers_are_extractive_on_every_question() {
    let engine = ChatPdfEngine::from_pdf_bytes(SAMPLE).unwrap();
    let ground_truth = norm(&document_text());
    for question in [
        "What is the maximum cable run from the house switchboard to the shed?",
        "What must the earth loop impedance be below?",
        "What device protects the shed circuit?",
        "What cable should be used for the submains?",
    ] {
        let answer = engine.ask(question, 4);
        for stmt in &answer.statements {
            assert!(
                ground_truth.contains(&norm(&stmt.text)),
                "statement not found in document (hallucination?): {stmt:?}"
            );
        }
    }
}

#[test]
fn summary_returns_page_ordered_statements() {
    let engine = ChatPdfEngine::from_pdf_bytes(SAMPLE).unwrap();
    let summary = engine.summary(4);
    assert_eq!(summary.len(), 4);
    let pages: Vec<usize> = summary.iter().map(|s| s.page).collect();
    let mut sorted = pages.clone();
    sorted.sort_unstable();
    assert_eq!(pages, sorted, "summary statements must be in document order");
}

#[test]
fn textless_pdf_reports_no_extractable_text() {
    // A syntactically valid PDF (proper xref offsets) whose single page has
    // an empty content stream — the scanned-document-without-text case.
    let pdf = build_textless_pdf();
    match ChatPdfEngine::from_pdf_bytes(pdf.as_bytes()) {
        Err(EngineError::NoExtractableText { page_count }) => assert_eq!(page_count, 1),
        Err(e) => panic!("expected NoExtractableText, got: {e}"),
        Ok(_) => panic!("expected NoExtractableText, got an index"),
    }
}

/// Builds a minimal valid one-page PDF with an empty content stream — the
/// same xref-offset arithmetic gen-sample.mjs uses for the real fixture.
fn build_textless_pdf() -> String {
    let objs: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>".into(),
        "<< /Length 0 >>\nstream\n\nendstream".into(),
    ];
    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, obj) in objs.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.push_str(&format!("{} 0 obj\n{}\nendobj\n", i + 1, obj));
    }
    let xref_start = pdf.len();
    pdf.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1));
    for off in &offsets {
        pdf.push_str(&format!("{:010} 00000 n \n", off));
    }
    pdf.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        objs.len() + 1,
        xref_start
    ));
    pdf
}

#[test]
fn garbage_bytes_are_a_pdf_error_not_a_panic() {
    let err = ChatPdfEngine::from_pdf_bytes(b"not a pdf at all")
        .err()
        .expect("garbage must not index");
    assert!(matches!(err, EngineError::Pdf(_)), "unexpected: {err}");
}
