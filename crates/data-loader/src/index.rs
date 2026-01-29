//! DataIndex building and indexing logic.
//!
//! This module will build the DataIndex from parsed data:
//! - Create primary indices (users, movies, ratings)
//! - Build secondary indices (genre_index, year_index)
//! - Compute aggregate statistics (movie stats)
//!
//! Rust concepts you'll learn:
//! - Building HashMaps and indices
//! - Using Rayon for parallel processing
//! - Iterator methods (map, filter, fold, etc.)
//! - Entry API for HashMap
//! - Borrowing and ownership in complex data structures

use crate::error::{DataLoadError, Result};
use crate::types::*;
use crate::parser;
use std::path::Path;
use rayon::prelude::*;

impl DataIndex {
    /// Load the entire MovieLens dataset from a directory
    ///
    /// This is the main entry point for loading data.
    ///
    /// Steps:
    /// 1. Parse all three files (users, movies, ratings)
    /// 2. Build primary indices
    /// 3. Build secondary indices (genre, year)
    /// 4. Compute movie statistics
    /// 5. Validate data integrity
    ///
    pub fn load_from_files(data_dir: &Path) -> Result<Self> {
        println!("Loading MovieLens dataset from {:?}", data_dir);

        // 1. Construct paths to the three .dat files
        let users_path = data_dir.join("users.dat");
        let movies_path = data_dir.join("movies.dat");
        let ratings_path = data_dir.join("ratings.dat");

        // 2. Parse all three files IN PARALLEL using Rayon
        // Rayon's `join` runs two closures in parallel
        // We nest joins to get three-way parallelism
        let ((users, movies), ratings) = rayon::join(
            || {
                // Parse users and movies in parallel
                rayon::join(
                    || parser::parse_users(&users_path),
                    || parser::parse_movies(&movies_path),
                )
            },
            || parser::parse_ratings(&ratings_path),
        );

        // Handle errors from parallel parsing
        // The ? operator works because all return Result<Vec<T>>
        let users = users?;
        let movies = movies?;
        let ratings = ratings?;

        println!(
            "Loaded {} users, {} movies, {} ratings",
            users.len(),
            movies.len(),
            ratings.len()
        );

        // 3. Build the DataIndex
        let mut index = DataIndex::new();

        // Insert all users
        for user in users {
            index.insert_user(user);
        }

        // Insert all movies
        for movie in movies {
            index.insert_movie(movie);
        }

        // Insert all ratings (this also populates user_ratings and movie_ratings)
        for rating in ratings {
            index.insert_rating(rating);
        }

        // 4. Build secondary indices (genre and year lookups)
        index.build_secondary_indices();

        // 5. Compute movie statistics in parallel
        index.compute_movie_stats();

        // 6. Validate data integrity
        index.validate()?;

        println!("DataIndex successfully built and validated!");
        Ok(index)
    }

    /// Build secondary indices after primary data is loaded
    ///
    /// This creates the genre_index and year_index for fast lookups
    ///
    pub fn build_secondary_indices(&mut self) {
        // Iterate through movies and populate:
        // - genre_index: Map each genre to list of movie IDs
        // - year_index: Map each year to list of movie IDs
        for (movie_id, movie) in &self.movies {
            // Index by genres
            for &genre in &movie.genres {
                self.genre_index
                    .entry(genre)
                    .or_insert_with(Vec::new)
                    .push(*movie_id);
            }

            // Index by release year (only if year is known)
            if let Some(year) = movie.year {
                self.year_index
                    .entry(year)
                    .or_insert_with(Vec::new)
                    .push(*movie_id);
            }
        }
    }

    /// Compute aggregate statistics for all movies
    ///
    /// For each movie, calculate:
    /// - Average rating
    /// - Rating count
    /// - Popularity score (you can use a formula like: avg_rating * log(rating_count + 1))
    ///
    /// Hint: Use rayon's par_iter() for parallel computation
    pub fn compute_movie_stats(&mut self) {
        // For each movie:
        // 1. Get all ratings for that movie
        // 2. Calculate average rating
        // 3. Count number of ratings
        // 4. Compute popularity score
        // 5. Store in movie_stats HashMap
        let movie_stats = self.movie_ratings.par_iter().map(|(&movie_id, ratings)| {
            let rating_count = ratings.len() as u32;
            let avg_rating = if rating_count > 0 {
                let total: f32 = ratings.iter().map(|r| r.rating).sum();
                total / rating_count as f32
            } else {
                0.0
            };
            let popularity_score = compute_popularity_score(avg_rating, rating_count);

            (movie_id, MovieStats {
                avg_rating: avg_rating,
                rating_count,
                popularity_score,
            })
        }).collect();
        self.movie_stats = movie_stats;         
    }

    /// Validate data integrity
    ///
    /// Check that:
    /// - All rating.user_id references exist in users
    /// - All rating.movie_id references exist in movies
    /// - Ratings are in valid range (1.0 - 5.0)
    ///
    /// Returns Ok(()) if valid, Err if any issues found
    ///
    pub fn validate(&self) -> Result<()> {
        // Iterate through all ratings and verify:
        // 1. User exists
        // 2. Movie exists
        // 3. Rating value is valid (1.0 - 5.0)
        for ratings in self.user_ratings.values() {
            for rating in ratings {
                if !self.users.contains_key(&rating.user_id) {
                    return Err(DataLoadError::MissingReference {
                        entity: "User".to_string(),
                        id: rating.user_id,
                    });
                }
                if !self.movies.contains_key(&rating.movie_id) {
                    return Err(DataLoadError::MissingReference {
                        entity: "Movie".to_string(),
                        id: rating.movie_id,
                    });
                }
                if rating.rating < 1.0 || rating.rating > 5.0 {
                    return Err(DataLoadError::InvalidValue 
                        { field: "rating".to_string(), 
                        value: rating.rating.to_string() 
                    });
                }
            }
        }
        Ok(())  
    }
}

/// Helper function to compute popularity score
///
/// One possible formula: avg_rating * log(rating_count + 1)
/// This rewards both high ratings and many ratings
fn compute_popularity_score(avg_rating: f32, rating_count: u32) -> f32 {
    avg_rating * (rating_count as f32 + 1.0).ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_popularity_score() {
        // High rating with few ratings
        let score1 = compute_popularity_score(4.5, 10);

        // Medium rating with many ratings
        let score2 = compute_popularity_score(3.5, 1000);

        // Should balance both factors
        assert!(score1 > 0.0);
        assert!(score2 > 0.0);
    }

    #[test]
    fn test_load_dataset() {
        // This test requires the actual dataset files
        // Place ml-1m data in ../../../data/ml-1m/
        let data_dir = Path::new("../../../data/ml-1m");

        if data_dir.exists() {
            let index = DataIndex::load_from_files(data_dir).unwrap();
            let (users, movies, ratings) = index.counts();

            // MovieLens 1M expected counts
            assert_eq!(users, 6040);
            assert_eq!(movies, 3883);
            assert_eq!(ratings, 1000209);
        }
    }
}
