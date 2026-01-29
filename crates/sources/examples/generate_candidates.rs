//! Example: Generate candidates for a user
//!
//! Run with: cargo run --package sources --example generate_candidates
//!
//! This example shows how to:
//! 1. Load the MovieLens dataset
//! 2. Build user context
//! 3. Generate Thunder (collaborative) candidates
//! 4. Generate Phoenix (discovery) candidates
//! 5. Display the results

use data_loader::DataIndex;
use sources::{user_context::build_user_context, CandidateSource, PhoenixSource, ThunderSource};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("=== ReelRecs Candidate Generation Example ===\n");

    // Load dataset
    println!("Loading MovieLens dataset...");
    let start = Instant::now();
    let data_dir = Path::new("data/ml-1m");
    let data_index = Arc::new(DataIndex::load_from_files(data_dir)?);
    println!("Loaded dataset in {:?}\n", start.elapsed());

    // Choose a test user (user 1)
    let user_id = 1;
    let user = data_index.get_user(user_id).expect("User not found");
    println!("Target User: {}", user_id);
    println!("  Gender: {:?}", user.gender);
    println!("  Age: {:?}", user.age);
    println!("  Occupation: {:?}\n", user.occupation);

    // Build user context
    println!("Building user context...");
    let start = Instant::now();
    let context = build_user_context(&data_index, user_id)?;
    println!("Built context in {:?}", start.elapsed());
    println!("  Watched movies: {}", context.watched_movies.len());
    println!("  Highly rated: {}", context.highly_rated_movies.len());
    println!("  Avg rating: {:.2}", context.avg_rating);
    println!("  Genre preferences: {} genres", context.genre_preferences.len());
    if let Some(era) = context.preferred_era {
        println!("  Preferred era: {}", era);
    }
    println!();

    // Generate Thunder candidates
    println!("Generating Thunder (collaborative) candidates...");
    let thunder = ThunderSource::new(data_index.clone());
    let start = Instant::now();
    let thunder_candidates = thunder.get_candidates(&context, 300);
    let thunder_time = start.elapsed();
    println!(
        "Generated {} Thunder candidates in {:?}",
        thunder_candidates.len(),
        thunder_time
    );

    // Show top 5 Thunder candidates
    println!("\nTop 5 Thunder Candidates:");
    for (i, candidate) in thunder_candidates.iter().take(5).enumerate() {
        if let Some(movie) = data_index.get_movie(candidate.movie_id) {
            println!(
                "  {}. {} (Score: {:.3})",
                i + 1,
                movie.title,
                candidate.base_score
            );
            if let Some(count) = candidate.metadata.similar_users_count {
                println!("     - Liked by {} similar users", count);
            }
        }
    }

    // Generate Phoenix candidates
    println!("\nGenerating Phoenix (discovery) candidates...");
    let phoenix = PhoenixSource::new(data_index.clone());
    let start = Instant::now();
    let phoenix_candidates = phoenix.get_candidates(&context, 200);
    let phoenix_time = start.elapsed();
    println!(
        "Generated {} Phoenix candidates in {:?}",
        phoenix_candidates.len(),
        phoenix_time
    );

    // Show top 5 Phoenix candidates
    println!("\nTop 5 Phoenix Candidates:");
    for (i, candidate) in phoenix_candidates.iter().take(5).enumerate() {
        if let Some(movie) = data_index.get_movie(candidate.movie_id) {
            println!(
                "  {}. {} (Score: {:.3})",
                i + 1,
                movie.title,
                candidate.base_score
            );
            if !candidate.metadata.matched_genres.is_empty() {
                println!("     - Matched genres: {:?}", candidate.metadata.matched_genres);
            }
            if candidate.metadata.from_popularity {
                println!("     - From popularity-based discovery");
            }
            if candidate.metadata.from_temporal {
                println!("     - From temporal discovery");
            }
        }
    }

    // Summary
    println!("\n=== Summary ===");
    println!(
        "Total candidates: {}",
        thunder_candidates.len() + phoenix_candidates.len()
    );
    println!("Thunder time: {:?}", thunder_time);
    println!("Phoenix time: {:?}", phoenix_time);

    // Check for overlap
    let thunder_ids: std::collections::HashSet<_> = thunder_candidates
        .iter()
        .map(|c| c.movie_id)
        .collect();
    let phoenix_ids: std::collections::HashSet<_> = phoenix_candidates
        .iter()
        .map(|c| c.movie_id)
        .collect();
    let overlap = thunder_ids.intersection(&phoenix_ids).count();
    println!("Overlap between sources: {} movies", overlap);

    // Performance check
    println!("\nPerformance targets:");
    println!(
        "  Thunder: {:?} (target: <5ms) {}",
        thunder_time,
        if thunder_time.as_millis() < 5 {
            "✓"
        } else {
            "✗"
        }
    );
    println!(
        "  Phoenix: {:?} (target: <3ms) {}",
        phoenix_time,
        if phoenix_time.as_millis() < 3 {
            "✓"
        } else {
            "✗"
        }
    );

    Ok(())
}
