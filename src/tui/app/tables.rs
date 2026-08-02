use std::fs;
use std::path::{Path, PathBuf};

use super::App;
use crate::domain::{ConnectionKind, InventoryRow, Scope};
use crate::inventory;
use crate::scanner::ScanResult;
use crate::tui::source_table::{SourceGroupItem, SourceTable, SourceTableRow};
use crate::tui::unified_list::UnifiedListRow;

impl App {
    pub(crate) fn selected_inventory_row(&self) -> Option<InventoryRow> {
        match self.list_table.selected_row()? {
            SourceTableRow::Item { item, .. } => match self.list_rows.get(item)? {
                UnifiedListRow::Exposed(row) => Some(row.clone()),
                UnifiedListRow::Discovered(_) => None,
            },
            SourceTableRow::Group { .. } => None,
        }
    }

    pub(super) fn selected_discovery_row(&self) -> Option<ScanResult> {
        match self.list_table.selected_row()? {
            SourceTableRow::Item { item, .. } => match self.list_rows.get(item)? {
                UnifiedListRow::Exposed(_) => None,
                UnifiedListRow::Discovered(result) => Some(result.clone()),
            },
            SourceTableRow::Group { .. } => None,
        }
    }

    pub(crate) fn checked_inventory_rows(&self) -> Vec<InventoryRow> {
        self.list_table
            .checked_items()
            .into_iter()
            .filter_map(|item| match self.list_rows.get(item)? {
                UnifiedListRow::Exposed(row) => Some(row.clone()),
                UnifiedListRow::Discovered(_) => None,
            })
            .collect()
    }

    pub(crate) fn checked_import_scan_results(&self) -> Vec<ScanResult> {
        self.list_table
            .checked_items()
            .into_iter()
            .filter_map(|item| match self.list_rows.get(item)? {
                UnifiedListRow::Exposed(row) => self.scan_result_for_inventory_row(row).cloned(),
                UnifiedListRow::Discovered(result) => Some(result.clone()),
            })
            .collect()
    }

    pub(super) fn selection_required_message(&self, table: &SourceTable) -> String {
        if matches!(table.selected_row(), Some(SourceTableRow::Group { .. })) {
            "Select a skill inside the group.".to_string()
        } else {
            "No skill row selected.".to_string()
        }
    }

    pub(super) fn unified_list_table_items(&self) -> Vec<SourceGroupItem> {
        self.list_rows
            .iter()
            .enumerate()
            .map(|(index, row)| match row {
                UnifiedListRow::Exposed(row) => inventory_table_item(index, row),
                UnifiedListRow::Discovered(result) => discovery_table_item(index, result),
            })
            .collect()
    }

    pub(super) fn scan_result_for_inventory_row(&self, row: &InventoryRow) -> Option<&ScanResult> {
        let display_id = display_inventory_row(row);
        let local_index = self
            .inventory
            .iter()
            .filter(|candidate| {
                display_inventory_row(candidate) == display_id
                    && candidate.source.repo_path == row.source.repo_path
            })
            .position(|candidate| candidate == row)
            .unwrap_or(0);
        let matches = self
            .scan_results
            .iter()
            .filter(|result| {
                result.skill_id == display_id && result.repo_path == row.source.repo_path
            })
            .collect::<Vec<_>>();
        matches
            .get(local_index)
            .copied()
            .or_else(|| matches.first().copied())
    }
}

fn inventory_table_item(index: usize, row: &InventoryRow) -> SourceGroupItem {
    let skill_path = source_path_for_inventory_row(row);
    let (repo_name, repo_path, relative_path) = match row.scope {
        Scope::Global => {
            let relative_path = row
                .source
                .repo_path
                .as_ref()
                .and_then(|root| skill_path.strip_prefix(root).ok())
                .map(Path::to_path_buf);
            (
                row.source.repo_name.clone(),
                row.source.repo_path.clone(),
                relative_path,
            )
        }
        Scope::ProjectLocal => {
            let project_root = row
                .exposures
                .first()
                .and_then(|exposure| inventory::project_root_from_exposure_path(&exposure.path));
            let repo_name = project_root
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .or_else(|| Some("project-local".to_string()));
            let relative_path = project_root
                .as_ref()
                .and_then(|root| skill_path.strip_prefix(root).ok())
                .map(Path::to_path_buf);
            (repo_name, project_root, relative_path)
        }
    };
    SourceGroupItem {
        item: index,
        skill_name: inventory_skill_label(row),
        skill_path,
        repo_name,
        repo_path,
        relative_path,
    }
}

fn discovery_table_item(index: usize, result: &ScanResult) -> SourceGroupItem {
    SourceGroupItem {
        item: index,
        skill_name: scan_skill_label(result),
        skill_path: result.skill_path.clone(),
        repo_name: result.repo_name.clone(),
        repo_path: result.repo_path.clone(),
        relative_path: result.skill_relative_path.clone(),
    }
}

fn source_path_for_inventory_row(row: &InventoryRow) -> PathBuf {
    let Some(exposure) = row.exposures.first() else {
        return row
            .source
            .repo_path
            .as_ref()
            .map(|path| path.join(&row.skill_id.name))
            .unwrap_or_else(|| PathBuf::from(&row.skill_id.name));
    };
    if exposure.connection == ConnectionKind::Symlink {
        return fs::canonicalize(&exposure.path).unwrap_or_else(|_| exposure.path.clone());
    }
    exposure.path.clone()
}

pub(super) fn display_inventory_row(row: &InventoryRow) -> String {
    row.skill_id.to_string()
}

fn inventory_skill_label(row: &InventoryRow) -> String {
    match row.disambiguation_index {
        Some(index) => format!("({index}) {}", row.skill_id.name),
        None => row.skill_id.name.clone(),
    }
}

fn scan_skill_label(result: &ScanResult) -> String {
    let name = result
        .skill_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| {
            result
                .skill_id
                .rsplit('/')
                .next()
                .unwrap_or(&result.skill_id)
                .to_string()
        });
    match result.disambiguation_index {
        Some(index) => format!("({index}) {name}"),
        None => name,
    }
}

pub(super) fn parse_selection(input: &str, max: usize) -> Option<usize> {
    let index = input.trim().parse::<usize>().ok()?;
    (1..=max).contains(&index).then_some(index - 1)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::config::Config;
    use crate::scanner::SourceKind;
    use crate::tui::unified_list::UnifiedListRow;

    #[test]
    fn unified_list_table_items_group_discoveries_by_their_source_repository() {
        let mut app = App::new(Config::default_config()).unwrap();
        app.list_rows = vec![UnifiedListRow::Discovered(ScanResult {
            skill_id: "repository/discovered".to_string(),
            skill_path: PathBuf::from("/workspace/repository/discovered"),
            skill_relative_path: Some(PathBuf::from("discovered")),
            repo_name: Some("repository".to_string()),
            repo_path: Some(PathBuf::from("/workspace/repository")),
            remote_url: None,
            source_kind: SourceKind::CentralDir,
            disambiguation_index: None,
        })];

        let items = app.unified_list_table_items();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].skill_name, "discovered");
        assert_eq!(items[0].relative_path, Some(PathBuf::from("discovered")));
        assert_eq!(items[0].repo_name.as_deref(), Some("repository"));
    }
}
