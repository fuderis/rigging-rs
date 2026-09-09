use std::time::Duration;
use tokio::time::sleep;

use rigging::widgets::ProgressBar;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ProgressBar::new(0, 100)
        .label("Initializing...")
        .handler(|handle| async move {
            for i in 1..=100 {
                sleep(Duration::from_millis(20)).await;

                handle.update_with_label(i, format!("Downloading..."));
            }
            handle.update_with_label(100, "Downloaded!");
        })
        .max_width(80)
        .render()
        .await?;

    Ok(())
}
