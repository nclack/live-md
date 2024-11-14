use anyhow::Result;
use live_md::{
    config::Config,
    server::start_server,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Use default configuration
    let config = Config::default();
    
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&config.output_dir)?;

    // Start server with default configuration
    start_server(config).await?;

    Ok(())
}
