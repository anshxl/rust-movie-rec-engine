//! Filter to ensure minimum quality threshold.
//!
//! Removes movies with low average ratings or too few ratings,
//! ensuring we only recommend quality content.

use crate::traits::Filter;
use anyhow::Result;
use data_loader::DataIndex;
use sources::{Candidate, UserContext};
use std::sync::Arc;

/// Removes candidates below quality thresholds.
///
/// ## Algorithm
/// For each candidate:
/// 1. Get MovieStats from DataIndex
/// 2. Check if avg_rating >= min_rating
/// 3. Check if rating_count >= min_count
/// 4. Keep only if both conditions met
pub struct MinimumRatingFilter {
    data_index: Arc<DataIndex>,
    min_rating: f32,
    min_count: u32,
}

impl MinimumRatingFilter {
    /// Create a new MinimumRatingFilter.
    ///
    /// # Arguments
    /// * `data_index` - Shared reference to DataIndex for stats lookups
    /// * `min_rating` - Minimum average rating (typically 3.5)
    /// * `min_count` - Minimum number of ratings (typically 10)
    pub fn new(data_index: Arc<DataIndex>, min_rating: f32, min_count: u32) -> Self {
        Self {
            data_index,
            min_rating,
            min_count,
        }
    }
}

impl Filter for MinimumRatingFilter {
    fn name(&self) -> &str {
        "MinimumRatingFilter"
    }

    fn apply(
        &self,
        candidates: Vec<Candidate>,
        _context: &UserContext,
    ) -> Result<Vec<Candidate>> {
        let filtered: Vec<Candidate> = candidates
            .into_iter()
            .filter(|candidate| {
                if let Some(stats) = self.data_index.get_movie_stats(candidate.movie_id) {
                    stats.avg_rating >= self.min_rating && stats.rating_count >= self.min_count
                } else {
                    false
                }
            })
            .collect();

        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_loader::{Movie, Rating};
    use sources::{Candidate, CandidateSource};

    fn create_test_index() -> DataIndex {
        let mut index = DataIndex::new();

        index.insert_movie(Movie {
            id: 1,
            title: "High Rated Movie".to_string(),
            year: Some(2000),
            genres: vec![],
        });

        index.insert_movie(Movie {
            id: 2,
            title: "Low Rated Movie".to_string(),
            year: Some(2000),
            genres: vec![],
        });

        index.insert_movie(Movie {
            id: 3,
            title: "Few Ratings Movie".to_string(),
            year: Some(2000),
            genres: vec![],
        });

        index
    }

    #[test]
    fn test_minimum_rating_filter() {
        let mut index = create_test_index();

        // Add ratings for movie 1 (high rated with many ratings)
        for i in 0..20 {
            index.insert_rating(Rating {
                user_id: i,
                movie_id: 1,
                rating: 4.5,
                timestamp: 1000000,
            });
        }

        // Add ratings for movie 2 (low rated)
        for i in 0..20 {
            index.insert_rating(Rating {
                user_id: i + 100,
                movie_id: 2,
                rating: 2.0,
                timestamp: 1000000,
            });
        }

        // Add ratings for movie 3 (good rating but too few)
        for i in 0..5 {
            index.insert_rating(Rating {
                user_id: i + 200,
                movie_id: 3,
                rating: 4.5,
                timestamp: 1000000,
            });
        }
        index.compute_movie_stats();
        
        let index = Arc::new(index);

        let candidates = vec![
            Candidate::new(1, CandidateSource::Thunder, 0.9),
            Candidate::new(2, CandidateSource::Phoenix, 0.8),
            Candidate::new(3, CandidateSource::Phoenix, 0.7),
        ];

        let filter = MinimumRatingFilter::new(index, 3.5, 10);
        let filtered = filter.apply(candidates, &UserContext::new(1)).unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].movie_id, 1);
    }
}
