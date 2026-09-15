use std::time::Duration;
use tokio::time::sleep;

use rigging::widgets::{ConfirmPrompt, Confirmation, ProgressBar, SelectMenu};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let confirm_res: Option<Confirmation> =
        ConfirmPrompt::new("Should I continue installing packages?")
            .default(Confirmation::Yes)
            .render()
            .await?;

    match confirm_res {
        Some(Confirmation::Yes) => {
            println!("User agreed to the execution.");
        }
        Some(Confirmation::No) => {
            println!("User rejected the action.");
            return Ok(());
        }
        None => {
            println!("Action has been canceled (Esc).");
            return Ok(());
        }
    }

    let packages = vec!["Fast-Engine v2.1", "Core-Utils v0.9", "Full-Suite v3.0"];

    let selected_idx: Option<usize> =
        SelectMenu::new("Select the package to download:", packages.clone())
            .render()
            .await?;

    let selected_pkg = match selected_idx {
        Some(idx) => packages[idx],
        None => {
            println!("Selection is canceled (Esc).");
            return Ok(());
        }
    };

    ProgressBar::new_with_label(0, 100, format!("Downloading {selected_pkg}..."))
        .handler(move |mut ctx| async move {
            for i in 1..=100 {
                sleep(Duration::from_millis(20)).await;
                ctx.current = i;
                ctx.notify();
            }
            ctx.current = 100;
            ctx.label.replace("Download is completed!".into());
            ctx.finish();
        })
        .max_width(80)
        .render()
        .await?;

    Ok(())
}
