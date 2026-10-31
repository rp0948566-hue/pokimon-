//! RetinaCare Plugin Pipeline & Model Context Protocol (MCP) Connectors
//! Manages active plugin registries, capabilities, and data flows.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginFilterTab {
    All,
    Yours,
    Discover,
}

#[derive(Debug, Clone)]
pub struct PluginItem {
    pub id: &'static str,
    pub name: &'static str,
    pub author: &'static str,
    pub description: &'static str,
    pub connected: bool,
    pub capabilities: &'static [&'static str],
}

impl PluginItem {
    pub const fn new(
        id: &'static str,
        name: &'static str,
        author: &'static str,
        description: &'static str,
        capabilities: &'static [&'static str],
    ) -> Self {
        Self {
            id,
            name,
            author,
            description,
            connected: false,
            capabilities,
        }
    }
}

pub fn get_default_catalog() -> Vec<PluginItem> {
    vec![
        PluginItem::new(
            "google-drive",
            "Google Drive",
            "by Google",
            "Search, read, and upload files instantly",
            &["SearchDrive", "ReadFileSpans", "UploadClinicalScan"],
        ),
        PluginItem::new(
            "gmail",
            "Gmail",
            "by Google",
            "Draft replies, summarize threads, & search your inbox",
            &["SearchInbox", "SummarizeThread", "DraftReferralNotice"],
        ),
        PluginItem::new(
            "google-calendar",
            "Google Calendar",
            "by Google",
            "Manage your schedule and coordinate meetings effortlessly",
            &["CheckAvailability", "ScheduleConsultation", "ListEvents"],
        ),
        PluginItem::new(
            "microsoft-365",
            "Microsoft 365",
            "by Anthropic",
            "Access your company's SharePoint, OneDrive, Outlook, and Teams directly in Claude",
            &["SharePointSync", "TeamsNotify", "OneDriveAccess"],
        ),
        PluginItem::new(
            "slack",
            "Slack",
            "by Slack",
            "Send messages, create canvases, and fetch Slack data",
            &["PostClinicalAlert", "QueryChannels", "SendSummary"],
        ),
        PluginItem::new(
            "notion",
            "Notion",
            "by Notion",
            "Connect your Notion workspace to search, update, and power workflows across tools",
            &["ReadConsultationNotes", "AppendScreeningSummary", "SearchPages"],
        ),
        PluginItem::new(
            "canva",
            "Canva",
            "by Canva",
            "Search, create, autofill, and export Canva designs",
            &["GeneratePatientEducation", "ExportDiagrams", "SearchTemplates"],
        ),
        PluginItem::new(
            "atlassian-mcp",
            "Atlassian MCP",
            "by Atlassian",
            "Search, read and update Jira, Confluence, Bitbucket, Loom and other Atlassian apps with your existing...",
            &["JiraTickets", "ConfluenceKnowledge", "LoomRecordings"],
        ),
        PluginItem::new(
            "google-sheets",
            "Google Sheets",
            "by Google",
            "Read, analyze, and update your spreadsheets",
            &["ReadBiomarkerTable", "AppendPatientRow", "FormatAuditRange"],
        ),
        PluginItem::new(
            "figma",
            "Figma",
            "by Figma",
            "Generate diagrams and better code from Figma context",
            &["ReadRetinalDiagrams", "ExportComponents", "InspectStyles"],
        ),
        PluginItem::new(
            "google-docs",
            "Google Docs",
            "by Google",
            "Edit, comment on, and summarize your documents",
            &["ReadClinicalRecord", "AppendDiagnosisSummary", "ExportPDF"],
        ),
        PluginItem::new(
            "supabase",
            "Supabase",
            "by Supabase",
            "Manage databases, authentication, and storage",
            &["QueryPatientRegistry", "StoreScreeningReport", "VerifyAuth"],
        ),
    ]
}

#[derive(Debug, Clone)]
pub struct PluginRegistry {
    pub plugins: Vec<PluginItem>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self {
            plugins: get_default_catalog(),
        }
    }
}

impl PluginRegistry {
    pub fn toggle_connection(&mut self, id: &str) -> bool {
        if let Some(plugin) = self.plugins.iter_mut().find(|p| p.id == id) {
            plugin.connected = !plugin.connected;
            plugin.connected
        } else {
            false
        }
    }

    pub fn connected_names(&self) -> Vec<String> {
        self.plugins
            .iter()
            .filter(|p| p.connected)
            .map(|p| p.name.to_string())
            .collect()
    }

    pub fn filter_plugins(&self, query: &str, tab: PluginFilterTab) -> Vec<&PluginItem> {
        let q = query.trim().to_lowercase();
        self.plugins
            .iter()
            .filter(|p| match tab {
                PluginFilterTab::All => true,
                PluginFilterTab::Yours => p.connected,
                PluginFilterTab::Discover => !p.connected,
            })
            .filter(|p| {
                if q.is_empty() {
                    true
                } else {
                    p.name.to_lowercase().contains(&q)
                        || p.description.to_lowercase().contains(&q)
                        || p.author.to_lowercase().contains(&q)
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_count() {
        let reg = PluginRegistry::default();
        assert_eq!(reg.plugins.len(), 12);
        assert_eq!(reg.plugins[0].name, "Google Drive");
        assert_eq!(reg.plugins[11].name, "Supabase");
    }

    #[test]
    fn test_toggle_connection() {
        let mut reg = PluginRegistry::default();
        assert!(!reg.plugins[0].connected);
        let now_connected = reg.toggle_connection("google-drive");
        assert!(now_connected);
        assert!(reg.plugins[0].connected);
        assert_eq!(reg.connected_names(), vec!["Google Drive"]);

        let now_disconnected = reg.toggle_connection("google-drive");
        assert!(!now_disconnected);
        assert!(reg.connected_names().is_empty());
    }

    #[test]
    fn test_filtering() {
        let mut reg = PluginRegistry::default();
        reg.toggle_connection("slack");

        let all = reg.filter_plugins("", PluginFilterTab::All);
        assert_eq!(all.len(), 12);

        let yours = reg.filter_plugins("", PluginFilterTab::Yours);
        assert_eq!(yours.len(), 1);
        assert_eq!(yours[0].id, "slack");

        let discover = reg.filter_plugins("", PluginFilterTab::Discover);
        assert_eq!(discover.len(), 11);

        let google_search = reg.filter_plugins("google", PluginFilterTab::All);
        assert_eq!(google_search.len(), 5); // Drive, Gmail, Calendar, Sheets, Docs
    }
}
