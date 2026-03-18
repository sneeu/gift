use ratatui::layout::Rect;

/// Centre a popup of the given percentage width and fixed height within `area`.
pub(super) fn popup_area(area: Rect, percent_x: u16, height: u16) -> Rect {
    let width = area.width * percent_x / 100;
    let x = (area.width.saturating_sub(width)) / 2 + area.x;
    let y = (area.height.saturating_sub(height)) / 2 + area.y;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
