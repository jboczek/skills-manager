use crate::domain::InventoryRow;

pub fn render_inventory(rows: &[InventoryRow]) -> String {
    if rows.is_empty() {
        return "No skills found.".to_string();
    }

    rows.iter()
        .map(|row| format!("{}/{}", row.skill_id.namespace, row.skill_id.name))
        .collect::<Vec<_>>()
        .join("\n")
}
