// Purpose: Command surface for Skill notes.

use crate::domains::skills;

#[tauri::command]
pub fn get_skill_note(skill_name: String) -> String {
    skills::get_skill_note(skill_name)
}

#[tauri::command]
pub fn set_skill_note(skill_name: String, note: String) -> Result<(), String> {
    skills::set_skill_note(skill_name, note)
}

#[tauri::command]
pub fn get_all_skill_notes() -> std::collections::HashMap<String, String> {
    skills::get_all_skill_notes()
}
