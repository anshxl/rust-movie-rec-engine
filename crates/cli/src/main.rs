use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use colored::Colorize;
use data_loader::{DataIndex, MovieId, UserId, Genre};
use server::{MovieRecommendation, RecommendationOrchestrator};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// ReelRecs - Movie Recommendation Engine
#[derive(Parser)]
#[command(name = "reel-recs")]
#[command(about = "Movie recommendation engine using collaborative filtering", long_about = None)]
struct Cli {
    /// Path to MovieLens dataset directory
    #[arg(short, long, default_value = "data/ml-1m")]
    data_dir: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get movie recommendations for a user
    Recommend {
        /// User ID to get recommendations for
        #[arg(long)]
        user_id: UserId,

        /// Number of recommendations to return
        #[arg(long, default_value = "20")]
        limit: usize,

        /// Show detailed explanation for each recommendation
        #[arg(long)]
        explain: bool,
    },

    /// Show user profile and watch history
    User {
        /// User ID to display
        #[arg(long)]
        user_id: UserId,
    },

    /// Search for movies by title
    Search {
        /// Movie title to search for (case-insensitive substring match)
        #[arg(long)]
        title: String,
    },

    /// Run benchmark to test performance
    Benchmark {
        /// Number of requests to make
        #[arg(long, default_value = "100")]
        requests: usize,

        /// Number of concurrent requests
        #[arg(long, default_value = "10")]
        concurrent: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Load data index (this may take a moment)
    println!("Loading MovieLens dataset from {}...", cli.data_dir.display());
    let start = Instant::now();
    let data_index = Arc::new(
        DataIndex::load_from_files(&cli.data_dir)
            .context("Failed to load MovieLens dataset")?,
    );
    println!(
        "{} Loaded dataset in {:?}",
        "✓".green(),
        start.elapsed()
    );

    // Dispatch to appropriate command handler
    match cli.command {
        Commands::Recommend {
            user_id,
            limit,
            explain,
        } => handle_recommend(data_index, user_id, limit, explain).await?,
        Commands::User { user_id } => handle_user(data_index, user_id)?,
        Commands::Search { title } => handle_search(data_index, title)?,
        Commands::Benchmark {
            requests,
            concurrent,
        } => handle_benchmark(data_index, requests, concurrent).await?,
    }

    Ok(())
}

/// Handle the 'recommend' command
async fn handle_recommend(
    data_index: Arc<DataIndex>,
    user_id: UserId,
    limit: usize,
    explain: bool,
) -> Result<()> {
    // Check if user exists
    let _user = data_index.get_user(user_id)
        .ok_or_else(|| anyhow!("User {} not found", user_id))?;

    // Create a RecommendationOrchestrator
    let orchestrator = RecommendationOrchestrator::new(data_index.clone(), "http://localhost:50051").await?;

    // Call orchestrator.get_recommendations(user_id, limit).await
    let recommendations = orchestrator.get_recommendations(user_id, limit).await?;

    // Format and print the results
    print_recommendations(&recommendations, explain);

    // If explain is true, show more detailed information
    if explain {
        for rec in &recommendations {
            println!("Recommendation for movie ID {}: {:?}", rec.movie_id, rec.explanation);
        }
    }
    Ok(())
}

/// Handle the 'user' command
fn handle_user(data_index: Arc<DataIndex>, user_id: UserId) -> Result<()> {
    // TODO: Implement this function
    // Hints:
    // 1. Get the user from data_index.get_user(user_id)
    let user = data_index.get_user(user_id)
        .ok_or_else(|| anyhow!("User {} not found", user_id))?;

    // 2. Get the user's ratings from data_index.get_user_ratings(user_id)
    let ratings = data_index.get_user_ratings(user_id);

    // 3. Display user information (age, gender, occupation)
    print!("{}", format!("User ID: {}\n", user_id).bold().blue());
    print!("{}Age: {:?}\n", "• ".green(), user.age);
    print!("{}Gender: {:?}\n", "• ".green(), user.gender);
    print!("{}Occupation: {:?}\n", "• ".green(), user.occupation);

    // 4. Show statistics (number of ratings, average rating)
    let num_ratings = ratings.len();
    let avg_rating = if num_ratings > 0 {
        let total: f32 = ratings.iter().map(|r| r.rating as f32).sum();
        total / num_ratings as f32
    } else {
        0.0
    };
    print!("{}Number of ratings: {}\n", "• ".cyan(), num_ratings);
    print!("{}Average rating: {:.2}\n", "• ".cyan(), avg_rating);

    // 5. Show top rated movies
    let mut top_rated: Vec<_> = ratings.iter().collect();
    top_rated.sort_by(|a, b| {
        b.rating.
            partial_cmp(&a.rating)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    print!("Top rated movies:\n");
    for rating in top_rated.iter().take(5) {
        if let Some(movie) = data_index.get_movie(rating.movie_id) {
            print!("  - {} (Rating: {})\n", movie.title, rating.rating);
        }
    }

    // 6. Show genre preferences (what genres they rate highly)
    let mut genre_ratings: std::collections::HashMap<Genre, (u32, u32)> = std::collections::HashMap::new();
    for rating in ratings {
        if let Some(movie) = data_index.get_movie(rating.movie_id) {
            for genre in &movie.genres {
                let entry = genre_ratings.entry(genre.clone()).or_insert((0, 0));
                entry.0 += rating.rating as u32;
                entry.1 += 1;
            }
        }
    }
    print!("Genre preferences:\n");
    for (genre, (total_rating, count)) in genre_ratings {
        let avg = total_rating as f32 / count as f32;
        print!("  - {:?}: Average Rating: {:.2} ({} ratings)\n", genre, avg, count);
    }
    Ok(())
}

/// Handle the 'search' command
fn handle_search(data_index: Arc<DataIndex>, title: String) -> Result<()> {
    // TODO: Implement this function
    // Hints:
    // Iterate through all movies in data_index
    let mut movie_ids = data_index.get_all_movie_ids();
    let title_lower = title.to_lowercase();
    let mut matches: Vec<(MovieId, String, Vec<Genre>, f32, usize, u32)> = Vec::new();

    for movie_id in movie_ids.drain(..) {
        // Check if the title contains the search string (case-insensitive)
        if let Some(movie) = data_index.get_movie(movie_id) {
            let movie_title_lower = movie.title.to_lowercase();

            // Get movie stats
            let rating_stats = data_index.get_movie_stats(movie_id);
            let avg_rating = rating_stats.map(|s| s.avg_rating).unwrap_or(0.0);
            let rating_count = rating_stats.map(|s| s.rating_count).unwrap_or(0);

            if movie_title_lower == title_lower {
                // Exact match
                matches.push((
                    movie_id, 
                    movie.title.clone(), 
                    movie.genres.clone(), 
                    avg_rating, 
                    0,
                    rating_count,
                ));
            } else if movie_title_lower.contains(&title_lower) {
                // Substring match
                matches.push((
                    movie_id, 
                    movie.title.clone(), 
                    movie.genres.clone(), 
                    avg_rating, 
                    1,
                    rating_count,
                ));
            }
        }
    }
    // Sort by relevance (exact match first, then contains)
    matches.sort_by(|a,b| {
        a.4
            .cmp(&b.4)
            .then_with(|| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal))   
    });
    // Display top 20 results with movie ID, title, genres, and rating stats
    println!("{}", format!("Search results for '{}':", title).bold().blue());
    for (movie_id, movie_title, genres, avg_rating, _, rating_count) in matches.iter().take(20) {
        let genres_str = genres
            .iter()
            .map(|g| format!("{:?}", g))
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "{}: {} [{}] avg {:.2} ({} ratings)",
            movie_id, movie_title, genres_str, avg_rating, rating_count
        );
    }
    Ok(())
}

/// Handle the 'benchmark' command
async fn handle_benchmark(
    data_index: Arc<DataIndex>,
    requests: usize,
    _concurrent: usize,
) -> Result<()> {
    // Create a RecommendationOrchestrator
    let orchestrator = RecommendationOrchestrator::new(data_index.clone(), "http://localhost:50051").await?;

    // Generate a set of random user IDs between 1 and 6040
    let user_ids: Vec<UserId> = (0..requests)
        .map(|_| {
            let user_id = rand::random::<u32>() % 6040 + 1;
            user_id as UserId
        })
        .collect();

    // Use tokio::spawn to make concurrent requests
    let mut handles = vec![];
    for user in user_ids {
        let orchestrator = orchestrator.clone();
        let handle = tokio::spawn(async move {
            let start = Instant::now();
            orchestrator.get_recommendations(user, 20).await?;
            Ok::<_, anyhow::Error>(start.elapsed())
        });
        handles.push(handle);
    }
    // Wait for all tasks to complete and collect timings
    let mut timings = vec![];
    for handle in handles {
        let elapsed = handle.await??;
        timings.push(elapsed);
    }
    // 5. Calculate and display statistics:
    //    - Total time
    //    - Average latency
    //    - P50, P95, P99 latencies
    //    - Throughput (requests/second)
    let total_time: std::time::Duration = timings.iter().sum();
    let avg_latency = total_time / (timings.len() as u32);
    timings.sort();
    let p50 = timings[timings.len() / 2];
    let p95 = timings[(timings.len() as f32 * 0.95) as usize];
    let p99 = timings[(timings.len() as f32 * 0.99) as usize];
    let throughput = requests as f32 / total_time.as_secs_f32();

    println!("Benchmark results:");
    println!("Total time: {:?}", total_time);
    println!("Average latency: {:?}", avg_latency);
    println!("P50 latency: {:?}", p50);
    println!("P95 latency: {:?}", p95);
    println!("P99 latency: {:?}", p99);
    println!("Throughput: {:.2} requests/second", throughput);

    Ok(())
}

/// Helper function to format and print recommendations
fn print_recommendations(recommendations: &[MovieRecommendation], explain: bool) {
    // 1. Print a nice header
    print!("{}", "Movie Recommendations:\n".bold().blue());
    // 2. For each recommendation, print:
    //    - Rank number
    //    - Title (with year if available)
    //    - Genres
    //    - Score
    //    - If explain=true, show the source and explanation
    for rec in recommendations.iter().enumerate() {
        let rank = rec.0 + 1;
        let movie = &rec.1;
        let genres = movie.genres
            .iter()
            .map(|g| format!("{:?}", g))
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "{}. {} ({}) [{}] - Score: {:.2}",
            rank.to_string().green(),
            movie.title,
            movie.year.unwrap_or(0),
            genres,
            movie.score
        );
        if explain {
            println!("   Explanation: {}", movie.explanation);
        }
    }
}