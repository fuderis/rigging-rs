use rigging::{Commands, pkg_meta};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    Commands::new()
        .meta(pkg_meta!())
        .args(&["--host=127.0.0.1"])
        // === Server Management Commands ===
        .group("server", "Server management commands")
        .cmd(
            "server start --port=8080",
            "Start the server",
            |ctx| async move {
                let host: String = ctx.get("host")?;
                let port: u16 = ctx.get("port")?;

                println!("Starting server on {}:{}...", host, port);
                Ok(())
            },
        )
        .cmd(
            "server stop --force=false",
            "Stop the server",
            |ctx| async move {
                let force: bool = ctx.get_bool("force");

                if force {
                    println!("Forcefully terminating server...");
                } else {
                    println!("Initiating graceful server shutdown...");
                }
                Ok(())
            },
        )
        .cmd("server status", "Check server status", |_| async move {
            println!("Server status: running.");
            Ok(())
        })
        .cmd(
            "server exec {command..}",
            "Execute a command on the server",
            |ctx| async move {
                let cmd = ctx.get_str("command").unwrap_or_default();
                println!("Executing command: {}", cmd);
                Ok(())
            },
        )
        .run()
        .await
}
