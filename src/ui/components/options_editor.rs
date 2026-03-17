use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::collections::HashMap;

pub struct OptionsEditorComponent {
    pub is_open: bool,
    entries: Vec<OptionEntry>,
    selected_index: usize,
    editing: bool,
    edit_buffer: String,
}

struct OptionEntry {
    key: String,
    value: serde_json::Value,
}

impl Default for OptionsEditorComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl OptionsEditorComponent {
    pub fn new() -> Self {
        Self {
            is_open: false,
            entries: Vec::new(),
            selected_index: 0,
            editing: false,
            edit_buffer: String::new(),
        }
    }

    pub fn open(&mut self, options: &HashMap<String, serde_json::Value>) {
        self.is_open = true;
        self.selected_index = 0;
        self.editing = false;
        self.edit_buffer.clear();

        let mut entries: Vec<OptionEntry> = options
            .iter()
            .map(|(k, v)| OptionEntry {
                key: k.clone(),
                value: v.clone(),
            })
            .collect();
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        self.entries = entries;
    }

    pub fn close(&mut self) -> Option<HashMap<String, serde_json::Value>> {
        self.is_open = false;
        let entries = std::mem::take(&mut self.entries);
        if entries.is_empty() {
            return None;
        }
        Some(entries.into_iter().map(|e| (e.key, e.value)).collect())
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.editing || self.entries.is_empty() {
            return;
        }
        let new_idx =
            (self.selected_index as i32 + delta).clamp(0, self.entries.len() as i32 - 1) as usize;
        self.selected_index = new_idx;
    }

    pub fn start_editing(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.editing = true;
        self.edit_buffer = match &self.entries[self.selected_index].value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
    }

    pub fn confirm_edit(&mut self) {
        if !self.editing {
            return;
        }
        let buf = self.edit_buffer.trim();

        let new_value = if let Ok(n) = buf.parse::<i64>() {
            serde_json::Value::Number(n.into())
        } else if let Ok(n) = buf.parse::<f64>() {
            serde_json::Number::from_f64(n)
                .map(serde_json::Value::Number)
                .unwrap_or_else(|| serde_json::Value::String(buf.to_string()))
        } else if buf.eq_ignore_ascii_case("true") {
            serde_json::Value::Bool(true)
        } else if buf.eq_ignore_ascii_case("false") {
            serde_json::Value::Bool(false)
        } else {
            serde_json::Value::String(buf.to_string())
        };

        self.entries[self.selected_index].value = new_value;
        self.editing = false;
        self.edit_buffer.clear();
    }

    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.edit_buffer.clear();
    }

    pub fn input_char(&mut self, c: char) {
        if self.editing {
            self.edit_buffer.push(c);
        }
    }

    pub fn backspace(&mut self) {
        if self.editing {
            self.edit_buffer.pop();
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        if !self.is_open {
            return;
        }

        let popup_height = (self.entries.len() as u16 + 8).clamp(8, 20);
        let popup_width = 60;
        let popup_area = Rect {
            x: (area.width.saturating_sub(popup_width)) / 2,
            y: (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };

        f.render_widget(Clear, popup_area);

        let popup_block = Block::default()
            .borders(Borders::ALL)
            .title("Options Editor");
        let inner = popup_block.inner(popup_area);
        f.render_widget(popup_block, popup_area);

        if self.entries.is_empty() {
            f.render_widget(
                Paragraph::new("No options available for this segment.")
                    .style(Style::default().fg(Color::DarkGray)),
                inner,
            );
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(inner);

        let lines: Vec<Line> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let is_selected = i == self.selected_index;
                let value_str = if is_selected && self.editing {
                    format!("{}\u{2581}", self.edit_buffer)
                } else {
                    match &entry.value {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    }
                };

                let key_style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                let value_style = if is_selected && self.editing {
                    Style::default().fg(Color::White)
                } else if is_selected {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Gray)
                };

                let marker = if is_selected { "\u{25b8} " } else { "  " };
                Line::from(vec![
                    Span::styled(marker, key_style),
                    Span::styled(entry.key.clone(), key_style),
                    Span::styled(": ", Style::default().fg(Color::DarkGray)),
                    Span::styled(value_str, value_style),
                ])
            })
            .collect();

        f.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Options")),
            chunks[0],
        );

        let action_text = if self.editing {
            "[Enter] Confirm  [Esc] Cancel"
        } else {
            "[Enter] Edit  [Esc] Close"
        };
        f.render_widget(
            Paragraph::new(action_text).block(Block::default().borders(Borders::ALL)),
            chunks[1],
        );
    }
}
