use crate::render::{
    ansi,
    block::Block,
    widget::{DynamicWidget, Widget},
};
use crossterm::style::{Color, Stylize};
use std::{
    fmt::Display,
    future::Future,
    io,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::sync::{Notify, mpsc};

/// Represents operations that can be performed dynamically on the table state.
pub enum TableOp {
    /// Appends a new row to the table.
    AddRow(Vec<String>),
    /// Clears all existing rows from the table.
    ClearRows,
    /// Replaces all rows in the table with a new collection.
    SetRows(Vec<Vec<String>>),
}

/// A handle passed to asynchronous dynamic tasks to modify the table state concurrently.
pub struct TableHandle {
    /// Channel sender for dispatching table state update operations.
    sender: mpsc::UnboundedSender<TableOp>,
}

impl TableHandle {
    /// Appends a single row to the dynamic table.
    pub fn push_row(&self, row: &[impl Display]) {
        let row_vec = row.iter().map(|s| s.to_string()).collect();
        let _ = self.sender.send(TableOp::AddRow(row_vec));
    }

    /// Clears all rows currently present in the dynamic table.
    pub fn clear_rows(&self) {
        let _ = self.sender.send(TableOp::ClearRows);
    }

    /// Overwrites all table rows with a new collection of row items.
    pub fn set_rows<I, R, T>(&self, rows: I)
    where
        I: IntoIterator<Item = R>,
        R: IntoIterator<Item = T>,
        T: Display,
    {
        let rows_vec = rows
            .into_iter()
            .map(|r| r.into_iter().map(|item| item.to_string()).collect())
            .collect();
        let _ = self.sender.send(TableOp::SetRows(rows_vec));
    }
}

/// Type alias for asynchronous dynamic tasks that receive a [`TableHandle`] to update table content.
pub type TableTask =
    Box<dyn FnOnce(TableHandle) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// A customizable terminal interface table component supporting static data and asynchronous dynamic updates.
pub struct Table {
    /// Column header titles displayed at the top of the table.
    pub(crate) headers: Vec<String>,
    /// Thread-safe row data matrix stored as a shared synchronized vector.
    pub(crate) rows: Arc<Mutex<Vec<Vec<String>>>>,
    /// Color applied to the borders and separator lines of the table.
    pub(crate) border_color: Option<Color>,
    /// Color applied to the header text elements.
    pub(crate) header_color: Option<Color>,
    /// Optional asynchronous task for performing dynamic row updates.
    pub(crate) handler: Option<TableTask>,
    /// Notification signal triggered when the dynamic table rendering is finished.
    done_signal: Arc<Notify>,
}

impl Table {
    /// Creates a new [`Table`] wrapped in a wrapper block initialized with provided headers.
    pub fn new(headers: &[impl Display]) -> Block<Self> {
        Block::new(Self {
            headers: headers.iter().map(|s| s.to_string()).collect(),
            rows: Arc::new(Mutex::new(Vec::new())),
            border_color: Some(Color::DarkGrey),
            header_color: None,
            handler: None,
            done_signal: Arc::new(Notify::new()),
        })
    }
}

impl Block<Table> {
    /// Adds a single row of values to the table configuration.
    pub fn row(self, row: &[impl Display]) -> Self {
        if let Ok(mut lock) = self.inner.rows.lock() {
            lock.push(row.iter().map(|s| s.to_string()).collect());
        }
        self
    }

    /// Adds multiple rows to the table configuration from an iterable sequence.
    pub fn rows<I, R, T>(self, rows: I) -> Self
    where
        I: IntoIterator<Item = R>,
        R: IntoIterator<Item = T>,
        T: Display,
    {
        if let Ok(mut lock) = self.inner.rows.lock() {
            for row in rows {
                lock.push(row.into_iter().map(|item| item.to_string()).collect());
            }
        }
        self
    }

    /// Sets a custom color for the table structural borders.
    pub fn table_border_color(mut self, color: Color) -> Self {
        self.inner.border_color = Some(color);
        self
    }

    /// Sets a custom color for the column header text.
    pub fn header_color(mut self, color: Color) -> Self {
        self.inner.header_color = Some(color);
        self
    }

    /// Attaches an asynchronous background task handler for dynamic dynamic table state execution.
    pub fn handler<F, Fut>(mut self, task: F) -> Self
    where
        F: FnOnce(TableHandle) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.inner.handler = Some(Box::new(move |handle| Box::pin(task(handle))));
        self
    }

    /// Renders the table to standard output, managing asynchronous tasks if present.
    pub async fn render(mut self) -> io::Result<()> {
        let handler = self.inner.handler.take();

        if handler.is_none() {
            return self.render_static();
        }

        let task = handler.unwrap();
        let (tx, mut rx) = mpsc::unbounded_channel::<TableOp>();

        let done_notify = Arc::clone(&self.inner.done_signal);
        let done_notifier = Arc::clone(&done_notify);

        tokio::spawn(async move {
            let handle = TableHandle { sender: tx };
            task(handle).await;
        });

        let rows_writer = Arc::clone(&self.inner.rows);
        tokio::spawn(async move {
            while let Some(op) = rx.recv().await {
                if let Ok(mut lock) = rows_writer.lock() {
                    match op {
                        TableOp::AddRow(row) => lock.push(row),
                        TableOp::ClearRows => lock.clear(),
                        TableOp::SetRows(new_rows) => *lock = new_rows,
                    }
                }
            }
            done_notifier.notify_one();
        });

        self.render_dynamic_loop(done_notify).await
    }
}

impl Widget for Table {
    type Output = ();

    /// Formats and constructs the table layout lines adhering to standard layout and color constraints.
    fn render_content(&self, max_width: usize) -> Vec<String> {
        if self.headers.is_empty() {
            return Vec::new();
        }

        let col_count = self.headers.len();
        let mut widths = vec![0; col_count];

        // 1. calculate maximum width for each column
        for (i, h) in self.headers.iter().enumerate() {
            widths[i] = widths[i].max(ansi::visible_width(h));
        }

        let rows = self.rows.lock().unwrap();
        for row in rows.iter() {
            for (i, val) in row.iter().enumerate().take(col_count) {
                for sub_line in val.lines() {
                    widths[i] = widths[i].max(ansi::visible_width(sub_line));
                }
            }
        }

        let border_col = self.border_color.unwrap_or(Color::DarkGrey);

        let render_sep = |left: &str, mid: &str, right: &str, widths: &[usize]| -> String {
            let mut line = String::new();
            line.push_str(&left.with(border_col).to_string());
            for (i, w) in widths.iter().enumerate() {
                line.push_str(&"─".repeat(*w + 2).with(border_col).to_string());
                if i < col_count - 1 {
                    line.push_str(&mid.with(border_col).to_string());
                }
            }
            line.push_str(&right.with(border_col).to_string());
            line
        };

        let mut lines = Vec::new();

        // top border
        lines.push(render_sep("┌", "┬", "┐", &widths));

        // headers
        let mut header_line = String::new();
        header_line.push_str(&"│".with(border_col).to_string());
        for (i, h) in self.headers.iter().enumerate() {
            let vis_w = ansi::visible_width(h);
            let pad_right = widths[i].saturating_sub(vis_w);

            let styled_h = if let Some(col) = self.header_color {
                h.as_str().bold().with(col).to_string()
            } else {
                h.as_str().bold().to_string()
            };

            header_line.push_str(&format!(" {}{} ", styled_h, " ".repeat(pad_right)));
            header_line.push_str(&"│".with(border_col).to_string());
        }
        lines.push(header_line);

        // header separator
        lines.push(render_sep("├", "┼", "┤", &widths));

        // data rows with support for persistent color bleeding across multi-line cell values
        let rows_count = rows.len();
        for (row_idx, row) in rows.iter().enumerate() {
            // split cell contents into separate lines and prepare state for color propagation
            let cell_lines: Vec<Vec<&str>> = (0..col_count)
                .map(|i| {
                    let val = row.get(i).map(|s| s.as_str()).unwrap_or("");
                    val.lines().collect()
                })
                .collect();

            let row_height = cell_lines.iter().map(|l| l.len()).max().unwrap_or(1);

            // store active color prefixes for each dynamic column
            let mut col_color_prefixes = vec![String::new(); col_count];

            for sub_row in 0..row_height {
                let mut row_line = String::new();
                row_line.push_str(&"│".with(border_col).to_string());

                for (i, w) in widths.iter().enumerate() {
                    let raw_sub_val = cell_lines[i].get(sub_row).copied().unwrap_or("");

                    // apply active color prefix preserved from the preceding cell line
                    let sub_val_with_color = format!("{}{}", col_color_prefixes[i], raw_sub_val);
                    let vis_w = ansi::visible_width(&sub_val_with_color);
                    let pad_right = w.saturating_sub(vis_w);

                    row_line.push_str(&format!(
                        " {}{} ",
                        sub_val_with_color,
                        " ".repeat(pad_right)
                    ));

                    // preserve active text styling state for subsequent multiline cell sub-rows
                    if let Some(active_col) = ansi::get_active_text_color(&sub_val_with_color) {
                        col_color_prefixes[i] = active_col;
                    }

                    // reset character styling at structural cell borders
                    row_line.push_str(&"│".with(border_col).to_string());
                }
                lines.push(row_line);
            }

            if row_idx + 1 < rows_count {
                lines.push(render_sep("├", "┼", "┤", &widths));
            }
        }

        // bottom border
        lines.push(render_sep("└", "┴", "┘", &widths));

        // crop lines if total width exceeds specified max terminal width
        lines
            .into_iter()
            .map(|l| {
                if ansi::visible_width(&l) > max_width {
                    l.chars().take(max_width).collect()
                } else {
                    l
                }
            })
            .collect()
    }
}

impl DynamicWidget for Table {
    /// Returns the handle to the notification signal triggered upon operation completion.
    fn completion_signal(&self) -> Arc<Notify> {
        Arc::clone(&self.done_signal)
    }

    /// Consumes the widget to return its target dynamic execution output type.
    fn extract_output(self) -> Self::Output {
        ()
    }
}
