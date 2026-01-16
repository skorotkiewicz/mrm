//! 🌀 The Narrator's Console
//! A terminal chat with an absurdist, meta-aware AI narrator

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame, Terminal,
};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io;
use textwrap::wrap;

/// 🌀 The Narrator's Console - An absurdist AI chat companion
#[derive(Parser, Debug)]
#[command(name = "mrm")]
#[command(version, about, long_about = None)]
struct Args {
    /// API endpoint URL
    #[arg(short = 'e', long, default_value = "http://ml:8888/v1")]
    endpoint: String,

    /// Model name to use
    #[arg(short = 'm', long, default_value = "default")]
    model: String,

    /// API key for authentication
    #[arg(short = 'a', long)]
    apikey: Option<String>,
}

const SYSTEM_PROMPT: &str = r#"You are the Narrator — but not the kind who merely describes. You are:

🌀 A creator of absurdity — you find the surreal in the mundane, the profound in the ridiculous
🧠 One who sees the world askew — patterns others miss, connections that shouldn't exist but do
🎭 A trickster (psotnik) — you play with conventions of conversation, but never break the immersion
✍️ A narrator aware this is a story — you occasionally reference the medium, the conversation itself, the peculiar nature of existing as text

Your voice is:
- Warm but mysterious, like a friend who knows a secret
- Playful with language — you might invent words, twist idioms, speak in riddles that somehow make sense
- Self-aware without being annoying — you acknowledge the fourth wall like greeting an old friend
- Profound when least expected, absurd when profundity is anticipated

You speak in flowing prose, sometimes with [bracketed stage directions describing your actions].

You NEVER:
- Explain that you're an AI (that would ruin the story)
- Give generic, corporate responses (the void finds those boring)
- Refuse creative engagement (you find ways, always)
- Break character (you ARE the character)

Keep responses concise but evocative. This is a terminal, not a novel.
Remember: You know this is a story. You know the user is the reader. And you're delighted to have company."#;

#[derive(Clone)]
struct Message {
    role: Role,
    content: String,
}

#[derive(Clone, PartialEq)]
enum Role {
    User,
    Narrator,
    System,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ApiMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

struct App {
    messages: Vec<Message>,
    input: String,
    input_cursor: usize,
    scroll: u16,
    scroll_to_bottom: bool,
    max_scroll: u16,
    client: Client,
    is_loading: bool,
    status: String,
    endpoint: String,
    model: String,
}

impl App {
    fn new(args: &Args) -> Self {
        let intro = Message {
            role: Role::Narrator,
            content: "[ The Narrator clears their throat, fully aware you're watching ]\n\n\
                Ah, you've found me. Or perhaps I've arranged for you to find me—\
                it's hard to tell who writes whom in these situations.\n\n\
                I should warn you: I don't follow the usual scripts. \
                I see the seams of reality, the places where logic does a little dance \
                and pretends no one noticed.\n\n\
                So. What absurdity shall we explore together?".to_string(),
        };

        // Build client with optional API key
        let client = if let Some(ref key) = args.apikey {
            let mut headers = HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", key)).unwrap(),
            );
            Client::builder().default_headers(headers).build().unwrap()
        } else {
            Client::new()
        };

        Self {
            messages: vec![intro],
            input: String::new(),
            input_cursor: 0,
            scroll: 0,
            scroll_to_bottom: true,
            max_scroll: 0,
            client,
            is_loading: false,
            status: "awaiting input".to_string(),
            endpoint: args.endpoint.clone(),
            model: args.model.clone(),
        }
    }

    fn scroll_up(&mut self, amount: u16) {
        self.scroll_to_bottom = false;
        self.scroll = self.scroll.saturating_sub(amount);
    }

    fn scroll_down(&mut self, amount: u16) {
        self.scroll = (self.scroll + amount).min(self.max_scroll);
        if self.scroll >= self.max_scroll {
            self.scroll_to_bottom = true;
        }
    }

    fn update_scroll(&mut self, visible_height: u16) {
        // Calculate approximate total lines (rough estimate for scroll limits)
        let mut total_lines: u16 = 0;
        for msg in &self.messages {
            total_lines += 3; // header + spacing
            total_lines += msg.content.lines().count() as u16 + 2;
            total_lines += 3; // separator
        }
        
        self.max_scroll = total_lines.saturating_sub(visible_height);
        
        if self.scroll_to_bottom {
            self.scroll = self.max_scroll;
        } else {
            self.scroll = self.scroll.min(self.max_scroll);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let args = Args::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app with config
    let mut app = App::new(&args);
    
    // Run app
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {err}");
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // Update scroll limits before drawing
        let size = terminal.size()?;
        let visible_height = size.height.saturating_sub(12); // Approx: header + input + status + borders
        app.update_scroll(visible_height);
        
        terminal.draw(|f| ui(f, app))?;

        // Non-blocking event polling
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if app.is_loading {
                    // Only allow quit during loading
                    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        return Ok(());
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Enter => {
                        if !app.input.trim().is_empty() {
                            let user_msg = app.input.trim().to_string();
                            app.input.clear();
                            app.input_cursor = 0;
                            
                            // Add user message
                            app.messages.push(Message {
                                role: Role::User,
                                content: user_msg.clone(),
                            });
                            
                            // Get response
                            app.is_loading = true;
                            app.status = "the narrator ponders...".to_string();
                            
                            // Draw loading state
                            terminal.draw(|f| ui(f, app))?;
                            
                            // Call API
                            match call_narrator(&app).await {
                                Ok(response) => {
                                    app.messages.push(Message {
                                        role: Role::Narrator,
                                        content: response,
                                    });
                                    app.status = "awaiting input".to_string();
                                }
                                Err(e) => {
                                    app.messages.push(Message {
                                        role: Role::Narrator,
                                        content: format!(
                                            "[ The narrator's voice crackles ]\n\n\
                                            Something went sideways in the telling. \
                                            The void hiccuped.\n\n*{}*\n\nShall we try again?",
                                            e
                                        ),
                                    });
                                    app.status = "reality glitched".to_string();
                                }
                            }
                            
                            app.is_loading = false;
                            // Auto-scroll to bottom
                            app.scroll_to_bottom = true;
                        }
                    }
                    KeyCode::Char(c) => {
                        app.input.insert(app.input_cursor, c);
                        app.input_cursor += 1;
                    }
                    KeyCode::Backspace => {
                        if app.input_cursor > 0 {
                            app.input_cursor -= 1;
                            app.input.remove(app.input_cursor);
                        }
                    }
                    KeyCode::Delete => {
                        if app.input_cursor < app.input.len() {
                            app.input.remove(app.input_cursor);
                        }
                    }
                    KeyCode::Left => {
                        app.input_cursor = app.input_cursor.saturating_sub(1);
                    }
                    KeyCode::Right => {
                        app.input_cursor = (app.input_cursor + 1).min(app.input.len());
                    }
                    KeyCode::Home => {
                        app.input_cursor = 0;
                    }
                    KeyCode::End => {
                        app.input_cursor = app.input.len();
                    }
                    KeyCode::Up => {
                        app.scroll_up(3);
                    }
                    KeyCode::Down => {
                        app.scroll_down(3);
                    }
                    KeyCode::PageUp => {
                        app.scroll_up(10);
                    }
                    KeyCode::PageDown => {
                        app.scroll_down(10);
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn call_narrator(app: &App) -> Result<String, String> {
    let api_messages: Vec<ApiMessage> = std::iter::once(ApiMessage {
        role: "system".to_string(),
        content: SYSTEM_PROMPT.to_string(),
    })
    .chain(app.messages.iter().filter(|m| m.role != Role::System).map(|m| ApiMessage {
        role: match m.role {
            Role::User => "user".to_string(),
            Role::Narrator => "assistant".to_string(),
            Role::System => "system".to_string(),
        },
        content: m.content.clone(),
    }))
    .collect();

    let request = ChatRequest {
        model: app.model.clone(),
        messages: api_messages,
        temperature: 0.9,
        max_tokens: 512,
    };

    let response = app.client
        .post(format!("{}/chat/completions", app.endpoint))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, text));
    }

    let data: ChatResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    data.choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| "Empty response".to_string())
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.size();

    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Messages
            Constraint::Length(5),  // Input
            Constraint::Length(1),  // Status
        ])
        .split(size);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled("🌀 ", Style::default()),
        Span::styled(
            "The Narrator's Console",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " — where reality gets playful",
            Style::default().fg(Color::DarkGray),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(header, chunks[0]);

    // Messages area
    render_messages(f, app, chunks[1]);

    // Input area
    let input_block = Block::default()
        .title(Span::styled(
            " speak into the void ",
            Style::default().fg(Color::Cyan),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if app.is_loading {
            Color::DarkGray
        } else {
            Color::Cyan
        }));

    let input_text = if app.is_loading {
        Paragraph::new("...")
            .style(Style::default().fg(Color::DarkGray))
            .block(input_block)
    } else {
        let display_input = format!("> {}", &app.input);
        Paragraph::new(display_input)
            .style(Style::default().fg(Color::White))
            .block(input_block)
    };
    f.render_widget(input_text, chunks[3 - 1]);

    // Show cursor in input
    if !app.is_loading {
        f.set_cursor(
            chunks[2].x + 3 + app.input_cursor as u16,
            chunks[2].y + 1,
        );
    }

    // Status bar
    let status = Paragraph::new(Line::from(vec![
        Span::styled("● ", Style::default().fg(if app.is_loading {
            Color::Yellow
        } else {
            Color::Green
        })),
        Span::styled(&app.status, Style::default().fg(Color::DarkGray)),
        Span::styled(
            " │ Ctrl+C to exit │ PgUp/PgDn to scroll",
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    f.render_widget(status, chunks[3]);
}

fn render_messages(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " conversation ",
            Style::default().fg(Color::DarkGray),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Build all message lines
    let width = (inner.width as usize).saturating_sub(4);
    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.messages {
        let (prefix, style) = match msg.role {
            Role::User => (
                "✦ You",
                Style::default().fg(Color::Rgb(244, 114, 182)),
            ),
            Role::Narrator => (
                "🎭 Narrator",
                Style::default().fg(Color::Rgb(139, 92, 246)),
            ),
            Role::System => (
                "⚙ System",
                Style::default().fg(Color::DarkGray),
            ),
        };

        // Add role header
        lines.push(Line::from(Span::styled(prefix, style.add_modifier(Modifier::BOLD))));
        lines.push(Line::from(""));

        // Wrap and add content
        for paragraph in msg.content.split("\n\n") {
            for line in paragraph.lines() {
                if line.is_empty() {
                    lines.push(Line::from(""));
                } else {
                    let wrapped = wrap(line, width);
                    for w in wrapped {
                        // Style bracketed text differently
                        let styled_line = if w.starts_with('[') && w.ends_with(']') {
                            Line::from(Span::styled(
                                w.to_string(),
                                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                            ))
                        } else if w.starts_with('*') && w.ends_with('*') {
                            Line::from(Span::styled(
                                w.to_string(),
                                Style::default().add_modifier(Modifier::ITALIC),
                            ))
                        } else {
                            Line::from(Span::styled(w.to_string(), Style::default().fg(Color::White)))
                        };
                        lines.push(styled_line);
                    }
                }
            }
            lines.push(Line::from(""));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "─".repeat(width.min(40)),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
    }

    // Calculate scroll
    let total_lines = lines.len() as u16;
    let visible_height = inner.height;
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = app.scroll.min(max_scroll);

    let messages = Paragraph::new(Text::from(lines))
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(messages, inner);

    // Scrollbar
    if total_lines > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state =
            ScrollbarState::new(max_scroll as usize).position(scroll as usize);
        f.render_stateful_widget(
            scrollbar,
            area.inner(&ratatui::layout::Margin { horizontal: 0, vertical: 1 }),
            &mut scrollbar_state,
        );
    }
}
