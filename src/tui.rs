use crate::mempool::MempoolEntry;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use std::{
    error::Error,
    io::{self, Stdout},
};

struct EntryInfo {
    index: usize,
    txid_string: String,
    wtxid_string: String,
}

// Which window is active for navigation
#[derive(PartialEq)]
enum FocusedWindow {
    TransactionList,
    TransactionDetail,
}

#[derive(PartialEq, Clone, Copy)]
enum IdMode {
    Txid,
    Wtxid,
}

// Vim-style modes
#[derive(PartialEq, Clone, Copy)]
enum InputMode {
    Normal,
    Insert,
}

pub struct TuiApp<'a> {
    entries: &'a [MempoolEntry],
    entry_infos: Vec<EntryInfo>,
    selected_index: usize,
    search_input: String,
    filtered_indices: Vec<usize>,
    focused_window: FocusedWindow,
    detail_scroll: u16,
    id_mode: IdMode,
    input_mode: InputMode,
    show_header_popup: bool,
    header_info: String,
    // For handling 'g' key press (waiting for second 'g')
    g_pressed: bool,
}

impl<'a> TuiApp<'a> {
    pub fn new(entries: &'a [MempoolEntry], header_info: String) -> Self {
        // Precompute all txids and wtxids and store them
        let entry_infos: Vec<EntryInfo> = (0..entries.len())
            .map(|idx| EntryInfo {
                index: idx,
                txid_string: entries[idx].transaction.compute_txid().to_string(),
                wtxid_string: entries[idx].transaction.compute_wtxid().to_string(),
            })
            .collect();

        let filtered_indices = (0..entries.len()).collect();
        Self {
            entries,
            entry_infos,
            selected_index: 0,
            search_input: String::new(),
            filtered_indices,
            focused_window: FocusedWindow::TransactionList,
            detail_scroll: 0,
            id_mode: IdMode::Txid,         // Default to txid mode
            input_mode: InputMode::Normal, // Start in normal mode
            show_header_popup: false,
            header_info,
            g_pressed: false,
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

        let result = self.run_app(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), Box<dyn Error>> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match self.input_mode {
                        // Normal mode - vim-like movement and commands
                        InputMode::Normal => {
                            // Reset g_pressed state on any key except 'g'
                            if !matches!(key.code, KeyCode::Char('g')) {
                                self.g_pressed = false;
                            }

                            match key.code {
                                KeyCode::Char('q') => return Ok(()),

                                // 'i' to enter insert mode (for search)
                                KeyCode::Char('i') => {
                                    if self.focused_window == FocusedWindow::TransactionList {
                                        self.input_mode = InputMode::Insert;
                                    }
                                }

                                // 'm' key to toggle between txid and wtxid modes
                                KeyCode::Char('m') => {
                                    if self.focused_window == FocusedWindow::TransactionList {
                                        self.id_mode = match self.id_mode {
                                            IdMode::Txid => IdMode::Wtxid,
                                            IdMode::Wtxid => IdMode::Txid,
                                        };
                                        // Re-filter with the new mode
                                        self.update_filtered_entries();
                                    }
                                }

                                // Tab key to switch focus between windows
                                KeyCode::Tab => {
                                    self.focused_window = match self.focused_window {
                                        FocusedWindow::TransactionList => {
                                            FocusedWindow::TransactionDetail
                                        }
                                        FocusedWindow::TransactionDetail => {
                                            FocusedWindow::TransactionList
                                        }
                                    };
                                    // Reset scroll when switching to detail view
                                    if self.focused_window == FocusedWindow::TransactionDetail {
                                        self.detail_scroll = 0;
                                    }
                                }

                                // Handle navigation keys based on focused window
                                KeyCode::Char('j') | KeyCode::Down => {
                                    if self.focused_window == FocusedWindow::TransactionList {
                                        if !self.filtered_indices.is_empty() {
                                            self.selected_index = (self.selected_index + 1)
                                                % self.filtered_indices.len();
                                        }
                                    } else {
                                        // Scroll down in transaction details
                                        self.detail_scroll = self.detail_scroll.saturating_add(1);
                                    }
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    if self.focused_window == FocusedWindow::TransactionList {
                                        if !self.filtered_indices.is_empty() {
                                            self.selected_index = if self.selected_index > 0 {
                                                self.selected_index - 1
                                            } else {
                                                self.filtered_indices.len() - 1
                                            };
                                        }
                                    } else {
                                        // Scroll up in transaction details
                                        self.detail_scroll = self.detail_scroll.saturating_sub(1);
                                    }
                                }

                                // Page up/down for both views
                                KeyCode::PageDown | KeyCode::Char('f') => {
                                    if self.focused_window == FocusedWindow::TransactionDetail {
                                        // Scroll down in transaction details
                                        self.detail_scroll = self.detail_scroll.saturating_add(10);
                                    } else if self.focused_window == FocusedWindow::TransactionList
                                    {
                                        // Move down in transaction list by 10 items
                                        if !self.filtered_indices.is_empty() {
                                            let list_len = self.filtered_indices.len();
                                            self.selected_index =
                                                (self.selected_index + 10).min(list_len - 1);
                                        }
                                    }
                                }
                                KeyCode::PageUp | KeyCode::Char('b') => {
                                    if self.focused_window == FocusedWindow::TransactionDetail {
                                        // Scroll up in transaction details
                                        self.detail_scroll = self.detail_scroll.saturating_sub(10);
                                    } else if self.focused_window == FocusedWindow::TransactionList
                                    {
                                        // Move up in transaction list by 10 items
                                        if !self.filtered_indices.is_empty() {
                                            self.selected_index =
                                                self.selected_index.saturating_sub(10);
                                        }
                                    }
                                }

                                // Clear search with 'c'
                                KeyCode::Char('c') => {
                                    if self.focused_window == FocusedWindow::TransactionList {
                                        self.search_input.clear();
                                        self.update_filtered_entries();
                                    }
                                }

                                // Toggle header popup with 'h'
                                KeyCode::Char('h') => {
                                    self.show_header_popup = !self.show_header_popup;
                                }

                                // Vim-style navigation: G to go to bottom, gg to go to top
                                KeyCode::Char('g') => {
                                    if self.focused_window == FocusedWindow::TransactionList {
                                        if self.g_pressed {
                                            // Second 'g' press - go to top
                                            if !self.filtered_indices.is_empty() {
                                                self.selected_index = 0;
                                            }
                                            self.g_pressed = false;
                                        } else {
                                            // First 'g' press - mark flag
                                            self.g_pressed = true;
                                        }
                                    }
                                }
                                KeyCode::Char('G') => {
                                    if self.focused_window == FocusedWindow::TransactionList {
                                        // Go to bottom
                                        if !self.filtered_indices.is_empty() {
                                            self.selected_index = self.filtered_indices.len() - 1;
                                        }
                                        // Reset the 'g' press state
                                        self.g_pressed = false;
                                    }
                                }

                                // ESC to return focus to transaction list from detail view or close popup
                                KeyCode::Esc => {
                                    if self.show_header_popup {
                                        self.show_header_popup = false;
                                    } else if self.focused_window
                                        == FocusedWindow::TransactionDetail
                                    {
                                        self.focused_window = FocusedWindow::TransactionList;
                                    }
                                }
                                _ => {}
                            }
                        }

                        // Insert mode - for text input
                        InputMode::Insert => {
                            match key.code {
                                // ESC to exit insert mode
                                KeyCode::Esc => {
                                    self.input_mode = InputMode::Normal;
                                }

                                // Typing characters for search
                                KeyCode::Char(c) => {
                                    if self.focused_window == FocusedWindow::TransactionList {
                                        self.search_input.push(c);
                                        self.update_filtered_entries();
                                    }
                                }

                                // Backspace for editing search
                                KeyCode::Backspace => {
                                    if self.focused_window == FocusedWindow::TransactionList {
                                        self.search_input.pop();
                                        self.update_filtered_entries();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    fn update_filtered_entries(&mut self) {
        if self.search_input.is_empty() {
            // If search is empty, show all entries
            self.filtered_indices = (0..self.entries.len()).collect();
            self.selected_index = 0;
            return;
        }

        let search_term = self.search_input.to_lowercase();

        // Use the appropriate ID string based on current mode
        self.filtered_indices = self
            .entry_infos
            .iter()
            .filter(|info| match self.id_mode {
                IdMode::Txid => info.txid_string.to_lowercase().contains(&search_term),
                IdMode::Wtxid => info.wtxid_string.to_lowercase().contains(&search_term),
            })
            .map(|info| info.index)
            .collect();

        // Reset selection if the list changed
        self.selected_index = 0;
    }

    fn ui(&self, f: &mut Frame) {
        // Create a main layout with a help bar at the bottom
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
            .split(f.area());

        // Create a vertically split layout for the main content
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
            .split(main_chunks[0]);

        // Left pane: Search and transaction list
        let left_chunk = chunks[0];
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
            .split(left_chunk);

        // Search input - show current modes in title
        let id_type = match self.id_mode {
            IdMode::Txid => "TxID",
            IdMode::Wtxid => "WTXID",
        };

        let input_mode_text = match self.input_mode {
            InputMode::Normal => "NORMAL (press 'i' to search)",
            InputMode::Insert => "INSERT (press Esc to exit)",
        };

        let search_title = format!("Search by {} | Mode: {}", id_type, input_mode_text);

        // Show cursor in insert mode
        let input_text = format!("Search: {}", self.search_input);
        let search_input = Paragraph::new(input_text)
            .block(Block::default().borders(Borders::ALL).title(search_title));

        // Set a different style based on input mode
        let search_input = if self.input_mode == InputMode::Insert {
            search_input.style(Style::default().fg(Color::Yellow))
        } else {
            search_input
        };
        f.render_widget(search_input, left_chunks[0]);

        // Transaction list - use precomputed IDs based on mode
        let transactions: Vec<ListItem> = self
            .filtered_indices
            .iter()
            .map(|&idx| {
                // Find the entry_info with matching index and use appropriate ID based on mode
                let id_string = match self.id_mode {
                    IdMode::Txid => &self.entry_infos[idx].txid_string,
                    IdMode::Wtxid => &self.entry_infos[idx].wtxid_string,
                };
                ListItem::new(id_string.clone())
            })
            .collect();

        // Add a special border style if this window is focused
        let transaction_list_block = if self.focused_window == FocusedWindow::TransactionList {
            Block::default()
                .borders(Borders::ALL)
                .title("Transactions [Active]")
                .border_style(Style::default().fg(Color::Yellow))
        } else {
            Block::default().borders(Borders::ALL).title("Transactions")
        };

        let transactions_list = List::new(transactions)
            .block(transaction_list_block)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        let mut state = ListState::default();
        if !self.filtered_indices.is_empty() {
            state.select(Some(self.selected_index));
        }
        f.render_stateful_widget(transactions_list, left_chunks[1], &mut state);

        // Right pane: Transaction details
        let right_chunk = chunks[1];

        // Add a special border style if this window is focused
        let transaction_detail_block = if self.focused_window == FocusedWindow::TransactionDetail {
            Block::default()
                .borders(Borders::ALL)
                .title("Transaction Details [Active] (Tab to switch, ↑/↓ to scroll)")
                .border_style(Style::default().fg(Color::Yellow))
        } else {
            Block::default()
                .borders(Borders::ALL)
                .title("Transaction Details (Tab to switch)")
        };

        // Show transaction details if there are filtered entries and a valid selection
        let content = if !self.filtered_indices.is_empty() {
            let entry_idx = self.filtered_indices[self.selected_index];
            let entry = &self.entries[entry_idx];
            format!("{:#?}", entry)
        } else {
            "No transaction selected".to_string()
        };

        let transaction_detail = Paragraph::new(content)
            .block(transaction_detail_block)
            .wrap(Wrap { trim: false })
            .scroll((self.detail_scroll, 0)); // Apply scrolling offset

        f.render_widget(transaction_detail, right_chunk);

        // Help bar at the bottom
        let help_text = match self.input_mode {
            InputMode::Normal => {
                " q: Quit | Tab: Switch Panes | i: Insert Mode | m: Toggle TxID/WTXID | c: Clear Search | h: Header Info | j/k: Navigate | PgDn/f, PgUp/b: Jump 10 | gg: Top | G: Bottom"
            }
            InputMode::Insert => " Esc: Normal Mode | Enter text to search",
        };

        let help_bar =
            Paragraph::new(help_text).style(Style::default().bg(Color::Blue).fg(Color::White));

        f.render_widget(help_bar, main_chunks[1]);

        // Render the header popup if it's active
        if self.show_header_popup {
            // Calculate popup dimensions
            let popup_width = 60;
            let popup_height = 8;
            let popup_x = (f.area().width.saturating_sub(popup_width)) / 2;
            let popup_y = (f.area().height.saturating_sub(popup_height)) / 2;

            // Create a centered popup area
            let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

            // Create a clear area behind the popup
            let clear_block = Block::default().style(Style::default().bg(Color::Black));
            f.render_widget(clear_block.clone(), popup_area);

            // Create the popup with header information
            let header_popup = Paragraph::new(self.header_info.clone())
                .block(
                    Block::default()
                        .title("Mempool Header Information")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow))
                        .style(Style::default().bg(Color::Black)),
                )
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false });

            f.render_widget(header_popup, popup_area);
        }
    }
}
