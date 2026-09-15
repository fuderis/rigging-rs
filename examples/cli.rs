//! Demonstration binary for the `rigging` TUI framework showcasing nested widget
//! execution via `Context::push_sub_widget`, interactive confirmation prompts,
//! and streaming output.

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

const BRAND_COLOR: Color = Color::Rgb {
    r: 255,
    g: 85,
    b: 65,
};

const BG_COLOR: Color = Color::Rgb {
    r: 15,
    g: 20,
    b: 35,
};

const ALT_COLOR: Color = Color::Rgb {
    r: 140,
    g: 120,
    b: 81,
};

const BLINK_COLOR: Color = Color::Rgb {
    r: 20,
    g: 26,
    b: 42,
};

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
