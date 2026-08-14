// Custom widgets for the TUI

use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Simple text widget
pub struct TextWidget {
    content: String,
    style: Style,
}

impl TextWidget {
    pub fn new(content: String) -> Self {
        Self {
            content,
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn render(&self, area: ratatui::layout::Rect, f: &mut Frame) {
        let paragraph = Paragraph::new(self.content.as_str())
            .style(self.style)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL));
        
        f.render_widget(paragraph, area);
    }
}

impl Default for TextWidget {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// Status bar widget
pub struct StatusBarWidget {
    left_content: String,
    right_content: String,
    style: Style,
}

impl StatusBarWidget {
    pub fn new(left: String, right: String) -> Self {
        Self {
            left_content: left,
            right_content: right,
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn render(&self, area: ratatui::layout::Rect, f: &mut Frame) {
        let line = Line::from(vec![
            Span::styled(&self.left_content, self.style),
            Span::raw(" "),
            Span::styled(&self.right_content, self.style),
        ]);
        
        let paragraph = Paragraph::new(line)
            .style(self.style)
            .block(Block::default().borders(Borders::ALL));
        
        f.render_widget(paragraph, area);
    }
}

impl Default for StatusBarWidget {
    fn default() -> Self {
        Self::new(String::new(), String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_widget() {
        let widget = TextWidget::new("Test".to_string());
        assert_eq!(widget.content, "Test");
    }

    #[test]
    fn test_status_bar_widget() {
        let widget = StatusBarWidget::new("Left".to_string(), "Right".to_string());
        assert_eq!(widget.left_content, "Left");
        assert_eq!(widget.right_content, "Right");
    }
}
