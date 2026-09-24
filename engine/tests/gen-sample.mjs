// Generates tests/fixtures/sample.pdf — a minimal, valid two-page PDF with
// deliberately checkable facts (used by tests/validation.rs for the
// known-answer grounding tests). Zero deps; computes xref offsets itself.
//
//   node gen-sample.mjs

import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

const esc = (s) => s.replace(/\\/g, "\\\\").replace(/\(/g, "\\(").replace(/\)/g, "\\)");

const pageContent = (lines) => {
  const body = lines
    .map((l, i) => (i === 0 ? `(${esc(l)}) Tj` : `T*\n(${esc(l)}) Tj`))
    .join("\n");
  return `BT\n/F1 12 Tf\n14 TL\n72 720 Td\n${body}\nET`;
};

const page1Lines = [
  "Garden Shed Submains - Installation Guide",
  "This guide covers wiring a garden shed from a house switchboard.",
  "The maximum cable run from the house switchboard to the shed is 40 metres.",
  "Use 6 mm2 SDI cable for the submains.",
  "The shed circuit must be protected by a 32 A RCBO at the house switchboard.",
];

const page2Lines = [
  "Commissioning and Testing",
  "Before energising the shed circuit, measure the earth loop impedance.",
  "The earth loop impedance must be below 1.5 ohms.",
  "Record all results in the schedule of test results.",
  "A certificate of compliance must be issued when the work is complete.",
];

const content1 = pageContent(page1Lines);
const content2 = pageContent(page2Lines);

const objs = [
  "<< /Type /Catalog /Pages 2 0 R >>",
  "<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>",
  "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 7 0 R >> >> >>",
  { stream: content1 },
  "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 6 0 R /Resources << /Font << /F1 7 0 R >> >> >>",
  { stream: content2 },
  "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
];

let pdf = "%PDF-1.4\n";
const offsets = [];
objs.forEach((obj, i) => {
  offsets.push(pdf.length);
  const num = i + 1;
  if (typeof obj === "string") {
    pdf += `${num} 0 obj\n${obj}\nendobj\n`;
  } else {
    const bytes = Buffer.byteLength(obj.stream, "latin1");
    pdf += `${num} 0 obj\n<< /Length ${bytes} >>\nstream\n${obj.stream}\nendstream\nendobj\n`;
  }
});

const xrefStart = pdf.length;
const count = objs.length + 1;
pdf += `xref\n0 ${count}\n0000000000 65535 f \n`;
for (const off of offsets) {
  pdf += `${String(off).padStart(10, "0")} 00000 n \n`;
}
pdf += `trailer\n<< /Size ${count} /Root 1 0 R >>\nstartxref\n${xrefStart}\n%%EOF\n`;

const out = join(here, "fixtures");
mkdirSync(out, { recursive: true });
writeFileSync(join(out, "sample.pdf"), pdf, "latin1");
console.log(`wrote ${join(out, "sample.pdf")} (${pdf.length} bytes)`);
