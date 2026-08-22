use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::git::RepositoryUpdate;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GroupKey {
    Repository(PathBuf),
    SourceContainer(String),
}

#[derive(Debug, Clone)]
pub struct SourceGroupItem {
    pub item: usize,
    pub skill_name: String,
    pub skill_path: PathBuf,
    pub repo_name: Option<String>,
    pub repo_path: Option<PathBuf>,
    pub relative_path: Option<PathBuf>,
    pub allow_repository_update: bool,
}

#[derive(Debug, Clone)]
pub struct SourceTableItem {
    pub item: usize,
    pub skill_name: String,
    pub display_path: String,
    skill_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SourceGroup {
    pub key: GroupKey,
    pub name: String,
    pub context: String,
    pub items: Vec<SourceTableItem>,
    pub repository_update: Option<RepositoryUpdate>,
    allow_repository_update: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceTableRow {
    Group {
        key: GroupKey,
        name: String,
        context: String,
        count: usize,
        expanded: bool,
        repository_update: Option<RepositoryUpdate>,
    },
    Item {
        group_key: GroupKey,
        item: usize,
        skill_name: String,
        display_path: String,
        source_path: PathBuf,
        checked: bool,
    },
}

impl SourceTableRow {
    pub fn rendered_height(&self) -> usize {
        match self {
            Self::Group {
                repository_update: Some(_),
                ..
            } => 3,
            _ => 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceTable {
    groups: Vec<SourceGroup>,
    expanded: HashSet<GroupKey>,
    checked: HashSet<ItemKey>,
    selected: Option<usize>,
    viewport_offset: usize,
}

impl Default for SourceTable {
    fn default() -> Self {
        Self {
            groups: Vec::new(),
            expanded: HashSet::new(),
            checked: HashSet::new(),
            selected: None,
            viewport_offset: 0,
        }
    }
}

impl SourceTable {
    pub fn new(items: Vec<SourceGroupItem>) -> Self {
        let groups = build_groups(items);
        Self {
            selected: (!groups.is_empty()).then_some(0),
            groups,
            expanded: HashSet::new(),
            checked: HashSet::new(),
            viewport_offset: 0,
        }
    }

    pub fn groups(&self) -> &[SourceGroup] {
        &self.groups
    }

    pub fn set_repository_updates(&mut self, updates: &[RepositoryUpdate]) {
        for group in &mut self.groups {
            group.repository_update = if group.allow_repository_update {
                match &group.key {
                    GroupKey::Repository(path) => updates
                        .iter()
                        .find(|update| normalized_path(&update.repo_path) == *path)
                        .cloned(),
                    GroupKey::SourceContainer(_) => None,
                }
            } else {
                None
            };
        }
    }

    pub fn selected_repository_update(&self) -> Option<RepositoryUpdate> {
        match self.selected_row()? {
            SourceTableRow::Group {
                repository_update, ..
            } => repository_update,
            SourceTableRow::Item { group_key, .. } => self
                .groups
                .iter()
                .find(|group| group.key == group_key)
                .and_then(|group| group.repository_update.clone()),
        }
    }

    pub fn expanded_keys(&self) -> &HashSet<GroupKey> {
        &self.expanded
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn viewport_offset(&self) -> usize {
        self.viewport_offset
    }

    pub fn viewport_indices(&self, viewport_height: usize) -> (usize, usize) {
        let rows = self.visible_rows();
        if rows.is_empty() {
            return (0, 0);
        }

        let viewport_height = viewport_height.max(1);
        let selected = self.selected.unwrap_or(0).min(rows.len() - 1);
        let max_offset = max_viewport_offset(&rows, viewport_height);
        let mut start = self.viewport_offset.min(selected).min(max_offset);
        while start < selected && rendered_height(&rows[start..=selected]) > viewport_height {
            start += 1;
        }

        let mut used = 0;
        let mut end = start;
        while end < rows.len() {
            let height = rows[end].rendered_height();
            if end > start && used + height > viewport_height {
                break;
            }
            used += height;
            end += 1;
        }
        (start, end)
    }

    pub fn visible_rows(&self) -> Vec<SourceTableRow> {
        let mut rows = Vec::new();
        for group in &self.groups {
            let expanded = self.expanded.contains(&group.key);
            rows.push(SourceTableRow::Group {
                key: group.key.clone(),
                name: group.name.clone(),
                context: group.context.clone(),
                count: group.items.len(),
                expanded,
                repository_update: group.repository_update.clone(),
            });
            if expanded {
                rows.extend(group.items.iter().map(|item| {
                    SourceTableRow::Item {
                        group_key: group.key.clone(),
                        item: item.item,
                        skill_name: item.skill_name.clone(),
                        display_path: item.display_path.clone(),
                        source_path: item.skill_path.clone(),
                        checked: self
                            .checked
                            .contains(&ItemKey::new(&group.key, &item.skill_path)),
                    }
                }));
            }
        }
        rows
    }

    pub fn selected_row(&self) -> Option<SourceTableRow> {
        self.selected
            .and_then(|selected| self.visible_rows().get(selected).cloned())
    }

    pub fn checked_items(&self) -> Vec<usize> {
        self.groups
            .iter()
            .flat_map(|group| {
                group.items.iter().filter_map(|item| {
                    self.checked
                        .contains(&ItemKey::new(&group.key, &item.skill_path))
                        .then_some(item.item)
                })
            })
            .collect()
    }

    pub fn toggle_selected_check(&mut self) {
        let Some(SourceTableRow::Item {
            group_key,
            source_path,
            ..
        }) = self.selected_row()
        else {
            return;
        };
        let key = ItemKey::new(&group_key, &source_path);
        if !self.checked.insert(key.clone()) {
            self.checked.remove(&key);
        }
    }

    pub fn move_up(&mut self, viewport_height: usize) {
        self.selected = self.selected.map(|selected| selected.saturating_sub(1));
        self.sync(viewport_height);
    }

    pub fn move_down(&mut self, viewport_height: usize) {
        let row_count = self.visible_rows().len();
        if row_count == 0 {
            self.sync(viewport_height);
            return;
        }
        self.selected = Some(
            self.selected
                .unwrap_or(0)
                .saturating_add(1)
                .min(row_count - 1),
        );
        self.sync(viewport_height);
    }

    pub fn move_right(&mut self, viewport_height: usize) {
        match self.selected_row() {
            Some(SourceTableRow::Group {
                key,
                expanded: false,
                ..
            }) => {
                self.expanded.insert(key);
            }
            Some(SourceTableRow::Group { expanded: true, .. }) => {
                let selected = self.selected.unwrap_or(0);
                if matches!(
                    self.visible_rows().get(selected + 1),
                    Some(SourceTableRow::Item { .. })
                ) {
                    self.selected = Some(selected + 1);
                }
            }
            _ => {}
        }
        self.sync(viewport_height);
    }

    pub fn move_left(&mut self, viewport_height: usize) {
        match self.selected_row() {
            Some(SourceTableRow::Item { group_key, .. }) => {
                self.selected = self.visible_rows().iter().position(
                    |row| matches!(row, SourceTableRow::Group { key, .. } if key == &group_key),
                );
            }
            Some(SourceTableRow::Group {
                key,
                expanded: true,
                ..
            }) => {
                self.expanded.remove(&key);
            }
            _ => {}
        }
        self.sync(viewport_height);
    }

    pub fn sync(&mut self, viewport_height: usize) {
        let rows = self.visible_rows();
        if rows.is_empty() {
            self.selected = None;
            self.viewport_offset = 0;
            return;
        }

        let viewport_height = viewport_height.max(1);
        let selected = self.selected.unwrap_or(0).min(rows.len() - 1);
        let max_offset = max_viewport_offset(&rows, viewport_height);
        let mut offset = self.viewport_offset.min(selected).min(max_offset);
        while offset < selected && rendered_height(&rows[offset..=selected]) > viewport_height {
            offset += 1;
        }

        self.selected = Some(selected);
        self.viewport_offset = offset;
    }

    pub fn refresh(&mut self, items: Vec<SourceGroupItem>, viewport_height: usize) {
        let selection = self.selection_key();
        let groups = build_groups(items);
        let surviving_keys = groups
            .iter()
            .map(|group| group.key.clone())
            .collect::<HashSet<_>>();
        let surviving_items = item_keys(&groups);
        self.expanded.retain(|key| surviving_keys.contains(key));
        self.checked.retain(|key| surviving_items.contains(key));
        self.groups = groups;

        let rows = self.visible_rows();
        self.selected = selection
            .as_ref()
            .and_then(|selection| find_selection(&rows, selection))
            .or_else(|| (!rows.is_empty()).then_some(0));
        self.sync(viewport_height);
    }

    fn selection_key(&self) -> Option<SelectionKey> {
        match self.selected_row()? {
            SourceTableRow::Group { key, .. } => Some(SelectionKey::Group(key)),
            SourceTableRow::Item {
                group_key,
                source_path,
                ..
            } => Some(SelectionKey::Item(group_key, source_path)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ItemKey {
    group_key: GroupKey,
    source_path: PathBuf,
}

impl ItemKey {
    fn new(group_key: &GroupKey, source_path: &Path) -> Self {
        Self {
            group_key: group_key.clone(),
            source_path: source_path.to_path_buf(),
        }
    }
}

#[derive(Debug)]
enum SelectionKey {
    Group(GroupKey),
    Item(GroupKey, PathBuf),
}

fn item_keys(groups: &[SourceGroup]) -> HashSet<ItemKey> {
    groups
        .iter()
        .flat_map(|group| {
            group
                .items
                .iter()
                .map(|item| ItemKey::new(&group.key, &item.skill_path))
        })
        .collect()
}

fn rendered_height(rows: &[SourceTableRow]) -> usize {
    rows.iter().map(SourceTableRow::rendered_height).sum()
}

fn max_viewport_offset(rows: &[SourceTableRow], viewport_height: usize) -> usize {
    let mut offset = 0;
    while offset + 1 < rows.len() && rendered_height(&rows[offset..]) > viewport_height {
        offset += 1;
    }
    offset
}

fn find_selection(rows: &[SourceTableRow], selection: &SelectionKey) -> Option<usize> {
    match selection {
        SelectionKey::Group(selected_key) => rows.iter().position(
            |row| matches!(row, SourceTableRow::Group { key, .. } if key == selected_key),
        ),
        SelectionKey::Item(selected_key, selected_path) => rows
            .iter()
            .position(|row| {
                matches!(
                    row,
                    SourceTableRow::Item {
                        group_key,
                        source_path,
                        ..
                    } if group_key == selected_key && source_path == selected_path
                )
            })
            .or_else(|| {
                rows.iter().position(
                    |row| matches!(row, SourceTableRow::Group { key, .. } if key == selected_key),
                )
            }),
    }
}

fn build_groups(items: Vec<SourceGroupItem>) -> Vec<SourceGroup> {
    let mut groups = Vec::<SourceGroup>::new();
    let mut indices = HashMap::<GroupKey, usize>::new();

    for item in items {
        let (key, name, context, display_path) = item_labels(&item);
        let group_index = *indices.entry(key.clone()).or_insert_with(|| {
            groups.push(SourceGroup {
                key: key.clone(),
                name,
                context,
                items: Vec::new(),
                repository_update: None,
                allow_repository_update: false,
            });
            groups.len() - 1
        });
        groups[group_index].allow_repository_update |= item.allow_repository_update;
        groups[group_index].items.push(SourceTableItem {
            item: item.item,
            skill_name: item.skill_name,
            display_path,
            skill_path: item.skill_path,
        });
    }

    for group in &mut groups {
        group.items.sort_by(|left, right| {
            left.skill_name
                .cmp(&right.skill_name)
                .then_with(|| left.display_path.cmp(&right.display_path))
        });
    }
    disambiguate_group_contexts(&mut groups);
    groups.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.context.cmp(&right.context))
            .then_with(|| left.key.cmp(&right.key))
    });
    groups
}

fn item_labels(item: &SourceGroupItem) -> (GroupKey, String, String, String) {
    if let Some(repo_path) = &item.repo_path {
        let context = bounded_path_suffix(repo_path, 2);
        let display_path = item
            .relative_path
            .as_ref()
            .map(|path| display_relative_path(path))
            .unwrap_or_else(|| {
                item.skill_path
                    .strip_prefix(repo_path)
                    .ok()
                    .map(display_relative_path)
                    .unwrap_or_else(|| bounded_path_suffix(&item.skill_path, 3))
            });
        return (
            GroupKey::Repository(normalized_path(repo_path)),
            item.repo_name
                .clone()
                .unwrap_or_else(|| path_name(repo_path).unwrap_or_else(|| "unknown".to_string())),
            context,
            display_path,
        );
    }

    let source_container = item
        .skill_path
        .parent()
        .map(|path| bounded_path_suffix(path, 2))
        .filter(|context| !context.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let source_key = item
        .skill_path
        .parent()
        .map(privacy_safe_path)
        .filter(|context| !context.is_empty())
        .unwrap_or_else(|| source_container.clone());
    (
        GroupKey::SourceContainer(source_key),
        item.repo_name
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        source_container,
        bounded_path_suffix(&item.skill_path, 3),
    )
}

fn disambiguate_group_contexts(groups: &mut [SourceGroup]) {
    let mut counts = HashMap::new();
    for group in groups.iter() {
        *counts
            .entry((group.name.clone(), group.context.clone()))
            .or_insert(0usize) += 1;
    }

    for group in groups {
        if counts
            .get(&(group.name.clone(), group.context.clone()))
            .copied()
            .unwrap_or_default()
            <= 1
        {
            continue;
        }
        group.context = match &group.key {
            GroupKey::Repository(path) => privacy_safe_path(path),
            GroupKey::SourceContainer(path) => path.clone(),
        };
    }
}

pub fn bounded_path_suffix(path: &Path, max_components: usize) -> String {
    let components = privacy_safe_components(path);
    let start = components.len().saturating_sub(max_components);
    components[start..].join("/")
}

fn privacy_safe_components(path: &Path) -> Vec<String> {
    let mut components = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().into_owned()),
            Component::RootDir | Component::CurDir | Component::ParentDir => None,
        })
        .collect::<Vec<_>>();

    let home_prefix_len = components
        .iter()
        .position(|component| component.eq_ignore_ascii_case("users") || component == "home")
        .filter(|position| components.len() > position + 1)
        .map(|position| position + 2)
        .or_else(|| {
            components
                .first()
                .is_some_and(|component| component == "root")
                .then_some(1)
        })
        .unwrap_or(0);
    components.drain(..home_prefix_len);
    components
}

fn privacy_safe_path(path: &Path) -> String {
    privacy_safe_components(path).join("/")
}

fn normalized_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn display_relative_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            Component::ParentDir => Some("..".to_string()),
            Component::CurDir | Component::RootDir | Component::Prefix(_) => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn path_name(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;

    use crate::git::{RepositoryCommit, RepositoryUpdate};

    use super::{GroupKey, SourceGroupItem, SourceTable, SourceTableRow, bounded_path_suffix};

    fn item(
        id: usize,
        skill_name: &str,
        skill_path: &str,
        repo_name: Option<&str>,
        repo_path: Option<&str>,
        relative_path: Option<&str>,
    ) -> SourceGroupItem {
        SourceGroupItem {
            item: id,
            skill_name: skill_name.to_string(),
            skill_path: PathBuf::from(skill_path),
            repo_name: repo_name.map(str::to_string),
            repo_path: repo_path.map(PathBuf::from),
            relative_path: relative_path.map(PathBuf::from),
            allow_repository_update: true,
        }
    }

    #[test]
    fn groups_git_items_by_repository_root_and_sorts_groups_stably() {
        let table = SourceTable::new(vec![
            item(
                1,
                "beta",
                "/Users/alice/pgit/z-repo/skills/beta",
                Some("z-repo"),
                Some("/Users/alice/pgit/z-repo"),
                Some("skills/beta"),
            ),
            item(
                2,
                "alpha",
                "/Users/alice/pgit/a-repo/skills/alpha",
                Some("a-repo"),
                Some("/Users/alice/pgit/a-repo"),
                Some("skills/alpha"),
            ),
            item(
                3,
                "gamma",
                "/Users/alice/pgit/z-repo/skills/gamma",
                Some("z-repo"),
                Some("/Users/alice/pgit/z-repo"),
                Some("skills/gamma"),
            ),
        ]);

        assert_eq!(table.groups().len(), 2);
        assert_eq!(table.groups()[0].name, "a-repo");
        assert_eq!(table.groups()[1].name, "z-repo");
        assert_eq!(table.groups()[1].items.len(), 2);
        assert_eq!(
            table.groups()[1].key,
            GroupKey::Repository(PathBuf::from("/Users/alice/pgit/z-repo"))
        );
    }

    #[test]
    fn keeps_same_name_repositories_as_distinct_groups_with_safe_suffixes() {
        let table = SourceTable::new(vec![
            item(
                1,
                "one",
                "/Users/alice/pgit/one/skills/one",
                Some("skills"),
                Some("/Users/alice/pgit/one/skills"),
                Some("one"),
            ),
            item(
                2,
                "two",
                "/Users/alice/external/two/skills/two",
                Some("skills"),
                Some("/Users/alice/external/two/skills"),
                Some("two"),
            ),
        ]);

        assert_eq!(table.groups().len(), 2);
        assert_eq!(table.groups()[0].name, "skills");
        assert_ne!(table.groups()[0].key, table.groups()[1].key);
        assert!(
            table
                .groups()
                .iter()
                .all(|group| !group.context.contains("/Users/alice"))
        );
        assert_ne!(table.groups()[0].context, table.groups()[1].context);
    }

    #[test]
    fn expands_colliding_repository_suffixes_without_home_prefixes() {
        let table = SourceTable::new(vec![
            item(
                1,
                "one",
                "/Users/alice/one/team/skills/one",
                Some("skills"),
                Some("/Users/alice/one/team/skills"),
                Some("one"),
            ),
            item(
                2,
                "two",
                "/Users/alice/two/team/skills/two",
                Some("skills"),
                Some("/Users/alice/two/team/skills"),
                Some("two"),
            ),
        ]);

        assert_eq!(table.groups().len(), 2);
        assert_ne!(table.groups()[0].context, table.groups()[1].context);
        assert!(
            table
                .groups()
                .iter()
                .all(|group| !group.context.contains("alice"))
        );
    }

    #[test]
    fn unresolved_items_group_by_safe_source_container() {
        let table = SourceTable::new(vec![
            item(1, "pdf", "/Users/alice/.codex/skills/pdf", None, None, None),
            item(
                2,
                "docs",
                "/Users/alice/.codex/skills/docs",
                None,
                None,
                None,
            ),
            item(3, "pdf", "/home/bob/.agents/skills/pdf", None, None, None),
        ]);

        assert_eq!(table.groups().len(), 2);
        assert_eq!(
            table.groups()[0].items.len() + table.groups()[1].items.len(),
            3
        );
        assert!(
            table
                .groups()
                .iter()
                .all(|group| !group.context.contains("alice") && !group.context.contains("bob"))
        );
    }

    #[test]
    fn toggles_checked_item_rows_and_ignores_group_rows() {
        let mut table = SourceTable::new(vec![
            item(1, "one", "/Users/alice/.codex/skills/one", None, None, None),
            item(2, "two", "/Users/alice/.codex/skills/two", None, None, None),
        ]);

        table.toggle_selected_check();
        assert!(table.checked_items().is_empty());

        table.move_right(10);
        table.move_right(10);
        table.toggle_selected_check();

        assert_eq!(table.checked_items(), vec![1]);
        assert!(matches!(
            table.visible_rows().get(1),
            Some(SourceTableRow::Item { checked: true, .. })
        ));

        table.move_down(10);
        table.toggle_selected_check();
        assert_eq!(table.checked_items(), vec![1, 2]);

        table.toggle_selected_check();
        assert_eq!(table.checked_items(), vec![1]);
    }

    #[test]
    fn same_suffix_unresolved_containers_remain_distinct() {
        let table = SourceTable::new(vec![
            item(
                1,
                "pdf",
                "/Users/alice/one/.codex/skills/pdf",
                None,
                None,
                None,
            ),
            item(
                2,
                "pdf",
                "/Users/alice/two/.codex/skills/pdf",
                None,
                None,
                None,
            ),
        ]);

        assert_eq!(table.groups().len(), 2);
        assert_ne!(table.groups()[0].key, table.groups()[1].key);
        assert_ne!(table.groups()[0].context, table.groups()[1].context);
        assert!(
            table
                .groups()
                .iter()
                .all(|group| !group.context.contains("alice"))
        );
    }

    #[test]
    fn formats_repository_relative_and_bounded_fallback_paths() {
        assert_eq!(
            bounded_path_suffix(&PathBuf::from("/Users/alice/.codex/skills/pdf"), 3),
            ".codex/skills/pdf"
        );
        assert_eq!(
            bounded_path_suffix(&PathBuf::from("/home/bob/skills/pdf"), 3),
            "skills/pdf"
        );
        assert_eq!(
            bounded_path_suffix(&PathBuf::from("/private/Users/alice/pgit/skills/pdf"), 3),
            "pgit/skills/pdf"
        );
        assert_eq!(bounded_path_suffix(&PathBuf::from("/pdf"), 3), "pdf");
    }

    #[test]
    fn starts_collapsed_and_right_expands_then_selects_first_child() {
        let mut table = SourceTable::new(vec![
            item(
                1,
                "alpha",
                "/repos/a/skills/alpha",
                Some("a"),
                Some("/repos/a"),
                Some("skills/alpha"),
            ),
            item(
                2,
                "beta",
                "/repos/a/skills/beta",
                Some("a"),
                Some("/repos/a"),
                Some("skills/beta"),
            ),
        ]);

        assert_eq!(table.visible_rows().len(), 1);
        assert!(matches!(
            table.selected_row(),
            Some(SourceTableRow::Group { .. })
        ));

        table.move_right(4);

        assert_eq!(table.visible_rows().len(), 3);
        assert!(matches!(
            table.selected_row(),
            Some(SourceTableRow::Group { expanded: true, .. })
        ));

        table.move_right(4);

        assert!(matches!(
            table.selected_row(),
            Some(SourceTableRow::Item { item: 1, .. })
        ));
    }

    #[test]
    fn selected_repository_update_is_available_on_group_and_child_rows() {
        let repo_path = PathBuf::from("/repos/skills");
        let update = RepositoryUpdate {
            repo_path: repo_path.clone(),
            commits: vec![RepositoryCommit {
                id: "abc1234".to_string(),
                subject: "Add a skill".to_string(),
            }],
        };
        let mut table = SourceTable::new(vec![item(
            0,
            "review",
            "/repos/skills/review",
            Some("skills"),
            Some("/repos/skills"),
            Some("review"),
        )]);
        table.set_repository_updates(&[update.clone()]);

        assert_eq!(table.selected_repository_update(), Some(update.clone()));
        table.move_right(4);
        table.move_right(4);
        assert_eq!(table.selected_repository_update(), Some(update));
    }

    #[test]
    fn project_local_group_does_not_receive_repository_update_actions() {
        let mut project_local = item(
            0,
            "review",
            "/repos/project/.agents/skills/review",
            Some("project"),
            Some("/repos/project"),
            Some(".agents/skills/review"),
        );
        project_local.allow_repository_update = false;
        let mut table = SourceTable::new(vec![project_local]);
        table.set_repository_updates(&[RepositoryUpdate {
            repo_path: PathBuf::from("/repos/project"),
            commits: vec![RepositoryCommit {
                id: "abc1234".to_string(),
                subject: "Project change".to_string(),
            }],
        }]);

        assert_eq!(table.selected_repository_update(), None);
    }

    #[test]
    fn multi_line_update_rows_keep_the_selected_group_in_the_physical_viewport() {
        let mut table = SourceTable::new(vec![
            item(
                0,
                "first",
                "/repos/first/skills/first",
                Some("first"),
                Some("/repos/first"),
                Some("skills/first"),
            ),
            item(
                1,
                "second",
                "/repos/second/skills/second",
                Some("second"),
                Some("/repos/second"),
                Some("skills/second"),
            ),
        ]);
        table.set_repository_updates(&[
            RepositoryUpdate {
                repo_path: PathBuf::from("/repos/first"),
                commits: vec![RepositoryCommit {
                    id: "first123".to_string(),
                    subject: "First update".to_string(),
                }],
            },
            RepositoryUpdate {
                repo_path: PathBuf::from("/repos/second"),
                commits: vec![RepositoryCommit {
                    id: "second12".to_string(),
                    subject: "Second update".to_string(),
                }],
            },
        ]);

        table.move_down(3);

        assert_eq!(table.selected_index(), Some(1));
        assert_eq!(table.viewport_offset(), 1);
    }

    #[test]
    fn left_on_child_selects_parent_then_collapses_group() {
        let mut table = SourceTable::new(vec![item(
            1,
            "alpha",
            "/repos/a/skills/alpha",
            Some("a"),
            Some("/repos/a"),
            Some("skills/alpha"),
        )]);
        table.move_right(4);
        table.move_right(4);

        table.move_left(4);

        assert!(matches!(
            table.selected_row(),
            Some(SourceTableRow::Group { expanded: true, .. })
        ));

        table.move_left(4);

        assert_eq!(table.visible_rows().len(), 1);
        assert!(matches!(
            table.selected_row(),
            Some(SourceTableRow::Group {
                expanded: false,
                ..
            })
        ));
    }

    #[test]
    fn refresh_preserves_surviving_expansion_and_resets_invalid_selection() {
        let mut table = SourceTable::new(vec![
            item(
                1,
                "alpha",
                "/repos/a/skills/alpha",
                Some("a"),
                Some("/repos/a"),
                Some("skills/alpha"),
            ),
            item(
                2,
                "beta",
                "/repos/b/skills/beta",
                Some("b"),
                Some("/repos/b"),
                Some("skills/beta"),
            ),
        ]);
        table.move_right(4);
        let expanded = table.expanded_keys().clone();

        table.refresh(
            vec![
                item(
                    3,
                    "gamma",
                    "/repos/a/skills/gamma",
                    Some("a"),
                    Some("/repos/a"),
                    Some("skills/gamma"),
                ),
                item(
                    4,
                    "delta",
                    "/repos/c/skills/delta",
                    Some("c"),
                    Some("/repos/c"),
                    Some("skills/delta"),
                ),
            ],
            4,
        );

        assert_eq!(table.expanded_keys(), &expanded);
        assert_eq!(table.selected_index(), Some(0));
        assert!(matches!(
            table.selected_row(),
            Some(SourceTableRow::Group { expanded: true, .. })
        ));
    }

    #[test]
    fn expanded_keys_ignore_groups_that_disappear() {
        let mut table = SourceTable::new(vec![item(
            1,
            "alpha",
            "/repos/a/skills/alpha",
            Some("a"),
            Some("/repos/a"),
            Some("skills/alpha"),
        )]);
        table.move_right(4);

        table.refresh(
            vec![item(
                2,
                "beta",
                "/repos/b/skills/beta",
                Some("b"),
                Some("/repos/b"),
                Some("skills/beta"),
            )],
            4,
        );

        assert_eq!(table.expanded_keys(), &HashSet::new());
        assert_eq!(table.visible_rows().len(), 1);
    }

    #[test]
    fn refresh_keeps_surviving_child_selected_and_falls_back_to_parent_when_removed() {
        let original = item(
            1,
            "alpha",
            "/repos/a/skills/alpha",
            Some("a"),
            Some("/repos/a"),
            Some("skills/alpha"),
        );
        let mut table = SourceTable::new(vec![original.clone()]);
        table.move_right(4);
        table.move_right(4);

        table.refresh(vec![original], 4);

        assert!(matches!(
            table.selected_row(),
            Some(SourceTableRow::Item { .. })
        ));

        table.refresh(
            vec![item(
                2,
                "beta",
                "/repos/a/skills/beta",
                Some("a"),
                Some("/repos/a"),
                Some("skills/beta"),
            )],
            4,
        );

        assert!(matches!(
            table.selected_row(),
            Some(SourceTableRow::Group { expanded: true, .. })
        ));
    }

    #[test]
    fn empty_table_has_no_visible_or_selected_rows() {
        let table = SourceTable::new(Vec::new());

        assert!(table.visible_rows().is_empty());
        assert_eq!(table.selected_index(), None);
        assert_eq!(table.viewport_offset(), 0);
    }

    #[test]
    fn resize_clamps_viewport_to_visible_rows() {
        let mut table = SourceTable::new(vec![
            item(1, "a", "/repos/a/a", Some("a"), Some("/repos/a"), Some("a")),
            item(2, "b", "/repos/a/b", Some("a"), Some("/repos/a"), Some("b")),
            item(3, "c", "/repos/a/c", Some("a"), Some("/repos/a"), Some("c")),
            item(4, "d", "/repos/a/d", Some("a"), Some("/repos/a"), Some("d")),
        ]);
        table.move_right(2);
        table.move_right(2);
        table.move_down(2);
        table.move_down(2);
        table.move_down(2);
        assert_eq!(table.viewport_offset(), 3);

        table.sync(4);

        assert_eq!(table.selected_index(), Some(4));
        assert_eq!(table.viewport_offset(), 1);
    }
}
