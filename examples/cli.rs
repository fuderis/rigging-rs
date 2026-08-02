//! Demonstration binary for the `rigging` TUI framework showcasing interactive
//! widgets, async handlers, status indicators, and streaming markdown text rendering.

use std::{error::Error, time::Duration};

use rigging::{
    Color, Stylize,
    style::{Align, BorderStyle, LineStyle, Margin, Padding, SpinnerStyle},
    widgets::{Input, Text},
};

// =============================================================================
//  GEOMETRY & PALETTE CONFIGURATION
// =============================================================================

/// Minimum layout width constraint for widgets.
const MIN_WIDTH: usize = 70;

/// Maximum layout width constraint for widgets.
const MAX_WIDTH: usize = 70;

/// Vibrant red brand accent color (`#FF5541`).
const BRAND_COLOR: Color = Color::Rgb {
    r: 255,
    g: 85,
    b: 65,
};

/// Soft, deep dark navy background color (`#0F1423`).
const BG_COLOR: Color = Color::Rgb {
    r: 15,
    g: 20,
    b: 35,
};

/// Muted golden-brown shade used for borders, quotes, and secondary accents (`#8C7851`).
const ALT_COLOR: Color = Color::Rgb {
    r: 140,
    g: 120,
    b: 81,
};

/// Slightly lighter dark navy tint used for blinking focus/cursor effects.
const BLINK_COLOR: Color = Color::Rgb {
    r: 20,
    g: 26,
    b: 42,
};

// =============================================================================
//  ENTRY POINT & DEMONSTRATION LOOP
// =============================================================================

/// runs the main interactive CLI event loop.
///
/// handles user prompts via `Input`, displays an asynchronous processing spinner,
/// and streams a simulated LLM Markdown response using `Text` widgets.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let response_chunks = get_test_response();

    loop {
        // -----------------------------------------------------------------
        //  1. Input Phase (User Query)
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
            .border(BorderStyle::Rounded)
            .border_color(BRAND_COLOR)
            .background(BG_COLOR)
            .padding(Padding::hor(1))
            .multiline(true)
            .clear_after(true)
            .render()
            .await?;

        let trimmed_query = user_query.trim();

        if trimmed_query.is_empty() {
            continue;
        }

        if trimmed_query == "/exit" || trimmed_query == "/quit" {
            return Ok(());
        }

        // -----------------------------------------------------------------
        //  2. Processing / Status Phase (Spinner Indicator)
        // -----------------------------------------------------------------
        Text::new("")
            .title(" Thinking... ".bold().with(BRAND_COLOR), Align::TopLeft)
            .min_width(MIN_WIDTH)
            .max_width(MAX_WIDTH)
            .spinner_style(SpinnerStyle::Dots)
            .spinner_color(BRAND_COLOR)
            .border(BorderStyle::Rounded)
            .border_color(BRAND_COLOR)
            .background(BG_COLOR)
            .padding(Padding::hor(1))
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

        // -----------------------------------------------------------------
        //  3. Streamed Response Phase (AI Output)
        // -----------------------------------------------------------------
        let chunks = response_chunks.clone();
        let timestamp = "Fri 05:31 AM";

        #[allow(unused_mut)]
        let mut text = Text::new(format!(
            "{} {}",
            "You:".bold().with(ALT_COLOR),
            trimmed_query.dim()
        ))
        .title(format!(" {timestamp} ").with(BRAND_COLOR), Align::TopLeft)
        .min_width(MIN_WIDTH)
        .max_width(MAX_WIDTH)
        .spinner_style(SpinnerStyle::MiniDots)
        .spinner_color(BRAND_COLOR)
        .prefix_color(ALT_COLOR)
        .prefix_line(LineStyle::Solid)
        .border(BorderStyle::Rounded)
        .border_color(ALT_COLOR)
        .background(BG_COLOR)
        .padding(Padding::hor(1))
        .margin(Margin {
            bottom: 1,
            ..Default::default()
        })
        .handler(move |handle| async move {
            let mut current_buffer = String::new();
            tokio::time::sleep(Duration::from_millis(200)).await;

            for chunk in chunks {
                tokio::time::sleep(Duration::from_millis(25)).await;
                current_buffer.push_str(&chunk);
                handle.update(current_buffer.clone());
            }
        })
        .blink_color(BLINK_COLOR);

        #[cfg(feature = "markdown")]
        {
            text = text.stripe_color(ALT_COLOR); // Vertical stripes (blockquotes)
            text = text.bullet_color(ALT_COLOR); // List bullets and numbers
            text = text.code_color(ALT_COLOR); // Inline code formatting (`text`)
        }

        text.render().await?;
    }
}

// =============================================================================
//  MOCK DATA PROVIDERS
// =============================================================================

/// returns a simulated chunked Markdown payload for visual UI styling verification.
///
/// contains code blocks, tables, task lists, and blockquotes to validate palette application.
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
