use crate::domain::{ConnectionKind, InventoryRow, Scope};

const AGENT_COLUMNS: [(&str, &str); 3] = [
    ("claude", "CLAUDE"),
    ("codex", "CODEX"),
    ("copilot", "COPILOT"),
];

pub fn render_inventory(rows: &[InventoryRow]) -> String {
    if rows.is_empty() {
        return "No skills found.".to_string();
    }

    let skill_width = rows
        .iter()
        .map(|row| skill_display(row).len())
        .max()
        .unwrap_or(24)
        .max(24);
    let source_width = rows
        .iter()
        .map(|row| source_display(row).len())
        .max()
        .unwrap_or(14)
        .max(14);
    let scope_width = rows
        .iter()
        .map(|row| scope_display(row).len())
        .max()
        .unwrap_or(8)
        .max(8);
    let connection_width = rows
        .iter()
        .map(|row| connection_display(row).len())
        .max()
        .unwrap_or(10)
        .max(10);

    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(format!(
        "{:<skill_width$}  {:<source_width$}  {:<7}  {:<7}  {:<7}  {:<scope_width$}  {:<connection_width$}",
        "SKILL", "SOURCE", "CLAUDE", "CODEX", "COPILOT", "SCOPE", "CONNECTION"
    ));

    for row in rows {
        lines.push(format!(
            "{:<skill_width$}  {:<source_width$}  {:<7}  {:<7}  {:<7}  {:<scope_width$}  {:<connection_width$}",
            skill_display(row),
            source_display(row),
            exposure_mark(row, AGENT_COLUMNS[0].0),
            exposure_mark(row, AGENT_COLUMNS[1].0),
            exposure_mark(row, AGENT_COLUMNS[2].0),
            scope_display(row),
            connection_display(row),
        ));
    }

    lines.join("\n")
}

fn skill_display(row: &InventoryRow) -> String {
    let base = if row.skill_id.namespace.is_empty() {
        row.skill_id.name.clone()
    } else {
        format!("{}/{}", row.skill_id.namespace, row.skill_id.name)
    };

    match row.disambiguation_index {
        Some(index) => format!("({index}) {base}"),
        None => base,
    }
}

fn source_display(row: &InventoryRow) -> String {
    row.source.repo_name.clone().unwrap_or_else(|| "unknown".to_string())
}

fn exposure_mark(row: &InventoryRow, agent_id: &str) -> &'static str {
    if row.exposures.iter().any(|exposure| exposure.agent_id.0 == agent_id) {
        "✓"
    } else {
        "-"
    }
}

fn scope_display(row: &InventoryRow) -> &'static str {
    if row.exposures.is_empty() {
        "unknown"
    } else {
        match row.scope {
            Scope::Global => "global",
            Scope::ProjectLocal => "local",
        }
    }
}

fn connection_display(row: &InventoryRow) -> &'static str {
    match primary_connection(row) {
        Some(ConnectionKind::Symlink) => "symlink",
        Some(ConnectionKind::PhysicalCopy) => "physical",
        Some(ConnectionKind::Missing) => "missing",
        Some(ConnectionKind::Unknown) | None => "unknown",
    }
}

fn primary_connection(row: &InventoryRow) -> Option<ConnectionKind> {
    row.exposures
        .iter()
        .map(|exposure| exposure.connection)
        .max_by_key(|connection| match connection {
            ConnectionKind::Symlink => 4,
            ConnectionKind::PhysicalCopy => 3,
            ConnectionKind::Missing => 2,
            ConnectionKind::Unknown => 1,
        })
}
