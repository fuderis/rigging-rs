[![github]](https://github.com/fuderis/rigging-rs)&ensp;
[![crates-io]](https://crates.io/crates/rigging)&ensp;
[![docs-rs]](https://docs.rs/rigging)

[github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
[crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
[docs-rs]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs

# Rigging: Inline TUI framework

**Rigging** is an asynchronous TUI framework for Rust designed to create reactive **inline** terminal interfaces
(such as interactive CLI utilities, AI chats, prompts, and dynamic widgets without switching to `EnterAlternateScreen`).<br>

The framework handles the complex terminal mechanics for you: safe terminal state management, dynamic viewport calculation,
precise frame re-rendering, code syntax highlighting, and user input handling.

## Key Features

* **Inline Rendering Engine**: Interactive rendering directly within the main terminal output stream without switching to an alternate screen buffer.

* **Built-in Widgets "Out of the Box"**:
  * **`Input`**: Multi-line input with support for placeholders, custom hotkeys (such as `Alt+Enter` to submit), padded layouts,
    standard `Vi` keybindings, and `Vim` navigation via `Alt + j/k/h/l`.
  * **`Text`**: Rendering text, animated spinners (`SpinnerStyle`), asynchronous data streams, and Markdown.

* **Markdown Support (`feature = "markdown"`)**: Automatic parsing and formatting for lists, tables, blockquotes, headers, and code blocks.

* **Syntax Highlighting (`feature = "highlight"`)**: Code highlighting inside Markdown code blocks powered by **Tree-sitter**,
    featuring customizable `CodeTheme` structures and preset themes.

* **Unicode & ANSI Aware Layout Engine**:
  * **Accurate Layouting (`unicode-width`)**: Calculates display width based on visual grapheme clusters rather than byte/character counts,
    preventing UI breaking on wide characters, CJK scripts, and multi-byte Emojis.
  * **Stateful ANSI Wrapping**: Dynamically tracks and carries over active SGR (ANSI escape code) styles across line wraps and frame boundaries,
    ensuring visual continuity without bleeding background colors or breaking reset states.

* **Smart Viewport Management**: When content exceeds the terminal height, only the latest (most relevant) lines are rendered during execution.
    Once finished (`is_finished() == true`), the widget expands and prints completely to prevent artifacts.

* **Optimization and Caching**: ANSI frame caching and frame re-rendering triggered strictly on changes (`is_changed() == true`).

* **Precise Cursor Positioning**: Accurate calculation of relative `(col, row)` cursor coordinates taking into account all borders, padding (`Padding`, `Margin`), and titles.

* **RAII Safety (`TerminalGuard`)**: Guaranteed Raw Mode reset and restoration of original terminal settings on exit or panic.

## Cargo Features

| Feature     | Description                                                                 |
| ---         | ---                                                                         |
| `markdown`  | Enables parsing and rich rendering of Markdown syntax in the `Text` widget. |
| `highlight` | Enables syntax highlighting within Markdown code blocks via Tree-sitter.    |


## Architecture and Core API

The framework is built around two fundamental concepts:

1. **`trait Widget`**: The core trait for custom and built-in widgets.
2. **`struct Block<W>`**: A decorator and rendering engine that manages styling, backgrounds, borders, post-processing animations (`blink`), and the asynchronous render loop.

### The `Widget` Trait

```rust
use rigging::crossterm::event::KeyEvent;

pub trait Widget {
    /// External state type owned by the application (read-only snapshot source).
    type State: Default + Clone + Sync + Send + 'static;

    /// The result type emitted when the widget completes its lifecycle.
    type Output;

    /// Internal event type dispatched via `Context::send_event`.
    type Event: Send + 'static;

    /// Indicates whether *internal widget-local state* changed and requires a redraw.
    ///
    /// User `State` updates are tracked externally by the renderer, so widgets without
    /// internal visual logic (e.g., animations) should return `false`.
    fn is_changed(&self) -> bool {
        false
    }

    /// Indicates whether the internal processing of the widget is complete.
    ///
    /// Custom handlers run synchronously before frame evaluation, so widgets without
    /// complex background logic can safely return `true`.
    fn is_finished(&self) -> bool {
        true
    }

    /// Renders a frame using the latest immutable snapshot of `State`.
    ///
    /// # Arguments
    /// * `state` - Read-only snapshot of the shared state.
    /// * `max_width` - Optional horizontal bounding constraint.
    /// * `max_height` - Optional vertical bounding constraint.
    /// * `is_final` - `true` if this is the final render pass before termination.
    fn render_frame(&mut self, state: &Arc<Self::State>, max_width: Option<usize>, max_height: Option<usize>, is_final: bool) -> Vec<String>;

    /// Handles raw keyboard input events from the terminal.
    fn handle_key(&mut self, _key: KeyEvent) {}

    /// Handles custom domain events emitted from background tasks or `Context`.
    fn handle_event(&mut self, _event: Self::Event) {}

    /// Responds to terminal or container resize notifications.
    fn on_resize(&mut self, _cols: u16, _rows: u16) {}

    /// Returns relative cursor coordinates `(column, row)` within the widget, if active.
    fn cursor_position(&self) -> Option<(usize, usize)> {
        None
    }

    /// Extracts the execution result without consuming `self`.
    ///
    /// Allows the widget instance to persist in memory (e.g., inside terminal history)
    /// while yielding its output to the engine.
    fn extract_output(&mut self) -> Self::Output;
}
```

## Input Widget & Vi/Vim Navigation

The built-in `Input` widget provides full-featured text entry capabilities:

* **Vi-Standard Shortcuts**: Supports standard Vi text editing operations and keybindings out of the box.
* **Vim Cursor Navigation**: Fast inline cursor movement using `Alt + h` (left), `Alt + j` (down), `Alt + k` (up), and `Alt + l` (right).
* **Multi-line Support**: Allows multi-line editing with submission triggers `Alt+Enter` or `Ctrl + j`.

## Code Highlighting and Themes (`feature = "highlight"`)

When the `highlight` feature is enabled, code blocks within Markdown are syntax-highlighted using the Tree-sitter AST parser.

### Preset Themes (`rigging::theme`)

* `Catppuccin`
* `Atom`
* `Dracula`
* `VS Code`
* `GitHub`

### Custom Theme (`CodeTheme`)

You can fully customize the color palette to match your interface:

```rust
use rigging::Color;

#[derive(Debug, Clone, Copy)]
pub struct CodeTheme {
    pub keyword: Color,     // Control keywords (fn, let, match)
    pub type_name: Color,   // Types, structs, enums
    pub function: Color,    // Functions and method calls
    pub macro_name: Color,  // Macros (println!, vec!)
    pub builtin: Color,     // Built-in primitives
    pub operator: Color,    // Operators (+, -, =>)
    pub string: Color,      // String literals
    pub number: Color,      // Numbers and booleans
    pub comment: Color,     // Comments
    pub variable: Color,    // Variables
    pub property: Color,    // Struct fields and properties
    pub constant: Color,    // Constants
}
```

## Viewport Management and Terminal Resizing

1. **Dynamic Viewport**: During interactive execution (e.g., while an AI streams a long response),
  the widget restricts its height to the current terminal viewport (`viewport_height`).
  If output exceeds screen height, the view automatically scrolls to display the **latest lines**.

2. **Final Expansion**: When a widget completes its execution (`is_finished() == true`), height constraints are removed (`max_height = None`),
  the temporary dynamic frame is cleared, and the complete widget content is printed into the terminal stream. This eliminates truncated artifacts in terminal scrollback history.

3. **ANSI & Unicode Line Wrapping**: During line wrapping, active ANSI formatting sequence states are preserved and automatically prepended to line continuations.
  Visual character widths are computed via `unicode-width` to guarantee exact alignment regardless of Emojis or wide characters.

4. **Resize Handling**: On terminal window resize events (`Event::Resize`), border caches are cleared, previous frames are wiped,
  and cursor positions are reset to ensure a clean re-render under new dimensions.

## Complete Example: AI CLI Chat

Below is a full example demonstrating the built-in `Input` and `Text` widgets to build an AI CLI chat with an asynchronous loading spinner (`SpinnerStyle`),
streaming Markdown output, and custom color palettes.

```rust
use std::time::Duration;

use rigging::{
    Color, Stylize,
    render::SubWidgetPosition,
    style::{Align, BorderStyle, LineStyle, Margin, Padding, SpinnerStyle},
    widgets::{
        Input, Text,
        confirm::{ConfirmPrompt, Confirmation},
    },
};

// =============================================================================
// GEOMETRY & PALETTE CONFIGURATION
// =============================================================================

const MIN_WIDTH: usize = 80;
const MAX_WIDTH: usize = 100;

const BRAND_COLOR: Color = Color::Rgb { r: 255, g: 85, b: 65 };
const BG_COLOR: Color = Color::Rgb { r: 15, g: 20, b: 35 };
const ALT_COLOR: Color = Color::Rgb { r: 140, g: 120, b: 81 };
const BLINK_COLOR: Color = Color::Rgb { r: 20, g: 26, b: 42 };

// =============================================================================
// ENTRY POINT
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let response_chunks = get_test_response();

    loop {
        // -----------------------------------------------------------------
        // 1. Input Phase (User Query)
        // -----------------------------------------------------------------
        let user_query = Input::new()
            .placeholder("Enter instructions...".with(ALT_COLOR))
            .title(" Prompt ".bold().with(BRAND_COLOR), Align::TopLeft)
            .title(
                " Qwen3 Coder Plus ".bold().with(BRAND_COLOR),
                Align::BottomLeft,
            )
            .title(
                " [Alt+Enter] Submit ".bold().with(ALT_COLOR),
                Align::BottomRight,
            )
            .min_width(MIN_WIDTH)
            .max_width(MAX_WIDTH)
            .border_style(BorderStyle::Rounded)
            .border_color(BRAND_COLOR)
            .background_color(BG_COLOR)
            .padding(Padding::hor(1))
            .show_cursor(true)
            .multiline(true)
            .clear_after(true)
            .render()
            .await?;

        let trimmed_query = user_query.trim().to_string();

        if trimmed_query.is_empty() {
            continue;
        }

        if trimmed_query == "/exit" || trimmed_query == "/quit" {
            return Ok(());
        }

        // -----------------------------------------------------------------
        // 2. Continuous Processing Phase (Thinking -> Confirm -> Response)
        // -----------------------------------------------------------------
        let chunks = response_chunks.clone();
        let timestamp = "Fri 05:31 AM";

        let user_prefix = format!("{} {}", "You:".bold().with(ALT_COLOR), trimmed_query.dim());

        #[allow(unused_mut)]
        let mut text = Text::new(user_prefix)
            .title(format!(" {timestamp} ").with(BRAND_COLOR), Align::TopLeft)
            .min_width(MIN_WIDTH)
            .max_width(MAX_WIDTH)
            .spinner_style(SpinnerStyle::Dots)
            .spinner_color(BRAND_COLOR)
            .prefix_color(ALT_COLOR)
            .prefix_line(LineStyle::Solid)
            .border_style(BorderStyle::Rounded)
            .border_color(BRAND_COLOR)
            .background_color(BG_COLOR)
            .padding(Padding::hor(1))
            .margin(Margin {
                bottom: 1,
                ..Default::default()
            })
            .handler(move |mut ctx| async move {
                // --- Step A: Initial Thinking & Progress ---
                let steps = [
                    "Parsing markdown syntax tree...",
                    "Analyzing system security policies...",
                ];

                for (i, step) in steps.iter().enumerate() {
                    tokio::time::sleep(Duration::from_millis(300)).await;

                    let pct = ((i + 1) * 100) / steps.len();
                    let filled = (pct / 10).min(10);
                    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled));

                    *ctx.state = format!("Thinking: {step} [{bar}] {pct}%");
                    ctx.notify();
                }

                tokio::time::sleep(Duration::from_millis(200)).await;

                // --- Step B: Push Sub-Widget via Context Runner ---
                let confirm_widget =
                    ConfirmPrompt::new("Execute action with elevated root (sudo) privileges?")
                        .default(Confirmation::No)
                        .border_style(BorderStyle::Rounded)
                        .border_color(BRAND_COLOR)
                        .background_color(BG_COLOR)
                        .clear_after(true);

                let confirmation_result: std::io::Result<Option<Confirmation>> = ctx
                    .push_sub_widget(SubWidgetPosition::Replace, false, move |writer, width| {
                        Box::pin(async move {
                            let (res, lines) = confirm_widget.render_to(writer, width).await?;
                            Ok((Box::new(res) as Box<dyn std::any::Any + Send>, lines))
                        })
                    })
                    .await;

                // --- Step C: Streaming LLM Response ---
                let mut body_buffer = String::new();

                match confirmation_result {
                    Ok(Some(Confirmation::Yes)) => {
                        body_buffer.push_str("> **Privileged Execution Context Active**\n\n");
                    }
                    _ => {
                        body_buffer.push_str("> **Sandboxed Mode Active (Non-root)**\n\n");
                    }
                }

                for chunk in chunks {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                    body_buffer.push_str(&chunk);

                    *ctx.state = body_buffer.clone();
                    ctx.notify();
                }

                ctx.finish();
            })
            .blink_color(BLINK_COLOR);

        #[cfg(feature = "markdown")]
        {
            text = text.stripe_color(ALT_COLOR);
            text = text.bullet_color(ALT_COLOR);
            text = text.code_color(ALT_COLOR);
        }

        text.render().await?;
    }
}

// =============================================================================
// MOCK DATA PROVIDERS
// =============================================================================

fn get_test_response() -> Vec<String> {
    vec![
        "# Main System Architecture\n\n".to_string(),
        "Here is the high-level breakdown of the active async pipeline:\n\n".to_string(),
        "## 1. Async Event Loop Implementation\n\n".to_string(),
        "```rust\n".to_string(),
        "use tokio::sync::mpsc;\n\n".to_string(),
        "#[derive(Debug)]\n".to_string(),
        "pub enum Command {\n".to_string(),
        "    SendPayload(Vec<u8>),\n".to_string(),
        "    Shutdown,\n".to_string(),
        "}\n\n".to_string(),
        "pub async fn run_actor(mut rx: mpsc::Receiver<Command>) -> Result<(), String> {\n".to_string(),
        "    while let Some(cmd) = rx.recv().await {\n".to_string(),
        "        match cmd {\n".to_string(),
        "            Command::SendPayload(bytes) => println!(\"Processing {} bytes\", bytes.len()),\n".to_string(),
        "            Command::Shutdown => break,\n".to_string(),
        "        }\n".to_string(),
        "    }\n".to_string(),
        "    Ok(())\n".to_string(),
        "}\n".to_string(),
        "```\n\n".to_string(),
        "### System Benchmarks & Status\n\n".to_string(),
        "| Subsystem Module | Latency Profile | Memory Overhead | Status |\n".to_string(),
        "| --- | --- | --- | --- |\n".to_string(),
        "| `auth_core` | `0.45 ms` (p95) | `1.2 MB` | *Nominal* |\n".to_string(),
        "| `stream_pipe` | `1.02 ms` (p95) | `4.8 MB` | *Stable* |\n".to_string(),
        "| `crypto_vault` | `0.18 ms` (p99) | `512 KB` | **Optimal** |\n\n".to_string(),
        "### Key Feature Checklist\n\n".to_string(),
        "- [x] Integrated `tokio::mpsc` channels for thread safety\n".to_string(),
        "- [x] Inline parsing for `code_color` customization\n".to_string(),
        "- [ ] Multi-node cluster failover support\n\n".to_string(),
        "#### Execution Steps & Workflow\n\n".to_string(),
        "1. Initialize logger via `tracing_subscriber`.\n".to_string(),
        "2. Parse workspace config from `Cargo.toml` file.\n".to_string(),
        "3. Spawn background workers:\n".to_string(),
        "   * Worker Alpha: `rx_pipe` listener\n".to_string(),
        "   * Worker Beta: `tx_heartbeat` monitor\n".to_string(),
        "4. Validate security credentials before execution.\n\n".to_string(),
        "> **Important Note:** Code blocks are formatted using custom palette colors. ".to_string(),
        "Verify that `ALT_COLOR` is applied to all quote borders and code lines.".to_string(),
    ]
}
```

## License & Feedback:

> Distributed under the [MIT](https://github.com/fuderis/rigging-rs/blob/main/LICENSE.md) license.

You can contact me via [GitHub](https://github.com/fuderis) or send a message to my [E-Mail](mailto:synapdrake@ya.ru).
This library is actively evolving, and your suggestions and feedback are always welcome!
