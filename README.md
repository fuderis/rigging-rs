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
use crossterm::event::KeyEvent;

pub trait Widget: Send + Sync {
    type Output;

    /// Signal indicating a new frame needs to be rendered
    fn is_changed(&self) -> bool;

    /// Generates content lines fitted to the available `width`
    fn render_content(&mut self, width: usize) -> Vec<String>;

    /// Handles keyboard input
    fn handle_key(&mut self, _key: KeyEvent) {}

    /// Handles terminal resize events
    fn on_resize(&mut self, _cols: u16, _rows: u16) {}

    /// Completion flag; when `true`, the render loop stops
    fn is_finished(&self) -> bool { true }

    /// Extracts the final execution result
    fn extract_output(self) -> Self::Output;

    /// Relative cursor position `(col, row)`
    fn cursor_position(&self) -> Option<(usize, usize)> { None }

    /// Indicates whether the widget needs a visible cursor.
    fn show_cursor(&self) -> bool {
        self.cursor_position().is_some()
    }
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
  the temporary dynamic frame is cleared, and the complete widget content is printed into the terminal stream.
  This eliminates truncated artifacts in terminal scrollback history.

3. **Resize Handling**: On terminal window resize events (`Event::Resize`), border caches are cleared, previous frames are wiped,
  and cursor positions are reset to ensure a clean re-render under new dimensions.

## Complete Example: AI CLI Chat

Below is a full example demonstrating the built-in `Input` and `Text` widgets to build an AI CLI chat with an asynchronous loading spinner (`SpinnerStyle`),
streaming Markdown output, and custom color palettes.

```rust
use std::{error::Error, time::Duration};
use rigging::{
    style::{Align, BorderStyle, LineStyle, Margin, Padding, SpinnerStyle},
    widgets::{Input, Text},
    Color, Stylize,
};

const MIN_WIDTH: usize = 70;
const MAX_WIDTH: usize = 70;

const BRAND_COLOR: Color = Color::Rgb { r: 255, g: 85, b: 65 };  // Red accent (#FF5541)
const BG_COLOR: Color = Color::Rgb { r: 15, g: 20, b: 35 };      // Dark blue background (#0F1423)
const ALT_COLOR: Color = Color::Rgb { r: 140, g: 120, b: 81 };   // Gold accent (#8C7851)
const BLINK_COLOR: Color = Color::Rgb { r: 20, g: 26, b: 42 };   // Blink flash color on completion

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let response_chunks = get_test_response();

    loop {
        // 1. Input Phase (supports Vi keybindings and Alt + h/j/k/l navigation)
        let user_query = Input::new()
            .placeholder("Enter instructions...".with(ALT_COLOR))
            .title(" Prompt ".bold().with(BRAND_COLOR), Align::TopLeft)
            .title(" Qwen3 Coder Plus ".bold().with(BRAND_COLOR), Align::BottomLeft)
            .title(" [Alt+Enter] Submit ".bold().with(ALT_COLOR), Align::BottomRight)
            .min_width(MIN_WIDTH)
            .max_width(MAX_WIDTH)
            .border(BorderStyle::Rounded)
            .border_color(BRAND_COLOR)
            .background(BG_COLOR)
            .padding(Padding::gor(1))
            .multiline(true)
            .clear_after(true) // Clear input block after submission
            .render()
            .await?;

        let trimmed_query = user_query.trim();
        if trimmed_query.is_empty() { continue; }
        if trimmed_query == "/exit" || trimmed_query == "/quit" { break; }

        // 2. Status Phase (Thinking Indicator with Spinner)
        Text::new("")
            .title(" Thinking... ".bold().with(BRAND_COLOR), Align::TopLeft)
            .min_width(MIN_WIDTH)
            .max_width(MAX_WIDTH)
            .spinner_style(SpinnerStyle::Dots)
            .spinner_color(BRAND_COLOR)
            .border(BorderStyle::Rounded)
            .border_color(BRAND_COLOR)
            .background(BG_COLOR)
            .padding(Padding::gor(1))
            .handler(|handle| async move {
                let steps = [
                    "Parsing markdown syntax tree...",
                    "Applying ALT_COLOR styles to tokens...",
                    "Rendering complex UI layout...",
                ];
                for step in steps {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    handle.update(step);
                }
            })
            .clear_after(true)
            .render()
            .await?;

        // 3. Streamed Markdown Response Phase
        let chunks = response_chunks.clone();
        
        Text::new(format!(
            "{} {}",
            "You:".bold().with(ALT_COLOR),
            trimmed_query.dim()
        ))
        .title(" Fri 05:31 AM ".with(BRAND_COLOR), Align::TopLeft)
        .min_width(MIN_WIDTH)
        .max_width(MAX_WIDTH)
        .spinner_style(SpinnerStyle::MiniDots)
        .spinner_color(BRAND_COLOR)
        .prefix_color(ALT_COLOR)
        .prefix_line(LineStyle::Solid)
        // Markdown styling configuration
        .stripe_color(ALT_COLOR)  // Blockquotes and containers
        .bullet_color(ALT_COLOR)  // Unordered and ordered lists
        .code_color(ALT_COLOR)    // Inline code elements
        .border(BorderStyle::Rounded)
        .border_color(ALT_COLOR)
        .background(BG_COLOR)
        .padding(Padding::gor(1))
        .margin(Margin { bottom: 1, ..Default::default() })
        .handler(move |handle| async move {
            let mut current_buffer = String::new();
            tokio::time::sleep(Duration::from_millis(200)).await;

            for chunk in chunks {
                tokio::time::sleep(Duration::from_millis(25)).await;
                current_buffer.push_str(&chunk);
                handle.update(current_buffer.clone()); // Dynamically updates frame
            }
        })
        .blink_color(BLINK_COLOR) // Flash effect on completion
        .render()
        .await?;
    }

    Ok(())
}

fn get_test_response() -> Vec<String> {
    vec![
        "# Main System Architecture\n\n".to_string(),
        "```rust\npub async fn run_actor() -> Result<(), String> {\n    Ok(())\n}\n```\n\n".to_string(),
        "| Subsystem | Latency | Status |\n| --- | --- | --- |\n| `auth_core` | `0.45 ms` | *Nominal* |\n\n".to_string(),
        "- [x] Integrated `tokio::mpsc` channels\n- [ ] Multi-node cluster failover\n\n".to_string(),
        "> **Note:** Verify that `ALT_COLOR` is applied to quote borders.".to_string(),
    ]
}
```

## License & Feedback:

> Distributed under the [MIT](https://github.com/fuderis/rigging-rs/blob/main/LICENSE.md) license.

You can contact me via [GitHub](https://github.com/fuderis) or send a message to my [E-Mail](mailto:synapdrake@ya.ru).
This library is actively evolving, and your suggestions and feedback are always welcome!
