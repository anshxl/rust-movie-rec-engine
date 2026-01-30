//! Simple test harness for the recommendation orchestrator.
//!
//! This binary lets you test the end-to-end pipeline by requesting
//! recommendations for a specific user.

use std::sync::Arc;
use std::path::Path;

use anyhow::Result;
use tracing::info;
use tracing_subscriber;

use data_loader::DataIndex;
use server::RecommendationOrchestrator;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,server=debug,sources=debug,pipeline=debug")
        .init();

    info!("Starting ReelRecs server test harness");

    // TODO: Load the data index
    // Hint: DataIndex::load_from_files("data/ml-1m")?
    info!("Loading data index...");
    let path = Path::new("data/ml-1m");
    let data_index = Arc::new(DataIndex::load_from_files(path)?);
    info!("Data index loaded successfully");

    // Hint: Make sure Python ML service is running on localhost:50051
    // Hint: RecommendationOrchestrator::new(data_index, "http://localhost:50051").await?
    info!("Connecting to ML service...");
    let orchestrator = RecommendationOrchestrator::new(
        data_index,
        "http://localhost:50051",
    ).await?;
    info!("Connected to ML service");

    // TODO: Test with a sample user
    // Hint: Try user_id = 1 or any valid user from the dataset
    let user_id = 1;
    let limit = 20;

    info!("Getting recommendations for user {} (limit: {})", user_id, limit);
    let recommendations = orchestrator.get_recommendations(user_id, limit).await?;

    // TODO: Print the recommendations
    // Hint: Iterate and print each recommendation with title, score, source
    info!("Received {} recommendations:", recommendations.len());
    for (i, rec) in recommendations.iter().enumerate() {
        info!(
            "{}. {} ({}) - Score: {:.3} [{}]",
             i + 1,
             rec.title,
             rec.year.map(|y| y.to_string()).unwrap_or("????".to_string()),
             rec.score,
             format!("{:?}", rec.source)
        );
        info!("   Genres: {}", rec.genres.join(", "));
        info!("   {}", rec.explanation);
    }
    info!("Test harness template ready!");

    Ok(())
}
