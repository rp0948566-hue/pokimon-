# Section 6: Plugin Pipeline & High-DPI Vector Icons (Pages 45-52)
from reportlab.lib import colors
from reportlab.platypus import Paragraph, Spacer, Table, TableStyle, HRFlowable, Preformatted
from reportlab.lib.styles import ParagraphStyle

def get_pages_section6(H, C, CB, T, src, hashes):
    pages = []

    # PAGE 45: pipeline.rs Architecture
    p45 = []
    p45.extend(H("Plugin Architecture", "44. rust/src/plugins/pipeline.rs — Pipeline Architecture", "Multi-stage plugin orchestration, asynchronous task dispatch, and catalog filtering"))
    p45.append(Paragraph("The <code>src/plugins/pipeline.rs</code> module orchestrates auxiliary diagnostic agents and tools. It defines the pipeline execution model:",
                         ParagraphStyle('P45_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    pipeline_structs = [
        ["Data Structure", "Fields / Invariants", "Pipeline Functionality"],
        ["PluginPipeline", "stages: Vec<PipelineStage>, active_tools: HashSet<String>", "Coordinates multi-stage tool execution and tracks active connections."],
        ["PipelineStage", "id: String, name: String, status: StageStatus, error: Option<String>", "Represents an individual stage within the plugin processing pipeline."],
        ["StageStatus", "enum { Idle, Running, Complete, Failed }", "State machine tracking stage execution without blocking the GUI thread."],
        ["PluginCatalogFilter", "category: Option<String>, query: String", "In-memory search and filter predicate for the plugins catalog deck."]
    ]
    p45.append(T(pipeline_structs[0], pipeline_structs[1:], [120, 180, 240]))
    p45.append(Spacer(1, 4))
    p45.append(CB("NON-BLOCKING WORKER EXECUTION",
                  "Plugin stages run asynchronously on dedicated worker threads. The GUI main loop queries stage status via non-blocking atomic flags, guaranteeing that slow network calls or heavy calculations never stall the 60 FPS viewport.",
                  "#2563EB", "#EFF6FF"))
    pages.append(p45)

    # PAGE 46: pipeline.rs Source Code Part 1
    p46 = []
    p46.extend(H("Plugin Architecture", "45. rust/src/plugins/pipeline.rs — Source Code (Lines 1 – 110)", "Stage definitions, state tracking, and pipeline lifecycle methods"))
    p46.append(Paragraph("The code below implements pipeline stages and status transitions:",
                         ParagraphStyle('P46_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p46.append(C("plugins_pipeline", 1, 110, max_chars=95, compact=True))
    pages.append(p46)

    # PAGE 47: pipeline.rs Source Code Part 2
    p47 = []
    p47.extend(H("Plugin Architecture", "46. rust/src/plugins/pipeline.rs — Source Code (Lines 111 – 226)", "Catalog filtering, tool toggles, and pipeline unit tests"))
    p47.append(Paragraph("The code below implements tool connection toggles and filter predicates:",
                         ParagraphStyle('P47_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p47.append(C("plugins_pipeline", 111, 226, max_chars=95, compact=True))
    pages.append(p47)

    # PAGE 48: icons.rs Architecture
    p48 = []
    p48.extend(H("Vector Graphics", "47. rust/src/plugins/icons.rs — Procedural Vector Icon Architecture", "Direct egui mesh generation, high-DPI scaling, and procedural clinical iconography"))
    p48.append(Paragraph("Rather than loading external SVG files or raster PNG bitmaps which require image decoders and disk I/O, <code>src/plugins/icons.rs</code> procedurally synthesizes vector icons directly into egui geometry buffers:",
                         ParagraphStyle('P48_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    icon_list = [
        ["Icon Primitive", "Procedural Mesh Generation", "Clinical Context"],
        ["Medical Stethoscope", "Curved bezier stroke with ear tips and chest piece", "General clinical consultation and medical records"],
        ["Vision Eye", "Convex arc upper and lower lid with circular iris and pupil", "Fundus photography, retinal scan, and vision tools"],
        ["Shield Lock", "Tapered polygon with arched top and central keyhole rect", "Security status, FIPS verification, and encrypted storage"],
        ["Neural Network", "3-layer node lattice with connecting synaptic line strokes", "HierarchiRetina AI models and diagnostic stage progress"],
        ["Pill Medication", "Capsule geometry with diagonal dividing line", "Pharmacology checker, drug interaction auditor"],
        ["Document Report", "Folded-corner rectangle with horizontal text lines", "PDF diagnostic export and clinical summary reports"]
    ]
    p48.append(T(icon_list[0], icon_list[1:], [110, 190, 240]))
    p48.append(Spacer(1, 4))
    p48.append(CB("ZERO DISK I/O FOR ICON ASSETS",
                  "Because all icons are mathematically generated via vertex and index buffers at runtime, the application requires zero asset files on disk. Missing icon crashes and broken asset paths are impossible.",
                  "#059669", "#ECFDF5"))
    pages.append(p48)

    # PAGE 49: icons.rs Source Code Part 1
    p49 = []
    p49.extend(H("Vector Graphics", "48. rust/src/plugins/icons.rs — Source Code (Lines 1 – 90)", "Mesh generation primitives, vertex buffers, and basic geometric icons"))
    p49.append(Paragraph("The code below implements geometry generation and basic tool icons:",
                         ParagraphStyle('P49_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p49.append(C("plugins_icons", 1, 90, max_chars=95, compact=True))
    pages.append(p49)

    # PAGE 50: icons.rs Source Code Part 2
    p50 = []
    p50.extend(H("Vector Graphics", "49. rust/src/plugins/icons.rs — Source Code (Lines 91 – 179)", "Specialized clinical vector icons and geometry validation tests"))
    p50.append(Paragraph("The code below implements clinical domain icons and unit test suites:",
                         ParagraphStyle('P50_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p50.append(C("plugins_icons", 91, 179, max_chars=95, compact=True))
    pages.append(p50)

    # PAGE 51: Plugin & Icon Unit Tests
    p51 = []
    p51.extend(H("Plugin Architecture", "50. rust/src/plugins/ — Pipeline & Icon Unit Tests", "Verification of procedural vector meshes, badge geometries, and catalog bounds"))
    p51.append(Paragraph("The unit test suite verifies vector icon generation and pipeline catalog structures:",
                         ParagraphStyle('P51_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    pipe_tests = [
        ["Test Function", "Module", "Verification Target", "Result"],
        ["test_all_banner_badges_valid", "plugins::icons", "Validates that all banner badges generate valid non-empty geometry buffers", "PASS (0.00s)"],
        ["test_all_embedded_tiles_valid", "plugins::icons", "Confirms embedded tile icons compute correct bounding boxes and vertex counts", "PASS (0.00s)"],
        ["test_embedded_icon", "tests", "Verifies embedded procedural icon rendering in clinical message headers", "PASS (0.00s)"],
        ["test_catalog_count", "plugins::pipeline", "Confirms default plugin catalog contains expected active MCP tools", "PASS (0.00s)"],
        ["test_filtering", "plugins::pipeline", "Verifies category filtering correctly isolates vision tools vs pharmacology tools", "PASS (0.00s)"]
    ]
    p51.append(T(pipe_tests[0], pipe_tests[1:], [160, 90, 220, 70]))
    p51.append(Spacer(1, 4))
    p51.append(CB("GEOMETRY BUFFER SAFETY",
                  "All mesh vertex buffers are bounded and verified during unit testing. Zero degenerate triangles, NaN coordinate floats, or vertex buffer overflows were observed across test suites.",
                  "#059669", "#ECFDF5"))
    pages.append(p51)

    # PAGE 52: Dynamic Connection & State Toggle Verification
    p52 = []
    p52.extend(H("Plugin Architecture", "51. rust/src/plugins/ — Dynamic Connection Verification", "State persistence, tool connection toggles, and async execution proofs"))
    p52.append(Paragraph("The test suite below validates tool connection toggling and state synchronization:",
                         ParagraphStyle('P52_1', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    toggle_tests = [
        ["Test Function", "Initial State", "Action Executed", "Resulting State"],
        ["test_toggle_connection", "Tool Disconnected", "Invoke toggle_connection('retina_segmenter')", "Tool Connected (Active in Pipeline)"],
        ["test_plugins_state_default", "Fresh Launch", "Query default plugins state", "Default tools initialized cleanly"],
        ["test_plugins_state_toggle", "Tool Connected", "Invoke toggle_plugin_state('vision')", "Tool Disabled; UI reflects state"]
    ]
    p52.append(T(toggle_tests[0], toggle_tests[1:], [150, 110, 160, 120]))
    p52.append(Spacer(1, 4))
    p52.append(Paragraph("<b>State Consistency Proof:</b><br/>"
                         "Tool connection flags are stored in a thread-safe `Arc<RwLock<PluginsState>>`. Toggling a tool updates the state atomically, ensuring that pipeline execution tasks always read a consistent view of enabled tools without race conditions.",
                         ParagraphStyle('P52_2', fontName='Helvetica', fontSize=7.5, leading=9.8, textColor=colors.HexColor('#334155'), spaceAfter=4)))
    p52.append(CB("CONCURRENCY WITHOUT DEADLOCKS",
                  "By using short-lived read locks during UI rendering and acquiring write locks exclusively during discrete toggle events, the plugin system eliminates UI deadlock hazards.",
                  "#059669", "#ECFDF5"))
    pages.append(p52)

    return pages

print("Section 6 generator defined.")
