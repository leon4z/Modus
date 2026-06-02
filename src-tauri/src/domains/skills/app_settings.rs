// Purpose: Skill notes backed by app configuration.

use crate::platform::config as app_config;
use std::collections::HashMap;

// === Skill Notes ===

pub fn get_skill_note(skill_name: String) -> String {
    let config = app_config::load_config();
    config
        .skill_notes
        .get(&skill_name)
        .cloned()
        .unwrap_or_default()
}

pub fn set_skill_note(skill_name: String, note: String) -> Result<(), String> {
    app_config::update_config(|config| {
        if note.is_empty() {
            config.skill_notes.remove(&skill_name);
        } else {
            config.skill_notes.insert(skill_name, note);
        }
        Ok(())
    })
    .map(|_| ())
}

pub fn get_all_skill_notes() -> HashMap<String, String> {
    let config = app_config::load_config();
    config.skill_notes
}
