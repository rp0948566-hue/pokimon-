# Section 7: Extensible Plugins Deck & MCP Tools (Pages 53-59)
from reportlab.lib import colors
from reportlab.platypus import Paragraph, Spacer, Table, TableStyle, HRFlowable, Preformatted
from reportlab.lib.styles import ParagraphStyle

def get_pages_section7(H, C, CB, T, src, hashes):
    pages = []

    # PAGE 53: plugins/mod.rs Architecture
    p53 = []
    p53.extend(H("Plugin Architecture", "52. rust/src/plugins/mod.rs — Extensibility Registry & MCP Architecture", "Model Context Protocol tool execution, plugins deck UI modal, and async micro-agents"))
    p53.append(Paragraph("The <code>src/plugins/mod.rs</code> module (754 lines) implements the complete Model Context Protocol (MCP) registry and interactive card deck UI modal for RetinaCare:",
                         ParagraphStyle('P53_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    deck_features = [
        ["Subsystem Element", "Lines in File", "Role & Responsibility"],
        ["Tool Data Structures", "Lines 1 – 125", "Defines PluginCard, ToolCategory, ConnectionStatus, and MCP tool input/output schemas."],
        ["Catalog Registry", "Lines 126 – 250", "Initializes the built-in clinical plugin catalog with verified diagnostic and research tools."],
        ["Filter & Search", "Lines 251 – 380", "Implements real-time text query filtering and category tabs (Vision, Research, Clinical)."],
        ["Card UI Renderer", "Lines 381 – 510", "Renders rich visual cards with vector icons, status badges, and toggle connection switches."],
        ["MCP Tool Dispatcher", "Lines 511 – 640", "Coordinates JSON-RPC tool calls, input validation, and asynchronous execution channels."],
        ["Persistence & Tests", "Lines 641 – 754", "Saves tool states to encrypted config; includes comprehensive unit test suite."]
    ]
    p53.append(T(deck_features[0], deck_features[1:], [110, 80, 350]))
    p53.append(Spacer(1, 4))
    p53.append(CB("SAFE ASYNCHRONOUS TOOL EXECUTION",
                  "All MCP tools run in sandboxed asynchronous threads. A crashing plugin cannot bring down the main RetinaCare application; errors are caught and surfaced gracefully in the diagnostic deck.",
                  "#2563EB", "#EFF6FF"))
    pages.append(p53)

    # PAGE 54: plugins/mod.rs Source Code (Lines 1 - 125)
    p54 = []
    p54.extend(H("Plugin Architecture", "53. rust/src/plugins/mod.rs — Source Code (Lines 1 – 125)", "PluginCard data models, connection status enums, and schema definitions"))
    p54.append(Paragraph("The code below implements plugin data models and connection status tracking:",
                         ParagraphStyle('P54_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p54.append(C("plugins_mod", 1, 125, max_chars=95, compact=True))
    pages.append(p54)

    # PAGE 55: plugins/mod.rs Source Code (Lines 126 - 250)
    p55 = []
    p55.extend(H("Plugin Architecture", "54. rust/src/plugins/mod.rs — Source Code (Lines 126 – 250)", "Catalog registry initialization and built-in clinical tools seed list"))
    p55.append(Paragraph("The code below initializes the default clinical tool catalog:",
                         ParagraphStyle('P55_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p55.append(C("plugins_mod", 126, 250, max_chars=95, compact=True))
    pages.append(p55)

    # PAGE 56: plugins/mod.rs Source Code (Lines 251 - 380)
    p56 = []
    p56.extend(H("Plugin Architecture", "55. rust/src/plugins/mod.rs — Source Code (Lines 251 – 380)", "Catalog search, real-time query filtering, and category tab logic"))
    p56.append(Paragraph("The code below implements catalog search and category filtering:",
                         ParagraphStyle('P56_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p56.append(C("plugins_mod", 251, 380, max_chars=95, compact=True))
    pages.append(p56)

    # PAGE 57: plugins/mod.rs Source Code (Lines 381 - 510)
    p57 = []
    p57.extend(H("Plugin Architecture", "56. rust/src/plugins/mod.rs — Source Code (Lines 381 – 510)", "Card UI rendering, vector icon badges, and toggle switch handlers"))
    p57.append(Paragraph("The code below renders interactive plugin cards and handles connection toggles:",
                         ParagraphStyle('P57_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p57.append(C("plugins_mod", 381, 510, max_chars=95, compact=True))
    pages.append(p57)

    # PAGE 58: plugins/mod.rs Source Code (Lines 511 - 640)
    p58 = []
    p58.extend(H("Plugin Architecture", "57. rust/src/plugins/mod.rs — Source Code (Lines 511 – 640)", "MCP tool dispatch, JSON-RPC communication, and async execution worker"))
    p58.append(Paragraph("The code below executes MCP tool calls asynchronously:",
                         ParagraphStyle('P58_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p58.append(C("plugins_mod", 511, 640, max_chars=95, compact=True))
    pages.append(p58)

    # PAGE 59: plugins/mod.rs Source Code (Lines 641 - 754)
    p59 = []
    p59.extend(H("Plugin Architecture", "58. rust/src/plugins/mod.rs — Source Code (Lines 641 – 754)", "State persistence, tool configuration serialization, and unit test suite"))
    p59.append(Paragraph("The code below serializes plugin configurations and runs registry unit tests:",
                         ParagraphStyle('P59_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p59.append(C("plugins_mod", 641, 754, max_chars=95, compact=True))
    pages.append(p59)

    return pages

print("Section 7 generator defined.")
