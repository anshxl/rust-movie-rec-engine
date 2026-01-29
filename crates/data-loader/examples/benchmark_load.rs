use data_loader::DataIndex;
use std::path::Path;
use std::time::Instant;

fn main() {
    let data_dir = Path::new("data/ml-1m");

    println!("Loading MovieLens 1M dataset...\n");

    let start = Instant::now();
    let index = DataIndex::load_from_files(data_dir)
        .expect("Failed to load dataset");
    let elapsed = start.elapsed();

    let (users, movies, ratings) = index.counts();

    println!("\n=== Load Complete ===");
    println!("Time taken: {:?}", elapsed);
    println!("Users: {}", users);
    println!("Movies: {}", movies);
    println!("Ratings: {}", ratings);
    println!("\nPerformance: {:.0} ratings/second",
             ratings as f64 / elapsed.as_secs_f64());
}
