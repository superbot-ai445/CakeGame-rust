/// CakeGame 数据编辑器
/// 用于编辑游戏数据

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EditorData {
    pub title: String,
    pub version: String,
    pub author: String,
    pub start_scene: String,
    pub scenes: Vec<EditorScene>,
    pub dialogs: Vec<EditorDialog>,
    pub items: Vec<EditorItem>,
    pub characters: Vec<EditorCharacter>,
    pub variables: Vec<EditorVariable>,
    pub flags: Vec<EditorFlag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorScene {
    pub id: String,
    pub name: String,
    pub description: String,
    pub first_dialog: String,
    pub dialogs: Vec<String>,
    pub unlocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorDialog {
    pub id: String,
    pub speaker: String,
    pub text: String,
    pub next: String,
    pub choices: Vec<EditorChoice>,
    pub effects: Vec<EditorEffect>,
    pub conditions: Vec<EditorCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorChoice {
    pub text: String,
    pub target: String,
    pub conditions: Vec<EditorCondition>,
    pub effects: Vec<EditorEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorEffect {
    pub effect_type: String,
    pub target: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorCondition {
    pub condition_type: String,
    pub target: String,
    pub value: String,
    pub op: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorItem {
    pub id: String,
    pub name: String,
    pub item_type: String,
    pub description: String,
    pub max_count: i32,
    pub usable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorCharacter {
    pub id: String,
    pub name: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorVariable {
    pub id: String,
    pub name: String,
    pub default_value: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorFlag {
    pub id: String,
    pub name: String,
    pub description: String,
}

pub struct GameEditor {
    pub data: EditorData,
    pub dirty: bool,
}

impl GameEditor {
    pub fn new() -> Self {
        Self {
            data: EditorData::default(),
            dirty: false,
        }
    }

    pub fn new_project(&mut self, title: &str, author: &str) {
        self.data = EditorData {
            title: title.to_string(),
            author: author.to_string(),
            version: "1.0.0".to_string(),
            start_scene: String::new(),
            ..Default::default()
        };
        self.dirty = true;
    }

    pub fn set_start_scene(&mut self, scene_id: &str) {
        self.data.start_scene = scene_id.to_string();
        self.dirty = true;
    }

    pub fn load(&mut self, json: &str) -> Result<(), String> {
        self.data = serde_json::from_str(json).map_err(|e| e.to_string())?;
        self.dirty = false;
        Ok(())
    }

    pub fn save(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.data).map_err(|e| e.to_string())
    }

    pub fn add_scene(&mut self, id: &str, name: &str, desc: &str) -> Result<(), String> {
        self.data.scenes.push(EditorScene {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            first_dialog: String::new(),
            dialogs: Vec::new(),
            unlocked: true,
        });
        self.dirty = true;
        Ok(())
    }

    pub fn add_dialog(&mut self, id: &str, speaker: &str, text: &str, next: &str) -> Result<(), String> {
        self.data.dialogs.push(EditorDialog {
            id: id.to_string(),
            speaker: speaker.to_string(),
            text: text.to_string(),
            next: next.to_string(),
            choices: Vec::new(),
            effects: Vec::new(),
            conditions: Vec::new(),
        });
        self.dirty = true;
        Ok(())
    }

    pub fn add_choice(&mut self, dialog_id: &str, text: &str, target: &str) -> Result<(), String> {
        let dialog = self.data.dialogs.iter_mut().find(|d| d.id == dialog_id)
            .ok_or("对话不存在")?;
        dialog.choices.push(EditorChoice {
            text: text.to_string(),
            target: target.to_string(),
            conditions: Vec::new(),
            effects: Vec::new(),
        });
        self.dirty = true;
        Ok(())
    }

    pub fn add_item(&mut self, id: &str, name: &str, desc: &str, max_count: i32, usable: bool) -> Result<(), String> {
        self.data.items.push(EditorItem {
            id: id.to_string(),
            name: name.to_string(),
            item_type: "Other".to_string(),
            description: desc.to_string(),
            max_count,
            usable,
        });
        self.dirty = true;
        Ok(())
    }

    pub fn add_character(&mut self, id: &str, name: &str, title: &str, desc: &str) -> Result<(), String> {
        self.data.characters.push(EditorCharacter {
            id: id.to_string(),
            name: name.to_string(),
            title: title.to_string(),
            description: desc.to_string(),
        });
        self.dirty = true;
        Ok(())
    }

    pub fn add_variable(&mut self, id: &str, name: &str, default: &str, desc: &str) -> Result<(), String> {
        self.data.variables.push(EditorVariable {
            id: id.to_string(),
            name: name.to_string(),
            default_value: default.to_string(),
            description: desc.to_string(),
        });
        self.dirty = true;
        Ok(())
    }

    pub fn add_flag(&mut self, id: &str, name: &str, desc: &str) -> Result<(), String> {
        self.data.flags.push(EditorFlag {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
        });
        self.dirty = true;
        Ok(())
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.data.title.is_empty() {
            errors.push("标题为空".to_string());
        }

        for scene in &self.data.scenes {
            if !scene.first_dialog.is_empty() && !self.data.dialogs.iter().any(|d| d.id == scene.first_dialog) {
                errors.push(format!("场景 '{}' 的首对话不存在", scene.id));
            }
        }

        for dialog in &self.data.dialogs {
            if !dialog.next.is_empty() && !self.data.dialogs.iter().any(|d| d.id == dialog.next) {
                errors.push(format!("对话 '{}' 的下一对话不存在", dialog.id));
            }
        }

        errors
    }

    pub fn get_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        stats.insert("scenes".to_string(), self.data.scenes.len());
        stats.insert("dialogs".to_string(), self.data.dialogs.len());
        stats.insert("items".to_string(), self.data.items.len());
        stats.insert("characters".to_string(), self.data.characters.len());
        stats
    }
}
