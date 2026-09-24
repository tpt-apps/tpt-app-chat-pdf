//! Pro-library validation: two documents in one library, questions that
//! each have their answer in a different document, plus a question whose
//! best evidence spans both.

use tpt_chatpdf_engine::ChatPdfLibrary;

const SAMPLE: &[u8] = include_bytes!("fixtures/sample.pdf");

/// A second fixture built with the same generator logic as sample.pdf but
/// different facts (heat pump content), so cross-document attribution is
/// provable. Written inline via the same xref arithmetic the generator
/// script uses.
fn heat_pump_pdf() -> Vec<u8> {
    let page_lines = [
        "Heat Pump Service Notes",
        "The recommended clearance around an outdoor heat pump unit is 300 mm.",
        "Replace the air filter every three months under normal use.",
        "Operating range for heating is minus 15 to 24 degrees Celsius.",
    ];
    let esc = |s: &str| s.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)");
    let body = page_lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            if i == 0 {
                format!("({}) Tj", esc(l))
            } else {
                format!("T*\n({}) Tj", esc(l))
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!("BT\n/F1 12 Tf\n14 TL\n72 720 Td\n{body}\nET");
    let objs: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".into(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into(),
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
    pdf.into_bytes()
}

fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase()
}

#[test]
fn library_indexes_and_lists_documents() {
    let mut lib = ChatPdfLibrary::new();
    lib.add_pdf("shed-submains.pdf", SAMPLE).expect("shed indexes");
    lib.add_pdf("heat-pump.pdf", &heat_pump_pdf()).expect("heat pump indexes");
    assert_eq!(lib.len(), 2);
    assert_eq!(
        lib.document_names(),
        vec!["shed-submains.pdf".to_string(), "heat-pump.pdf".to_string()]
    );
}

#[test]
fn cross_document_ask_finds_each_document() {
    let mut lib = ChatPdfLibrary::new();
    lib.add_pdf("shed-submains.pdf", SAMPLE).unwrap();
    lib.add_pdf("heat-pump.pdf", &heat_pump_pdf()).unwrap();

    let shed = lib.ask("What is the maximum cable run to the shed?", 4);
    assert!(!shed.statements.is_empty());
    assert!(
        norm(&shed.statements[0].text).contains("40 metres"),
        "expected the shed fact, got: {:?}",
        shed.statements[0]
    );
    assert_eq!(shed.statements[0].document, "shed-submains.pdf");

    let pump = lib.ask("How often should the air filter be replaced?", 4);
    assert!(!pump.statements.is_empty());
    assert!(
        norm(&pump.statements[0].text).contains("every three months"),
        "expected the heat pump fact, got: {:?}",
        pump.statements[0]
    );
    assert_eq!(pump.statements[0].document, "heat-pump.pdf");
}

#[test]
fn library_statements_stay_extractive() {
    let pump = heat_pump_pdf();
    let mut lib = ChatPdfLibrary::new();
    lib.add_pdf("shed-submains.pdf", SAMPLE).unwrap();
    lib.add_pdf("heat-pump.pdf", &pump).unwrap();
    // Same no-hallucination ground truth as validation.rs: the statements
    // must appear in text extracted from the source documents.
    let shed_text = norm(
        &pdf_extract::extract_text_from_mem_by_pages(SAMPLE)
            .expect("shed extracts")
            .join("\n"),
    );
    let pump_text = norm(
        &pdf_extract::extract_text_from_mem_by_pages(&pump)
            .expect("heat pump extracts")
            .join("\n"),
    );
    for question in [
        "What is the maximum cable run to the shed?",
        "How often should the air filter be replaced?",
        "What clearance does the heat pump need?",
        "What device protects the shed circuit?",
    ] {
        let answer = lib.ask(question, 4);
        for statement in &answer.statements {
            let source = if statement.document == "shed-submains.pdf" {
                &shed_text
            } else {
                &pump_text
            };
            assert!(
                source.contains(&norm(&statement.text)),
                "statement not found in {}: {statement:?}",
                statement.document
            );
        }
    }
}

#[test]
fn remove_document_by_name() {
    let mut lib = ChatPdfLibrary::new();
    lib.add_pdf("shed-submains.pdf", SAMPLE).unwrap();
    lib.add_pdf("heat-pump.pdf", &heat_pump_pdf()).unwrap();
    assert!(lib.remove("shed-submains.pdf"));
    assert!(!lib.remove("shed-submains.pdf"), "second remove is a no-op");
    assert_eq!(lib.len(), 1);
    let pump = lib.ask("What clearance does the heat pump need?", 4);
    assert_eq!(pump.statements[0].document, "heat-pump.pdf");
}

#[test]
fn summary_is_grouped_per_document() {
    let mut lib = ChatPdfLibrary::new();
    lib.add_pdf("shed-submains.pdf", SAMPLE).unwrap();
    lib.add_pdf("heat-pump.pdf", &heat_pump_pdf()).unwrap();
    let summary = lib.summarize(3);
    assert_eq!(summary.len(), 2);
    assert_eq!(summary[0].0, "shed-submains.pdf");
    assert_eq!(summary[1].0, "heat-pump.pdf");
    assert!(summary[0].1.len() <= 3);
    assert!(summary[1].1.len() <= 3);
}
