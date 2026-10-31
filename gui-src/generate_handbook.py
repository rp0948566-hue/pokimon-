#!/usr/bin/env python3
"""
Master Builder: RetinaCare AI Screening Suite — Comprehensive Technical Architecture & Source Code Handbook (80 Pages)
"""

import os
import sys
import hashlib
import subprocess
from reportlab.lib.pagesizes import letter
from reportlab.lib import colors
from reportlab.platypus import (
    SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle, PageBreak, Preformatted, HRFlowable
)
from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
from reportlab.pdfgen import canvas

# Two-pass canvas for header/footer page numbering
class NumberedCanvas(canvas.Canvas):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._saved_page_states = []

    def showPage(self):
        self._saved_page_states.append(dict(self.__dict__))
        self._startPage()

    def save(self):
        num_pages = len(self._saved_page_states)
        for state in self._saved_page_states:
            self.__dict__.update(state)
            self.draw_page_decorations(num_pages)
            super().showPage()
        super().save()

    def draw_page_decorations(self, page_count):
        self.saveState()
        self.setFont("Helvetica-Bold", 7.5)
        self.setFillColor(colors.HexColor("#475569"))

        # Running Header (pages > 1)
        if self._pageNumber > 1:
            self.drawString(36, letter[1] - 24, "RETINACARE AI SCREENING SUITE — COMPREHENSIVE CODEBASE & ARCHITECTURE HANDBOOK")
            self.setFont("Helvetica", 7.5)
            self.drawRightString(letter[0] - 36, letter[1] - 24, "100% PURE NATIVE RUST (eframe/egui 0.37)")
            self.setStrokeColor(colors.HexColor("#CBD5E1"))
            self.setLineWidth(0.6)
            self.line(36, letter[1] - 28, letter[0] - 36, letter[1] - 28)

        # Running Footer (all pages)
        self.setFont("Helvetica", 7.5)
        self.setFillColor(colors.HexColor("#64748B"))
        footer_text = f"Page {self._pageNumber} of {page_count}"
        self.drawRightString(letter[0] - 36, 18, footer_text)
        self.drawString(36, 18, "CONFIDENTIAL & CLINICAL GRADE — FIPS 140-3 SHA-256 | Zero SQL Attack Surface | Zero WebView")
        self.setStrokeColor(colors.HexColor("#CBD5E1"))
        self.setLineWidth(0.6)
        self.line(36, 26, letter[0] - 36, 26)

        self.restoreState()

# Load all source files from disk
FILES = {
    "cargo": "rust/Cargo.toml",
    "domain": "rust/src/domain.rs",
    "security": "rust/src/security.rs",
    "engine_mod": "rust/src/engine/mod.rs",
    "engine_manifest": "rust/src/engine/manifest.rs",
    "engine_pdf": "rust/src/engine/pdf.rs",
    "ui_mod": "rust/src/ui/mod.rs",
    "ui_theme": "rust/src/ui/theme.rs",
    "ui_components": "rust/src/ui/components.rs",
    "ui_composer": "rust/src/ui/composer.rs",
    "plugins_icons": "rust/src/plugins/icons.rs",
    "plugins_pipeline": "rust/src/plugins/pipeline.rs",
    "plugins_mod": "rust/src/plugins/mod.rs",
    "main": "rust/src/main.rs",
}

SRC_LINES = {}
FILE_HASHES = {}
for k, path in FILES.items():
    if os.path.exists(path):
        with open(path, "r", encoding="utf-8") as f:
            SRC_LINES[k] = f.readlines()
        with open(path, "rb") as f:
            FILE_HASHES[k] = hashlib.sha256(f.read()).hexdigest()
    else:
        SRC_LINES[k] = []
        FILE_HASHES[k] = "N/A"

styles = getSampleStyleSheet()

STYLE_H1 = ParagraphStyle('DocH1', fontName='Helvetica-Bold', fontSize=18, leading=22, textColor=colors.HexColor('#0F172A'), spaceAfter=8)
STYLE_H2 = ParagraphStyle('DocH2', fontName='Helvetica-Bold', fontSize=10.5, leading=13.5, textColor=colors.HexColor('#0F172A'), spaceAfter=2)
STYLE_H3 = ParagraphStyle('DocH3', fontName='Helvetica-Bold', fontSize=8.5, leading=11, textColor=colors.HexColor('#1E293B'), spaceAfter=2)
STYLE_BODY = ParagraphStyle('DocBody', fontName='Helvetica', fontSize=7.2, leading=9.2, textColor=colors.HexColor('#334155'), spaceAfter=2)
STYLE_BODY_BOLD = ParagraphStyle('DocBodyBold', fontName='Helvetica-Bold', fontSize=7.2, leading=9.2, textColor=colors.HexColor('#0F172A'), spaceAfter=2)
STYLE_BADGE = ParagraphStyle('DocBadge', fontName='Helvetica-Bold', fontSize=6.5, leading=8.0, textColor=colors.HexColor('#2563EB'))
STYLE_CALLOUT = ParagraphStyle('DocCallout', fontName='Helvetica', fontSize=6.8, leading=8.8, textColor=colors.HexColor('#1E293B'))
STYLE_CODE = ParagraphStyle('DocCode', fontName='Courier', fontSize=5.8, leading=7.0, textColor=colors.HexColor('#E2E8F0'))
STYLE_CODE_COMPACT = ParagraphStyle('DocCodeCompact', fontName='Courier', fontSize=5.3, leading=6.3, textColor=colors.HexColor('#E2E8F0'))
STYLE_TH = ParagraphStyle('DocTH', fontName='Helvetica-Bold', fontSize=6.8, leading=8.5, textColor=colors.white)
STYLE_TD = ParagraphStyle('DocTD', fontName='Helvetica', fontSize=6.4, leading=8.0, textColor=colors.HexColor('#1E293B'))

def header_block(section_badge, page_title, subtitle=None):
    elems = []
    elems.append(Paragraph(f"<b><font color='#2563EB'>[{section_badge.upper()}]</font></b>", STYLE_BADGE))
    elems.append(Paragraph(page_title, STYLE_H2))
    if subtitle:
        elems.append(Paragraph(subtitle, STYLE_BODY))
    elems.append(HRFlowable(width="100%", thickness=0.6, color=colors.HexColor("#CBD5E1"), spaceAfter=3, spaceBefore=1))
    return elems

def code_box(key, start_line, end_line, max_chars=95, compact=False):
    lines = SRC_LINES.get(key, [])
    slice_lines = lines[start_line - 1 : end_line]
    n_lines = len(slice_lines)
    if n_lines == 0:
        return Spacer(1, 10)
    
    # Calculate dynamic font size and leading so it always fits safely within max 610 pt
    max_height = 580.0
    calc_leading = max_height / max(n_lines, 1)
    leading = min(7.0, max(3.8, calc_leading))
    font_size = max(3.2, min(5.8, leading * 0.82))

    formatted = []
    for idx, raw in enumerate(slice_lines, start=start_line):
        clean = raw.rstrip('\r\n')
        if len(clean) > max_chars:
            clean = clean[:max_chars - 3] + "..."
        formatted.append(f"{idx:4d} | {clean}")
    code_text = "\n".join(formatted)

    c_style = ParagraphStyle(
        f'CStyle_{key}_{start_line}_{end_line}',
        fontName='Courier',
        fontSize=font_size,
        leading=leading,
        textColor=colors.HexColor('#E2E8F0')
    )
    pref = Preformatted(code_text, c_style)
    t = Table([[pref]], colWidths=[540])
    t.setStyle(TableStyle([
        ('BACKGROUND', (0,0), (-1,-1), colors.HexColor('#0F172A')),
        ('BOX', (0,0), (-1,-1), 1, colors.HexColor('#334155')),
        ('TOPPADDING', (0,0), (-1,-1), 1.5),
        ('BOTTOMPADDING', (0,0), (-1,-1), 1.5),
        ('LEFTPADDING', (0,0), (-1,-1), 3),
        ('RIGHTPADDING', (0,0), (-1,-1), 3),
    ]))
    return t

def callout_box(title, text, color_hex="#2563EB", bg_hex="#EFF6FF"):
    p_title = Paragraph(f"<b>{title}</b>", ParagraphStyle('CT', fontName='Helvetica-Bold', fontSize=7.2, leading=9.0, textColor=colors.HexColor(color_hex)))
    p_body = Paragraph(text, STYLE_CALLOUT)
    t = Table([[p_title], [p_body]], colWidths=[540])
    t.setStyle(TableStyle([
        ('BACKGROUND', (0,0), (-1,-1), colors.HexColor(bg_hex)),
        ('LINEBEFORE', (0,0), (0,-1), 3, colors.HexColor(color_hex)),
        ('BOX', (0,0), (-1,-1), 0.5, colors.HexColor("#CBD5E1")),
        ('TOPPADDING', (0,0), (-1,-1), 2),
        ('BOTTOMPADDING', (0,0), (-1,-1), 2),
        ('LEFTPADDING', (0,0), (-1,-1), 4),
        ('RIGHTPADDING', (0,0), (-1,-1), 4),
    ]))
    return t

def make_table(headers, rows, col_widths=None):
    table_data = [[Paragraph(h, STYLE_TH) for h in headers]]
    for r in rows:
        row_cells = []
        for cell in r:
            if isinstance(cell, str):
                row_cells.append(Paragraph(cell, STYLE_TD))
            else:
                row_cells.append(cell)
        table_data.append(row_cells)
    t = Table(table_data, colWidths=col_widths)
    t.setStyle(TableStyle([
        ('BACKGROUND', (0,0), (-1,0), colors.HexColor('#1E293B')),
        ('GRID', (0,0), (-1,-1), 0.5, colors.HexColor('#CBD5E1')),
        ('ROWBACKGROUNDS', (0,1), (-1,-1), [colors.white, colors.HexColor('#F8FAFC')]),
        ('TOPPADDING', (0,0), (-1,-1), 1.5),
        ('BOTTOMPADDING', (0,0), (-1,-1), 1.5),
        ('LEFTPADDING', (0,0), (-1,-1), 3),
        ('RIGHTPADDING', (0,0), (-1,-1), 3),
    ]))
    return t

from handbook_s1 import get_pages_section1
from handbook_s2 import get_pages_section2
from handbook_s3 import get_pages_section3
from handbook_s4 import get_pages_section4
from handbook_s5 import get_pages_section5
from handbook_s6 import get_pages_section6
from handbook_s7 import get_pages_section7
from handbook_s8 import get_pages_section8
from handbook_s9 import get_pages_section9

def build_complete_handbook(output_pdf="docs/RetinaCare_Codebase_and_Architecture_Comprehensive_Handbook_80Pages.pdf"):
    os.makedirs(os.path.dirname(output_pdf), exist_ok=True)

    H = header_block
    C = code_box
    CB = callout_box
    T = make_table

    p_s1 = get_pages_section1(H, C, CB, T, SRC_LINES, FILE_HASHES)
    p_s2 = get_pages_section2(H, C, CB, T, SRC_LINES, FILE_HASHES)
    p_s3 = get_pages_section3(H, C, CB, T, SRC_LINES, FILE_HASHES)
    p_s4 = get_pages_section4(H, C, CB, T, SRC_LINES, FILE_HASHES)
    p_s5 = get_pages_section5(H, C, CB, T, SRC_LINES, FILE_HASHES)
    p_s6 = get_pages_section6(H, C, CB, T, SRC_LINES, FILE_HASHES)
    p_s7 = get_pages_section7(H, C, CB, T, SRC_LINES, FILE_HASHES)
    p_s8 = get_pages_section8(H, C, CB, T, SRC_LINES, FILE_HASHES)
    p_s9 = get_pages_section9(H, C, CB, T, SRC_LINES, FILE_HASHES)

    all_pages = p_s1 + p_s2 + p_s3 + p_s4 + p_s5 + p_s6 + p_s7 + p_s8 + p_s9
    print(f"Total structured pages planned: {len(all_pages)}")

    story = []
    for idx, page_elems in enumerate(all_pages, start=1):
        story.extend(page_elems)
        if idx < len(all_pages):
            story.append(PageBreak())

    doc = SimpleDocTemplate(
        output_pdf,
        pagesize=letter,
        leftMargin=36,
        rightMargin=36,
        topMargin=32,
        bottomMargin=32
    )

    print(f"Building PDF: {output_pdf}...")
    doc.build(story, canvasmaker=NumberedCanvas)
    print("PDF build complete!")

    # Verify with pdfinfo
    try:
        out = subprocess.check_output(['pdfinfo', output_pdf]).decode()
        for line in out.splitlines():
            if 'Pages:' in line or 'File size:' in line:
                print(f"PDFINFO: {line}")
    except Exception as e:
        print("pdfinfo error:", e)

if __name__ == "__main__":
    out_file = "docs/RetinaCare_Codebase_and_Architecture_Comprehensive_Handbook_80Pages.pdf"
    if len(sys.argv) > 1:
        out_file = sys.argv[1]
    build_complete_handbook(out_file)
