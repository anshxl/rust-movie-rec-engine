//! Phoenix Source - Out-of-Network Discovery
//!
//! Generates candidate recommendations through discovery mechanisms:
//! - Genre-based: Movies in user's preferred genres they haven't seen
//! - Popularity-based: Generally well-liked movies matching user preferences
//! - Temporal: Movies from user's preferred era
//!
//! ## Algorithm
//! 1. Analyze user's genre preferences
//! 2. Find movies through multiple strategies:
//!    - Top movies in user's favorite genres
//!    - Popular movies the user hasn't seen
//!    - Movies from user's preferred time period
//! 3. Combine and score by relevance
//! 4. Return top ~200 candidates
//!
//! ## Learning Goals
//! - Multiple discovery strategies in one source
//! - Combining scores from different approaches
//! - Using DataIndex secondary indices (genre_index, year_index)
//! - Deduplication across strategies

use crate::types::{Candidate, CandidateSource, UserContext};
use data_loader::{DataIndex, MovieId};
use std::collections::{HashMap};
use std::sync::Arc;
use tracing::{debug, instrument};

/// Phoenix source generates out-of-network candidates through discovery
pub struct PhoenixSource {
    /// Shared reference to the data index
    data_index: Arc<DataIndex>,

    /// Minimum average rating for movie quality filter
    min_avg_rating: f32,

    /// Minimum rating count for movie quality filter
    min_rating_count: u32,
}

impl PhoenixSource {
    /// Create a new Phoenix source
    pub fn new(data_index: Arc<DataIndex>) -> Self {
        Self {
            data_index,
            min_avg_rating: 3.5,
            min_rating_count: 10,
        }
    }

    /// Configure minimum average rating threshold (default: 3.5)
    pub fn with_min_avg_rating(mut self, rating: f32) -> Self {
        self.min_avg_rating = rating;
        self
    }

    /// Configure minimum rating count threshold (default: 10)
    pub fn with_min_rating_count(mut self, count: u32) -> Self {
        self.min_rating_count = count;
        self
    }

    /// Generate candidate recommendations for a user
    ///
    /// Combines multiple discovery strategies to find new movies
    #[instrument(skip(self, user_context), fields(user_id = user_context.user_id))]
    pub fn get_candidates(&self, user_context: &UserContext, limit: usize) -> Vec<Candidate> {
        debug!(
            "Generating Phoenix candidates for user {}",
            user_context.user_id
        );

        let mut all_candidates: HashMap<MovieId, Candidate> = HashMap::new();

        // Strategy 1: Genre-based discovery
        let genre_candidates = self.get_genre_based(user_context, limit / 2);
        for candidate in genre_candidates {
            all_candidates.insert(candidate.movie_id, candidate);
        }

        // Strategy 2: Popularity-based discovery
        let popularity_candidates = self.get_popularity_based(user_context, limit / 3);
        for candidate in popularity_candidates {
            // Merge with existing or insert new
            all_candidates
                .entry(candidate.movie_id)
                .and_modify(|c| {
                    // Combine scores if movie found by multiple strategies
                    c.base_score = (c.base_score + candidate.base_score) / 2.0;
                    c.metadata.from_popularity = true;
                })
                .or_insert(candidate);
        }

        // Strategy 3: Temporal discovery (if user has preferred era)
        if user_context.preferred_era.is_some() {
            let temporal_candidates = self.get_temporal(user_context, limit / 3);
            for candidate in temporal_candidates {
                all_candidates
                    .entry(candidate.movie_id)
                    .and_modify(|c| {
                        c.base_score = (c.base_score + candidate.base_score) / 2.0;
                        c.metadata.from_temporal = true;
                    })
                    .or_insert(candidate);
            }
        }

        // Convert to Vec, sort by score, and limit
        let mut candidates: Vec<Candidate> = all_candidates.into_values().collect();
        candidates.sort_by(|a, b| b.base_score.partial_cmp(&a.base_score).unwrap());
        candidates.truncate(limit);

        debug!("Generated {} Phoenix candidates", candidates.len());
        candidates
    }

    /// Genre-based discovery: Find highly-rated movies in user's preferred genres
    fn get_genre_based(&self, user_context: &UserContext, limit: usize) -> Vec<Candidate> {
        // Top genres
        let top_genres = user_context.top_genres(3);
        let mut candidates = Vec::new();

        // For each top genre, find unwatched, high-quality movies
        for genre in &top_genres {
            let genre_pref = user_context.genre_preferences.get(genre).unwrap_or(&0.0);
            let movie_ids = self.data_index.get_movies_by_genre(*genre);

            for &movie_id in movie_ids {
                // Filter unwatched
                if user_context.watched_movies.contains(&movie_id) {
                    continue;
                }

                // Get stats and filter quality
                if let Some(stats) = self.data_index.get_movie_stats(movie_id)
                    && stats.avg_rating >= self.min_avg_rating
                        && stats.rating_count >= self.min_rating_count
                    {
                        // Score = normalized rating * genre preference
                        let score = (stats.avg_rating / 5.0) * genre_pref;

                        let mut candidate = Candidate::new(
                            movie_id,
                            CandidateSource::Phoenix,
                            score,
                        );
                        candidate.metadata.matched_genres.push(*genre);
                        candidates.push(candidate);
                    }
            }
        }
        // Deduplicate, sort, and limit
        candidates.sort_by(|a, b| b.base_score.partial_cmp(&a.base_score).unwrap());
        candidates.truncate(limit);
        candidates

    }

    /// Popularity-based discovery: Find generally popular movies user hasn't seen
    fn get_popularity_based(&self, user_context: &UserContext, limit: usize) -> Vec<Candidate> {
        let mut candidates = Vec::new();
        
        // TODO: Get all movie IDs from DataIndex
        let all_movie_ids = self.data_index.get_all_movie_ids();
        
        for movie_id in all_movie_ids {
            if user_context.watched_movies.contains(&movie_id) {
                continue;
            }
        
            if let Some(stats) = self.data_index.get_movie_stats(movie_id)
                && stats.avg_rating >= self.min_avg_rating
                    && stats.rating_count >= self.min_rating_count
                {
                    let mut candidate = Candidate::new(
                        movie_id,
                        CandidateSource::Phoenix,
                        stats.popularity_score,
                    );
                    candidate.metadata.from_popularity = true;
                    candidates.push(candidate);
                }
        }
        
        candidates.sort_by(|a, b| b.base_score.partial_cmp(&a.base_score).unwrap());
        candidates.truncate(limit);
        candidates
    }

    /// Temporal discovery: Find movies from user's preferred era
    fn get_temporal(&self, user_context: &UserContext, limit: usize) -> Vec<Candidate> {
        let preferred_year = match user_context.preferred_era {
            Some(year) => year,
            None => return Vec::new(),
        }; // Already checked by caller
        let start_year = preferred_year.saturating_sub(5);
        let end_year = preferred_year.saturating_add(5);
        
        let mut candidates = Vec::new();
        
        //Query movies in year range from DataIndex
        let movies_in_range = self.data_index.get_movies_in_year_range(start_year, end_year);

        for movie_id in movies_in_range {
            if user_context.watched_movies.contains(&movie_id) {
                continue;
            }
        
            if let Some(movie) = self.data_index.get_movie(movie_id)
                && let Some(year) = movie.year
                    && let Some(stats) = self.data_index.get_movie_stats(movie_id)
                        && stats.avg_rating >= self.min_avg_rating
                            && stats.rating_count >= self.min_rating_count
                        {
                            // Score by year proximity + rating
                            let year_diff = (year as i32 - preferred_year as i32).abs();
                            let year_score = 1.0 - (year_diff as f32 / 10.0).min(1.0);
                            let rating_score = stats.avg_rating / 5.0;
                            let score = (year_score + rating_score) / 2.0;
        
                            let mut candidate = Candidate::new(
                                movie_id,
                                CandidateSource::Phoenix,
                                score,
                            );
                            candidate.metadata.from_temporal = true;
                            candidates.push(candidate);
                        }
        }
        
        candidates.sort_by(|a, b| b.base_score.partial_cmp(&a.base_score).unwrap());
        candidates.truncate(limit);
        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user_context::build_user_context;
    use data_loader::{AgeGroup, Gender, Movie, Occupation, Rating, User, Genre};

    fn create_test_index() -> DataIndex {
        let mut index = DataIndex::new();

        // User 1
        index.insert_user(User {
            id: 1,
            gender: Gender::Male,
            age: AgeGroup::Age25To34,
            occupation: Occupation::Programmer,
            zipcode: "12345".to_string(),
        });

        // Movies with different genres and years
        index.insert_movie(Movie {
            id: 1,
            title: "Action Movie (2000)".to_string(),
            year: Some(2000),
            genres: vec![Genre::Action],
        });

        index.insert_movie(Movie {
            id: 2,
            title: "Drama Movie (2000)".to_string(),
            year: Some(2000),
            genres: vec![Genre::Drama],
        });

        index.insert_movie(Movie {
            id: 3,
            title: "Action Movie 2 (2005)".to_string(),
            year: Some(2005),
            genres: vec![Genre::Action],
        });

        index.insert_movie(Movie {
            id: 4,
            title: "Unseen Action (2002)".to_string(),
            year: Some(2002),
            genres: vec![Genre::Action],
        });

        index.insert_movie(Movie {
            id: 5,
            title: "Old Drama (1980)".to_string(),
            year: Some(1980),
            genres: vec![Genre::Drama],
        });

        // User 1 ratings (likes Action movies)
        index.insert_rating(Rating {
            user_id: 1,
            movie_id: 1,
            rating: 5.0,
            timestamp: 1000000,
        });

        index.insert_rating(Rating {
            user_id: 1,
            movie_id: 3,
            rating: 4.5,
            timestamp: 1000001,
        });

        // Add some ratings to other movies to give them stats
        for movie_id in 2..=5 {
            for user_id in 10..20 {
                index.insert_rating(Rating {
                    user_id,
                    movie_id,
                    rating: 4.0,
                    timestamp: 1000000,
                });
            }
        }

        // IMPORTANT: Build secondary indices and compute stats
        // (Required when manually creating DataIndex with new())
        index.build_secondary_indices();
        index.compute_movie_stats();

        index
    }

    #[test]
    fn test_genre_based_discovery() {
        let index = Arc::new(create_test_index());
        let phoenix = PhoenixSource::new(Arc::clone(&index));
        let context = build_user_context(&index, 1).unwrap();

        let candidates = phoenix.get_genre_based(&context, 10);

        // Should find unseen Action movies (4)
        let movie_ids: Vec<MovieId> = candidates.iter().map(|c| c.movie_id).collect();
        assert!(movie_ids.contains(&4));

        // Should not include watched movies (1, 3)
        assert!(!movie_ids.contains(&1));
        assert!(!movie_ids.contains(&3));
    }

    #[test]
    fn test_get_candidates() {
        let index = Arc::new(create_test_index());
        let phoenix = PhoenixSource::new(Arc::clone(&index));
        let context = build_user_context(&index, 1).unwrap();

        let candidates = phoenix.get_candidates(&context, 10);

        // Should get some candidates
        assert!(!candidates.is_empty());

        // All should be from Phoenix source
        for candidate in &candidates {
            assert_eq!(candidate.source, CandidateSource::Phoenix);
        }

        // Should not include watched movies
        for candidate in &candidates {
            assert!(!context.watched_movies.contains(&candidate.movie_id));
        }
    }
}
