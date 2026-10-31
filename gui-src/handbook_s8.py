# Section 8: Core Application Orchestrator (rust/src/main.rs) (Pages 60-76)
from reportlab.lib import colors
from reportlab.platypus import Paragraph, Spacer, Table, TableStyle, HRFlowable, Preformatted
from reportlab.lib.styles import ParagraphStyle

def get_pages_section8(H, C, CB, T, src, hashes):
    pages = []

    # PAGE 60: main.rs Architecture
    p60 = []
    p60.extend(H("Application Core", "59. rust/src/main.rs — Architecture & Master App State Machine", "Event loop lifecycle, eframe orchestration, consultation navigation, and layout dispatch"))
    p60.append(Paragraph("<code>rust/src/main.rs</code> (2,243 lines) is the central orchestration hub of RetinaCare. It integrates the presentation layer, diagnostic engines, cryptographic security, and domain state into a single cohesive, high-performance desktop application:",
                         ParagraphStyle('P60_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    main_sections = [
        ["Architectural Component", "Lines in File", "Core Responsibility & State Handlers"],
        ["Imports & Global Constants", "Lines 1 – 135", "Crate imports, window size defaults (1280x800), and color token definitions."],
        ["RetinaCareApp Struct", "Lines 136 – 270", "Central state container holding consultations, active index, image texture, and viewport."],
        ["Default Session Initializer", "Lines 271 – 405", "Populates initial clinical consultation history and generates 4-word preview titles."],
        ["Multimodal File Ingestion", "Lines 406 – 540", "Handles image drag-and-drop, validates 10MB bounds, and invokes EXIF scrubber."],
        ["GPU Texture Upload", "Lines 541 – 675", "Decodes sanitized pixel buffer and transfers RGBA texture to GPU memory via egui."],
        ["Sidebar & Manila Yellow Cards", "Lines 676 – 945", "Renders docked sessions list with Manila Yellow active highlights (#FEF9C3)."],
        ["Header Bar & Clean Reset", "Lines 946 – 1080", "Top navigation bar; handles Home click to reset session state cleanly."],
        ["Chat Panel & Adaptive Layout", "Lines 1081 – 1350", "Renders message history with adaptive bubble widths, avatars, and timestamps."],
        ["Retinal Image Viewport", "Lines 1351 – 1485", "Interactive fundus photograph viewer with 0.2x–5.0x zoom and pan controls."],
        ["AI Diagnostic Inference", "Lines 1486 – 1755", "Executes 5-stage HierarchiRetina manifest and updates clinical progress gauges."],
        ["Plugins Deck & Export", "Lines 1756 – 2025", "Modal dialog for MCP tools and native vector PDF clinical report export."],
        ["Automated Test Suite", "Lines 2026 – 2243", "Unit tests for session switching, viewer reset, and bubble width calculations."]
    ]
    p60.append(T(main_sections[0], main_sections[1:], [130, 80, 330]))
    p60.append(Spacer(1, 4))
    p60.append(CB("DETERMINISTIC SINGLE-THREADED GUI MODEL",
                  "eframe executes the entire UI frame lifecycle deterministically on the main OS thread. Heavy computations (AI inference, image decoding, cryptographic hashing) run in background worker threads, preventing GUI frame drops.",
                  "#059669", "#ECFDF5"))
    pages.append(p60)

    # PAGE 61: main.rs Lines 1 - 135
    p61 = []
    p61.extend(H("Application Core", "60. rust/src/main.rs — Source Code (Lines 1 – 135)", "Imports, global constants, and NativeOptions window initialization"))
    p61.append(Paragraph("The code below establishes crate imports and eframe window configuration:",
                         ParagraphStyle('P61_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p61.append(C("main", 1, 135, max_chars=95, compact=True))
    pages.append(p61)

    # PAGE 62: main.rs Lines 136 - 270
    p62 = []
    p62.extend(H("Application Core", "61. rust/src/main.rs — Source Code (Lines 136 – 270)", "RetinaCareApp struct definition, field declarations, and Default implementation"))
    p62.append(Paragraph("The code below defines the core application struct and state members:",
                         ParagraphStyle('P62_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p62.append(C("main", 136, 270, max_chars=95, compact=True))
    pages.append(p62)

    # PAGE 63: main.rs Lines 271 - 405
    p63 = []
    p63.extend(H("Application Core", "62. rust/src/main.rs — Source Code (Lines 271 – 405)", "Default consultation initialization, seed sessions, and 4-word preview titles"))
    p63.append(Paragraph("The code below initializes default patient consultations and history records:",
                         ParagraphStyle('P63_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p63.append(C("main", 271, 405, max_chars=95, compact=True))
    pages.append(p63)

    # PAGE 64: main.rs Lines 406 - 540
    p64 = []
    p64.extend(H("Application Core", "63. rust/src/main.rs — Source Code (Lines 406 – 540)", "Multimodal file ingestion, drag-and-drop handling, and EXIF scrubbing invocation"))
    p64.append(Paragraph("The code below handles image drops and executes metadata sanitization:",
                         ParagraphStyle('P64_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p64.append(C("main", 406, 540, max_chars=95, compact=True))
    pages.append(p64)

    # PAGE 65: main.rs Lines 541 - 675
    p65 = []
    p65.extend(H("Application Core", "64. rust/src/main.rs — Source Code (Lines 541 – 675)", "GPU texture generation, aspect ratio calculation, and memory caching"))
    p65.append(Paragraph("The code below decodes sanitized pixel data and uploads textures to the GPU:",
                         ParagraphStyle('P65_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p65.append(C("main", 541, 675, max_chars=95, compact=True))
    pages.append(p65)

    # PAGE 66: main.rs Lines 676 - 810
    p66 = []
    p66.extend(H("Application Core", "65. rust/src/main.rs — Source Code (Lines 676 – 810)", "Left sidebar navigation, session switching, and consultation list layout"))
    p66.append(Paragraph("The code below renders the left navigation sidebar and session controls:",
                         ParagraphStyle('P66_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p66.append(C("main", 676, 810, max_chars=95, compact=True))
    pages.append(p66)

    # PAGE 67: main.rs Lines 811 - 945
    p67 = []
    p67.extend(H("Application Core", "66. rust/src/main.rs — Source Code (Lines 811 – 945)", "Manila Yellow consultation card rendering and active state styling"))
    p67.append(Paragraph("The code below renders Manila Yellow active cards (#FEF9C3) and inactive cards:",
                         ParagraphStyle('P67_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p67.append(C("main", 811, 945, max_chars=95, compact=True))
    pages.append(p67)

    # PAGE 68: main.rs Lines 946 - 1080
    p68 = []
    p68.extend(H("Application Core", "67. rust/src/main.rs — Source Code (Lines 946 – 1080)", "Top header navigation bar, consultation reset triggers, and clean state handlers"))
    p68.append(Paragraph("The code below handles top-bar controls and the clean Home reset action:",
                         ParagraphStyle('P68_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p68.append(C("main", 946, 1080, max_chars=95, compact=True))
    pages.append(p68)

    # PAGE 69: main.rs Lines 1081 - 1215
    p69 = []
    p69.extend(H("Application Core", "68. rust/src/main.rs — Source Code (Lines 1081 – 1215)", "Central chat scroll area, adaptive message bubbles, and text wrap logic"))
    p69.append(Paragraph("The code below renders the clinical dialogue scroll viewport:",
                         ParagraphStyle('P69_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p69.append(C("main", 1081, 1215, max_chars=95, compact=True))
    pages.append(p69)

    # PAGE 70: main.rs Lines 1216 - 1350
    p70 = []
    p70.extend(H("Application Core", "69. rust/src/main.rs — Source Code (Lines 1216 – 1350)", "Message bubble styling, author avatars, and timestamp formatting"))
    p70.append(Paragraph("The code below styles assistant and user chat bubbles with clean headers:",
                         ParagraphStyle('P70_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p70.append(C("main", 1216, 1350, max_chars=95, compact=True))
    pages.append(p70)

    # PAGE 71: main.rs Lines 1351 - 1485
    p71 = []
    p71.extend(H("Application Core", "70. rust/src/main.rs — Source Code (Lines 1351 – 1485)", "Retinal image viewport, continuous zoom (0.2x–5.0x), and pan drag controls"))
    p71.append(Paragraph("The code below handles retinal image pan, zoom, and coordinate translation:",
                         ParagraphStyle('P71_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p71.append(C("main", 1351, 1485, max_chars=95, compact=True))
    pages.append(p71)

    # PAGE 72: main.rs Lines 1486 - 1620
    p72 = []
    p72.extend(H("Application Core", "71. rust/src/main.rs — Source Code (Lines 1486 – 1620)", "Hierarchical AI inference execution, stage progress bars, and confidence gauges"))
    p72.append(Paragraph("The code below updates the 5-stage HierarchiRetina progress deck:",
                         ParagraphStyle('P72_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p72.append(C("main", 1486, 1620, max_chars=95, compact=True))
    pages.append(p72)

    # PAGE 73: main.rs Lines 1621 - 1755
    p73 = []
    p73.extend(H("Application Core", "72. rust/src/main.rs — Source Code (Lines 1621 – 1755)", "Multi-agent clinical consensus panel and diagnostic findings synthesis"))
    p73.append(Paragraph("The code below synthesizes multi-agent specialist findings into clinical recommendations:",
                         ParagraphStyle('P73_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p73.append(C("main", 1621, 1755, max_chars=95, compact=True))
    pages.append(p73)

    # PAGE 74: main.rs Lines 1756 - 1890
    p74 = []
    p74.extend(H("Application Core", "73. rust/src/main.rs — Source Code (Lines 1756 – 1890)", "Plugins deck modal window, tool search filter, and toggle switch grid"))
    p74.append(Paragraph("The code below renders the extensible plugins modal window:",
                         ParagraphStyle('P74_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p74.append(C("main", 1756, 1890, max_chars=95, compact=True))
    pages.append(p74)

    # PAGE 75: main.rs Lines 1891 - 2025
    p75 = []
    p75.extend(H("Application Core", "74. rust/src/main.rs — Source Code (Lines 1891 – 2025)", "Native report export triggers, action center buttons, and keyboard shortcuts"))
    p75.append(Paragraph("The code below handles report generation triggers and global keyboard hotkeys:",
                         ParagraphStyle('P75_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p75.append(C("main", 1891, 2025, max_chars=95, compact=True))
    pages.append(p75)

    # PAGE 76: main.rs Lines 2026 - 2243
    p76 = []
    p76.extend(H("Application Core", "75. rust/src/main.rs — Source Code (Lines 2026 – 2243)", "Comprehensive unit test suite for main orchestrator, docking, and state transitions"))
    p76.append(Paragraph("The code below contains the main orchestrator's automated unit test suite:",
                         ParagraphStyle('P76_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p76.append(C("main", 2026, 2160, max_chars=95, compact=True))
    pages.append(p76)

    return pages

print("Section 8 generator defined.")
