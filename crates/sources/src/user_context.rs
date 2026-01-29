//! Helper functions to build UserContext from DataIndex
//!
//! This module provides utilities to aggregate user information
//! into a UserContext struct for efficient candidate generation.

use crate::types::UserContext;
use anyhow::{anyhow, Result};
use data_loader::{DataIndex, Genre, UserId};
use std::collections::HashMap;

/// Build a UserContext from DataIndex for a given user
///
/// This function aggregates all the user information we need:
/// - Movies watched (all rated movies)
/// - Highly rated movies (rating >= 4.0)
/// - Genre preferences (average rating per genre)
/// - Preferred era (median year of highly-rated movies)
/// - Average rating given by user
///
/// ## Learning Note
/// This demonstrates the "context builder" pattern in Rust:
/// - Gather data once upfront
/// - Avoid repeated DataIndex queries during candidate generation
/// - Use HashSet for O(1) lookups
pub fn build_user_context(data_index: &DataIndex, user_id: UserId) -> Result<UserContext> {
    // Verify user exists
    let _user = data_index
        .get_user(user_id)
        .ok_or_else(|| anyhow!("User {} not found", user_id))?;

    let mut context = UserContext::new(user_id);

    // Get all user's ratings
    let ratings = data_index.get_user_ratings(user_id);

    if ratings.is_empty() {
        // Return early for users with no ratings
        return Ok(context);
    }

    // Compute average rating
    let total: f32 = ratings.iter().map(|r| r.rating).sum();
    context.avg_rating = total / ratings.len() as f32;

    // Build watched movies set and highly rated movies list
    for rating in ratings {
        context.watched_movies.insert(rating.movie_id);

        if rating.rating >= 4.0 {
            context.highly_rated_movies.push(rating.movie_id);
        }
    }

    // Compute genre preferences
    context.genre_preferences = compute_genre_preferences(data_index, ratings);

    // Compute preferred era
    context.preferred_era = compute_preferred_era(data_index, &context.highly_rated_movies);

    Ok(context)
}

/// Compute average rating per genre for a user
fn compute_genre_preferences(
    data_index: &DataIndex,
    ratings: &[data_loader::Rating],
) -> HashMap<Genre, f32> {
    // Steps:
    // 1. Create HashMap<Genre, (f32, u32)> to track (sum, count)
    // 2. Iterate through ratings
    // 3. For each rating, get the movie
    // 4. For each genre in movie, update the (sum, count)
    // 5. Convert to HashMap<Genre, f32> with averages
    let mut genre_stats: HashMap<Genre, (f32, u32)> = HashMap::new();
    for rating in ratings {
        if let Some(movie) = data_index.get_movie(rating.movie_id) {
            for genre in &movie.genres {
                let entry = genre_stats.entry(*genre).or_insert((0.0, 0));
                entry.0 += rating.rating;
                entry.1 += 1;
            }
        }
    }

    // Convert to averages
    genre_stats.into_iter()
        .map(|(genre, (sum, count))| (genre, sum / count as f32))
        .collect()
}

/// Compute user's preferred movie era (median year of highly-rated movies)
fn compute_preferred_era(
    data_index: &DataIndex,
    highly_rated_movies: &[data_loader::MovieId],
) -> Option<u16> {
    // Steps:
    // 1. Collect years: Vec<u16>
    // 2. Filter out movies with no year
    // 3. Sort the years
    // 4. Return middle element
    let mut years: Vec<u16> = highly_rated_movies
        .iter()
        .filter_map(|&movie_id| {
            data_index.get_movie(movie_id)?.year
        })
        .collect();

    if years.is_empty() {
        return None;
    }
    years.sort_unstable();
    Some(years[years.len() / 2])    
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_loader::{AgeGroup, Gender, Movie, Occupation, Rating, User};

    fn create_test_index() -> DataIndex {
        let mut index = DataIndex::new();

        // Add a test user
        index.insert_user(User {
            id: 1,
            gender: Gender::Male,
            age: AgeGroup::Age25To34,
            occupation: Occupation::Programmer,
            zipcode: "12345".to_string(),
        });

        // Add test movies
        index.insert_movie(Movie {
            id: 1,
            title: "Action Movie (2000)".to_string(),
            year: Some(2000),
            genres: vec![Genre::Action, Genre::Adventure],
        });

        index.insert_movie(Movie {
            id: 2,
            title: "Drama Movie (1995)".to_string(),
            year: Some(1995),
            genres: vec![Genre::Drama],
        });

        index.insert_movie(Movie {
            id: 3,
            title: "Action Movie 2 (2005)".to_string(),
            year: Some(2005),
            genres: vec![Genre::Action, Genre::SciFi],
        });

        // Add test ratings
        index.insert_rating(Rating {
            user_id: 1,
            movie_id: 1,
            rating: 5.0,
            timestamp: 1000000,
        });

        index.insert_rating(Rating {
            user_id: 1,
            movie_id: 2,
            rating: 3.0,
            timestamp: 1000001,
        });

        index.insert_rating(Rating {
            user_id: 1,
            movie_id: 3,
            rating: 4.5,
            timestamp: 1000002,
        });

        index
    }

    #[test]
    fn test_build_user_context_basic() {
        let index = create_test_index();
        let context = build_user_context(&index, 1).unwrap();

        assert_eq!(context.user_id, 1);
        assert_eq!(context.watched_movies.len(), 3);
        assert!(context.watched_movies.contains(&1));
        assert!(context.watched_movies.contains(&2));
        assert!(context.watched_movies.contains(&3));
    }

    #[test]
    fn test_build_user_context_highly_rated() {
        let index = create_test_index();
        let context = build_user_context(&index, 1).unwrap();

        // Movies 1 (5.0) and 3 (4.5) should be highly rated
        assert_eq!(context.highly_rated_movies.len(), 2);
        assert!(context.highly_rated_movies.contains(&1));
        assert!(context.highly_rated_movies.contains(&3));
    }

    #[test]
    fn test_build_user_context_avg_rating() {
        let index = create_test_index();
        let context = build_user_context(&index, 1).unwrap();

        // Average: (5.0 + 3.0 + 4.5) / 3 = 4.166...
        assert!((context.avg_rating - 4.166).abs() < 0.01);
    }

    #[test]
    fn test_genre_preferences() {
        let index = create_test_index();
        let context = build_user_context(&index, 1).unwrap();

        // Action appears in movies 1 (5.0) and 3 (4.5) -> avg = 4.75
        // Drama appears in movie 2 (3.0) -> avg = 3.0
        // Adventure appears in movie 1 (5.0) -> avg = 5.0
        // SciFi appears in movie 3 (4.5) -> avg = 4.5

        assert!(context.genre_preferences.contains_key(&Genre::Action));
        let action_score = context.genre_preferences[&Genre::Action];
        assert!((action_score - 4.75).abs() < 0.01);
    }

    #[test]
    fn test_preferred_era() {
        let index = create_test_index();
        let context = build_user_context(&index, 1).unwrap();

        // Highly rated movies: 1 (2000) and 3 (2005)
        // Median: 2005 (or 2000 depending on implementation)
        assert!(context.preferred_era.is_some());
        let era = context.preferred_era.unwrap();
        assert!(era == 2000 || era == 2005);
    }

    #[test]
    fn test_user_not_found() {
        let index = DataIndex::new();
        let result = build_user_context(&index, 999);
        assert!(result.is_err());
    }

    #[test]
    fn test_user_with_no_ratings() {
        let mut index = DataIndex::new();
        index.insert_user(User {
            id: 1,
            gender: Gender::Male,
            age: AgeGroup::Age25To34,
            occupation: Occupation::Programmer,
            zipcode: "12345".to_string(),
        });

        let context = build_user_context(&index, 1).unwrap();
        assert_eq!(context.watched_movies.len(), 0);
        assert_eq!(context.highly_rated_movies.len(), 0);
        assert_eq!(context.avg_rating, 0.0);
    }
}
