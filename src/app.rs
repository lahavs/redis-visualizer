use color_eyre::Result;
use ratatui::{
    Frame,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Direction, Layout, Rect, Margin},
    style::{
        palette::tailwind::{BLUE, GREEN, SLATE},
        Color, Modifier, Style, Stylize,
    },
    symbols,
    text::{Line, Text, Span},
    widgets::{
        Block, Borders, HighlightSpacing, List, ListItem, ListState, Padding, Paragraph,
        StatefulWidget, Widget, Wrap,
        Scrollbar, ScrollbarState, ScrollbarOrientation,
    },
    DefaultTerminal,
};

use crate::redis_client::{RedisValue, RedisClient};

const TODO_HEADER_STYLE: Style = Style::new().fg(SLATE.c100).bg(BLUE.c800);
const NORMAL_ROW_BG: Color = SLATE.c950;
const ALT_ROW_BG_COLOR: Color = SLATE.c900;
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
const TEXT_FG_COLOR: Color = SLATE.c200;
const COMPLETED_TEXT_FG_COLOR: Color = GREEN.c500;
const HIGHLIGHTED_STYLE: Style = Style::new().bg(Color::LightYellow).fg(Color::Black);

#[derive(Eq, PartialEq, Copy, Clone)]
enum Part {
    KeyInput,
    RedisKeys,
    RedisValue
}

#[derive(Eq, PartialEq, Copy, Clone)]
enum Mode {
    SelectingParts(Part),
    FocusedOnPart(Part),
}

pub struct App {
    should_exit: bool,
    redis_client: RedisClient,
    redis_keys_list: RedisKeysList,
    ui_redis_value: Option<UiRedisValue>,
    key_input: String, // The current key
    mode: Mode,
    filtered_keys: Vec<RedisKeysItem>,
    redis_value_part_percent: u16,
    redis_key_horizontal_scroll: usize,
}

#[derive(Default)]
struct RedisKeysList {
    items: Vec<RedisKeysItem>,
    state: ListState,
    // scrollbar_state: ScrollbarState,
}

#[derive(Clone)]
struct RedisKeysItem {
    redis_key: String,
}

impl RedisKeysItem {
    fn new(redis_key: String) -> Self {
        Self {
            redis_key,
        }
    }
}

#[derive(Default)]
struct UiRedisValueList {
    items: Vec<(String, String)>,
    state: ListState,
}

enum UiRedisValue {
    Simple(String),
    List(UiRedisValueList),
}


impl From<&RedisKeysItem> for ListItem<'_> {
    fn from(value: &RedisKeysItem) -> Self {
        let line = Line::styled(value.redis_key.clone(), COMPLETED_TEXT_FG_COLOR);
        ListItem::new(line)
    }
}

impl From<Vec<String>> for RedisKeysList {
    fn from(redis_keys: Vec<String>) -> Self {
        let num_redis_keys = redis_keys.len();
        let items = redis_keys
            .into_iter()
            .map(|redis_key| RedisKeysItem::new(redis_key))
            .collect();
        RedisKeysList {
            items,
            state: ListState::default(),
            // scrollbar_state: ScrollbarState::default().content_length(num_redis_keys),
        }
    }
}

impl App {
    pub fn new(mut redis_client: RedisClient) -> redis::RedisResult<Self> {
        let redis_keys = redis_client.get_redis_keys()?;

        Ok(Self {
            should_exit: false,
            redis_client,
            redis_keys_list: RedisKeysList::from(redis_keys.clone()),
            ui_redis_value: None,
            key_input: String::new(),
            mode: Mode::SelectingParts(Part::KeyInput),
            filtered_keys: RedisKeysList::from(redis_keys).items,
            redis_value_part_percent: 30,
            redis_key_horizontal_scroll: 0,
        })
    }
}

impl App {
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        // self.select_first();

        while !self.should_exit {
            terminal.draw(|frame| self.draw(frame))?;
            if let Event::Key(key) = event::read()? {
                self.handle_key(key);
            };
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match self.mode {
            Mode::SelectingParts(highlighted_part) => match key.code {
                    KeyCode::Char('q') => self.should_exit = true,
                    KeyCode::Enter => self.focus_highlighted(),
                    KeyCode::Char('j') | KeyCode::Down => self.highlight_down(),
                    KeyCode::Char('k') | KeyCode::Up => self.highlight_up(),
                    KeyCode::Char('h') | KeyCode::Left => self.highlight_left(),
                    KeyCode::Char('l') | KeyCode::Right => self.highlight_right(),
                    KeyCode::Char('H') => self.redis_parts_left(),
                    KeyCode::Char('L') => self.redis_parts_right(),
                    _ => {}
                }
            Mode::FocusedOnPart(part) => match part {
                Part::KeyInput => match key.code {
                    KeyCode::Backspace => self.pop_key_input_char(),
                    KeyCode::Char(value) => self.push_key_input_char(value),
                    KeyCode::Enter | KeyCode::Esc => self.highlight_focused(),
                    _ => {}
                }
                Part::RedisKeys => match key.code {
                    KeyCode::Char('q') => self.should_exit = true,
                    KeyCode::Esc => self.highlight_focused(),
                    KeyCode::Char('h') | KeyCode::Left => self.redis_keys_left(),
                    KeyCode::Char('j') | KeyCode::Down => self.redis_keys_select_next(),
                    KeyCode::Char('k') | KeyCode::Up => self.redis_keys_select_previous(),
                    KeyCode::Char('l') | KeyCode::Left => self.redis_keys_right(),
                    KeyCode::Char('g') | KeyCode::Home => self.redis_keys_select_first(),
                    KeyCode::Char('G') | KeyCode::End => self.redis_keys_select_last(),
                    _ => {}
                }
                Part::RedisValue => match key.code {
                    KeyCode::Char('q') => self.should_exit = true,
                    KeyCode::Esc => self.highlight_focused(),
                    KeyCode::Char('j') | KeyCode::Down => self.redis_value_select_next(),
                    KeyCode::Char('k') | KeyCode::Up => self.redis_value_select_previous(),
                    KeyCode::Char('g') | KeyCode::Home => self.redis_value_select_first(),
                    KeyCode::Char('G') | KeyCode::End => self.redis_value_select_last(),
                    _ => {}
                }
            }
        }

        /*
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_exit = true,
            // KeyCode::Char('h') | KeyCode::Left => self.select_none(),
            KeyCode::Char('j') | KeyCode::Down => self.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.select_previous(),
            KeyCode::Char('g') | KeyCode::Home => self.select_first(),
            KeyCode::Char('G') | KeyCode::End => self.select_last(),
            // KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
            //     self.toggle_status();
            // }
            _ => {}
        }
        */
    }

    fn calculate_filtered_keys(&mut self) {
        self.filtered_keys = self
            .redis_keys_list
            .items
            .iter()
            .filter(|&redis_keys_item| redis_keys_item.redis_key.contains(&self.key_input))
            .map(|redis_keys_item| redis_keys_item.clone())
            .collect::<Vec<_>>()

    }

    fn pop_key_input_char(&mut self) {
        self.key_input.pop();
        self.calculate_filtered_keys();
        self.update_redis_value();
    }

    fn push_key_input_char(&mut self, value: char) {
        self.key_input.push(value);
        self.calculate_filtered_keys();
        self.update_redis_value();
    }

    fn redis_parts_left(&mut self) {
        self.redis_value_part_percent = self.redis_value_part_percent.saturating_sub(1).clamp(10, 89);
    }

    fn redis_parts_right(&mut self) {
        self.redis_value_part_percent = self.redis_value_part_percent.saturating_add(1).clamp(10, 89);
    }

    fn focus_highlighted(&mut self) {
        let new_mode = match self.mode {
            Mode::SelectingParts(part) => Mode::FocusedOnPart(part),
            _ => { return; }
        };

        self.mode = new_mode;
    }

    fn highlight_focused(&mut self) {
        let new_mode = match self.mode {
            Mode::FocusedOnPart(part) => Mode::SelectingParts(part),
            _ => { return; }
        };

        self.mode = new_mode;
    }

    fn highlight_down(&mut self) {
        let new_highlighted_part = match self.mode {
            Mode::SelectingParts(highlighted_part) => match highlighted_part {
                Part::KeyInput => Part::RedisKeys,
                _ => highlighted_part
            },
            _ => { return; }
        };

        self.mode = Mode::SelectingParts(new_highlighted_part);
    }

    fn highlight_up(&mut self) {
        let new_highlighted_part = match self.mode {
            Mode::SelectingParts(highlighted_part) => match highlighted_part {
                Part::RedisKeys | Part::RedisValue => Part::KeyInput,
                _ => highlighted_part
            },
            _ => { return; }
        };

        self.mode = Mode::SelectingParts(new_highlighted_part);
    }

    fn highlight_left(&mut self) {
        let new_highlighted_part = match self.mode {
            Mode::SelectingParts(highlighted_part) => match highlighted_part {
                Part::RedisValue => Part::RedisKeys,
                _ => highlighted_part
            },
            _ => { return; }
        };

        self.mode = Mode::SelectingParts(new_highlighted_part);
    }

    fn highlight_right(&mut self) {
        let new_highlighted_part = match self.mode {
            Mode::SelectingParts(highlighted_part) => match highlighted_part {
                Part::RedisKeys => Part::RedisValue,
                _ => highlighted_part
            },
            _ => { return; }
        };

        self.mode = Mode::SelectingParts(new_highlighted_part);
    }

    // fn select_none(&mut self) {
    //     self.redis_keys_list.state.select(None);
    //     self.redis_keys_list.scrollbar_state.first();
    // }

    fn update_redis_value(&mut self) {
        self.ui_redis_value = if let Some(i) = self.redis_keys_list.state.selected() {
            if self.filtered_keys.is_empty() {
                None
            } else {
                let index = i.clamp(0, self.filtered_keys.len() - 1);
                let redis_key = &self.filtered_keys[index];
                let redis_value = self.redis_client.get_redis_value(&redis_key.redis_key).unwrap();
                match redis_value {
                    RedisValue::Unknown => Some(UiRedisValue::Simple("Unknown value".to_string())),
                    RedisValue::Hash(hash) => Some(UiRedisValue::List(UiRedisValueList {
                        items: hash.into_iter().collect(),
                        state: ListState::default().with_selected(Some(0)),
                    })),
                }
            }
        } else {
            None
        };
    }

    fn redis_keys_left(&mut self) {
        self.redis_key_horizontal_scroll = self.redis_key_horizontal_scroll.saturating_sub(1);
    }

    fn redis_keys_right(&mut self) {
        self.redis_key_horizontal_scroll = self.redis_key_horizontal_scroll.saturating_add(1);
    }

    fn redis_keys_select_next(&mut self) {
        self.redis_keys_list.state.select_next();
        self.update_redis_value();
        // self.redis_key_horizontal_scroll = 0;
        // self.redis_keys_list.scrollbar_state.next();
    }

    fn redis_keys_select_previous(&mut self) {
        self.redis_keys_list.state.select_previous();
        self.update_redis_value();
        // self.redis_key_horizontal_scroll = 0;
        // self.redis_keys_list.scrollbar_state.prev();
    }

    fn redis_keys_select_first(&mut self) {
        self.redis_keys_list.state.select_first();
        self.update_redis_value();
        // self.redis_key_horizontal_scroll = 0;
        // self.redis_keys_list.scrollbar_state.first();
    }

    fn redis_keys_select_last(&mut self) {
        self.redis_keys_list.state.select_last();
        self.update_redis_value();
        // self.redis_key_horizontal_scroll = 0;
        // self.redis_keys_list.scrollbar_state.last();
    }

    fn redis_value_select_next(&mut self) {
        match &mut self.ui_redis_value {
            Some(UiRedisValue::List(value)) => value.state.select_next(),
            _ => {}
        }
    }

    fn redis_value_select_previous(&mut self) {
        match &mut self.ui_redis_value {
            Some(UiRedisValue::List(value)) => value.state.select_previous(),
            _ => {}
        }
    }

    fn redis_value_select_first(&mut self) {
        match &mut self.ui_redis_value {
            Some(UiRedisValue::List(value)) => value.state.select_first(),
            _ => {}
        }
    }

    fn redis_value_select_last(&mut self) {
        match &mut self.ui_redis_value {
            Some(UiRedisValue::List(value)) => value.state.select_last(),
            _ => {}
        }
    }
}

impl App {
    fn draw(&mut self, frame: &mut Frame) {
        // let area = Layout::default()
        //     .direction(Direction::Vertical)
        //     .constraints(vec![
        //         Constraint::Percentage(15),
        //     ])
        //     .split(area)[0];

        let [key_area, redis_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .areas(frame.area());

        let [redis_keys_area, redis_values_area] = Layout::horizontal([
            Constraint::Percentage(self.redis_value_part_percent),
            Constraint::Percentage(100 - self.redis_value_part_percent),
        ])
        .areas(redis_area);

        // let [list_area, item_area] =
        //     Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]).areas(main_area);

        self.render_key_area(key_area, frame);
        self.render_redis_keys_list(redis_keys_area, frame);
        self.render_selected_redis_value(redis_values_area, frame);
    }
}

fn render_redis_value(redis_value: RedisValue) -> String {
    match redis_value {
        RedisValue::Unknown => "Unknown value".to_string(),
        RedisValue::Hash(hash) => {
            hash
                .iter()
                .map(|(key, value)| format!("{}: {}", key, value))
                .collect::<Vec<String>>()
                .join("\n")
        }
    }
}

fn horizontal_slice(s: &str, start: usize, width: usize) -> &str {
    let start_byte = s.char_indices()
        .nth(start)
        .map(|(i, _)| i)
        .unwrap_or_else(|| s.len());

    let end_byte = s.char_indices()
        .nth(start + width)
        .map(|(i, _)| i)
        .unwrap_or_else(|| s.len());

    &s[start_byte..end_byte]
}

impl App {
    fn render_key_area(&mut self, area: Rect, frame: &mut Frame) {
        let [label_area, input_area] = Layout::horizontal([
            Constraint::Length(6),
            Constraint::Min(0)
        ])
        .areas(area);

        let label = Paragraph::new("Key:")
            .block(Block::default().padding(Padding::new(1,0,1,1)));
        // Widget::render(label, label_area, buf);
        frame.render_widget(label, label_area);

        let mut input = Paragraph::new(self.key_input.clone())
            .block(Block::default().borders(Borders::ALL));
        if self.mode == Mode::SelectingParts(Part::KeyInput) {
            input = input.style(HIGHLIGHTED_STYLE);
        }
        // Widget::render(input, input_area, buf);
        frame.render_widget(input, input_area);

        if self.mode == Mode::FocusedOnPart(Part::KeyInput) {
            frame.set_cursor_position(ratatui::layout::Position {x:7 + self.key_input.len() as u16, y:1});
        }

        // let highlight_block = Block::new()
        //     .style(HIGHLIGHTED_STYLE);
        // let highlight_input_area = input_area.inner(
        //     Margin {
        //         horizontal: 0,
        //         vertical: 0,
        //     });
        // Widget::render(highlight_block, highlight_input_area, buf);
    }

    fn render_redis_keys_list(&mut self, area: Rect, frame: &mut Frame) {
        let is_highlighted = self.mode == Mode::SelectingParts(Part::RedisKeys);

        // TODO(lahavs): Should it really be here?
        if self.redis_keys_list.state.selected().is_none() {
            self.redis_keys_list.state.select_first();
            self.update_redis_value();
        }

        let num_items = self.filtered_keys.len();

        let matches_text = if num_items == 0 {
            format!("No matches")
        } else {
            let current_item_index = self.redis_keys_list.state.selected().unwrap().clamp(0, num_items-1) + 1;

            let total_num_keys = self.redis_keys_list.items.len();
            if num_items == total_num_keys {
                format!("{current_item_index}/{num_items}")
            } else {
                format!("{current_item_index}/{num_items} (total {total_num_keys})")
            }
        };

        let mut block = Block::new()
            .title(Line::raw("Redis Keys").centered())
            .title_bottom(Line::from(format!(" {matches_text}")).left_aligned())
            .borders(Borders::ALL)
            .border_set(symbols::border::EMPTY)
            .border_style(TODO_HEADER_STYLE)
            .bg(NORMAL_ROW_BG);

        if is_highlighted {
            block = block.style(HIGHLIGHTED_STYLE);
        }

        let area_width = area.width as usize;

        let longest_filtered_redis_key = self.filtered_keys.iter().map(|redis_keys_item| redis_keys_item.redis_key.len()).max().unwrap_or(0);
        // -5: Due to the '>> ' (3 cells) and the border (1 cell at each size)
        let largest_horizontal_scroll = longest_filtered_redis_key.saturating_sub(area_width-5);
        self.redis_key_horizontal_scroll = self.redis_key_horizontal_scroll.min(largest_horizontal_scroll);

        // Iterate through all elements in the `items` and stylize them.
        let items: Vec<ListItem> = self.filtered_keys
            .iter()
            .enumerate()
            .map(|(i, redis_keys_item)| {
                // let start = self.redis_key_horizontal_scroll.min(redis_keys_item.redis_key.len());
                // let end = (start + area_width).min(redis_keys_item.redis_key.len());
                // let text = &redis_keys_item.redis_key[start..end];
                let text = horizontal_slice(&redis_keys_item.redis_key, self.redis_key_horizontal_scroll, area_width);
                let line = Line::styled(text.clone(), COMPLETED_TEXT_FG_COLOR);
                let mut list_item = ListItem::new(line).bg(NORMAL_ROW_BG);
                // let mut list_item = ListItem::from(redis_keys_item).bg(NORMAL_ROW_BG);
                if is_highlighted {
                    list_item = list_item.style(HIGHLIGHTED_STYLE);
                }

                list_item
            })
            // .skip(self.redis_keys_list.state.selected().unwrap())
            // .take(31*2)
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let mut list = List::new(items)
            .block(block)
            .highlight_symbol(">> ")
            .highlight_spacing(HighlightSpacing::Always);

        if ! is_highlighted {
            list = list.highlight_style(SELECTED_STYLE)
        }

        // We need to disambiguate this trait method as both `Widget` and `StatefulWidget` share the
        // same method name `render`.
        // StatefulWidget::render(list, area, buf, &mut self.redis_keys_list.state);
        frame.render_stateful_widget(list, area, &mut self.redis_keys_list.state);

        if let Some(selected) = self.redis_keys_list.state.selected() {
            // Vertical scroll
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalLeft)
                .symbols(symbols::scrollbar::VERTICAL);

            let mut scrollbar_state = ScrollbarState::default()
                .content_length(num_items)
                .position(selected);

            // StatefulWidget::render(scrollbar, area, buf, &mut scrollbar_state);
            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);

            // Horizontal scroll
            // let redis_key = &self.filtered_keys[selected];
            let horizontal_scrollbar = Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                    .symbols(symbols::scrollbar::HORIZONTAL);
            let mut horizontal_scrollbar_state = ScrollbarState::default()
                .content_length(largest_horizontal_scroll)
                .position(self.redis_key_horizontal_scroll);
            frame.render_stateful_widget(horizontal_scrollbar, area, &mut horizontal_scrollbar_state);
        }
        // StatefulWidget::render(scrollbar, area, buf, &mut self.redis_keys_list.scrollbar_state);
    }

    fn render_selected_redis_value(&mut self, area: Rect, frame: &mut Frame) {
        let is_highlighted = self.mode == Mode::SelectingParts(Part::RedisValue);

        // let redis_value = if let Some(i) = self.redis_keys_list.state.selected() {
        //     let redis_key = &self.filtered_keys[i];
        //     let redis_value = self.redis_client.get_redis_value(&redis_key.redis_key).unwrap();
        //     render_redis_value(redis_value)
        // } else {
        //     "No key selected".to_string()
        // };

        // We show the list item's info under the list in this paragraph
        let mut block = Block::new()
            .title(Line::raw("Redis Value").centered())
            .borders(Borders::ALL)
            .border_set(symbols::border::EMPTY)
            .border_style(TODO_HEADER_STYLE)
            .bg(NORMAL_ROW_BG)
            .padding(Padding::horizontal(1));

        if is_highlighted {
            block = block.style(HIGHLIGHTED_STYLE);
        }

        match &mut self.ui_redis_value {
            Some(UiRedisValue::Simple(value)) => {
                // We can now render the item info
                let paragraph = Paragraph::new(value.clone())
                    .block(block)
                    // .fg(TEXT_FG_COLOR)
                    .wrap(Wrap { trim: false });
                frame.render_widget(paragraph, area);
            }
            Some(UiRedisValue::List(value)) => {
                let num_items = value.items.len();

                let matches_text = if num_items == 0 {
                    format!("No fields")
                } else {
                    let current_item_index = value.state.selected().unwrap_or(0).clamp(0, num_items-1) + 1;
                    format!("{current_item_index}/{num_items}")
                };

                block = block.title_bottom(Line::from(format!(" {matches_text}")).left_aligned());
                let items: Vec<ListItem> = value
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, (key, value))| {
                        // TODO(lahavs): I like the unstyled text better for values
                        // Need to distinguish between the key's style..
                        /*
                        let line = Line::styled(format!("{}: {}", key, value), COMPLETED_TEXT_FG_COLOR);
                        let mut list_item = ListItem::new(line).bg(NORMAL_ROW_BG);
                        */
                        let line = Line::from(format!("{}: {}", key, value));

                        let mut list_item = ListItem::new(line).bg(NORMAL_ROW_BG);
                        if is_highlighted {
                            list_item = list_item.style(HIGHLIGHTED_STYLE);
                        }

                        list_item
                    })
                    .collect();

                // Create a List from all list items and highlight the currently selected one
                let mut list = List::new(items)
                    .block(block)
                    .highlight_symbol(">> ")
                    .highlight_spacing(HighlightSpacing::Always);

                if ! is_highlighted {
                    list = list.highlight_style(SELECTED_STYLE)
                }

                // We need to disambiguate this trait method as both `Widget` and `StatefulWidget` share the
                // same method name `render`.
                frame.render_stateful_widget(list, area, &mut value.state);

                if let Some(selected) = value.state.selected() {
                    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalLeft)
                        .symbols(symbols::scrollbar::VERTICAL);

                    let mut scrollbar_state = ScrollbarState::default()
                        .content_length(num_items)
                        .position(selected);

                    // StatefulWidget::render(scrollbar, area, buf, &mut scrollbar_state);
                    frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
                }
            }
            None => {
                let paragraph = Paragraph::new("No key selected".to_string())
                    .block(block)
                    // .fg(TEXT_FG_COLOR)
                    .wrap(Wrap { trim: false });
                frame.render_widget(paragraph, area);
            }
        }
    }
}
