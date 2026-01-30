//! Feature engineering for candidate scoring.
//!
//! This module computes features for each candidate that will be
//! used in the ML scoring phase.

use data_loader::{DataIndex, MovieId};
use rayon::prelude::*;
use sources::{Candidate, UserContext};
use std::sync::Arc;

/// Features computed for each candidate.
///
/// These features are used as inputs to the ML scoring service
/// and for final ranking decisions.
#[derive(Debug, Clone)]
pub struct CandidateFeatures {
    pub movie_id: MovieId,

    // Genre similarity
    pub genre_overlap_score: f32,
    pub genre_diversity_score: f32,

    // Collaborative signals
    pub collaborative_score: f32,
    pub similar_users_count: u32,

    // Popularity metrics
    pub avg_rating: f32,
    pub rating_count: u32,
    pub popularity_percentile: f32,

    // Temporal features
    pub movie_year: Option<u16>,
    pub year_preference_score: f32,

    // Recency
    pub days_since_released: f32,
}

impl CandidateFeatures {
    /// Create a new CandidateFeatures with default values.
    pub fn new(movie_id: MovieId) -> Self {
        Self {
            movie_id,
            genre_overlap_score: 0.0,
            genre_diversity_score: 0.0,
            collaborative_score: 0.0,
            similar_users_count: 0,
            avg_rating: 0.0,
            rating_count: 0,
            popularity_percentile: 0.0,
            movie_year: None,
            year_preference_score: 0.0,
            days_since_released: 0.0,
        }
    }
}

/// Computes features for candidates in parallel.
///
/// ## Performance Note
/// Uses Rayon for parallel feature computation. For 200 candidates,
/// target is <10ms total.
#[derive(Clone)]
pub struct FeatureEngineer {
    data_index: Arc<DataIndex>,
}

impl FeatureEngineer {
    /// Create a new FeatureEngineer.
    pub fn new(data_index: Arc<DataIndex>) -> Self {
        Self { data_index }
    }

    /// Compute features for all candidates in parallel.
    ///
    /// # Arguments
    /// * `candidates` - The candidates to compute features for
    /// * `user_context` - User context for personalized features
    ///
    /// # Returns
    /// Vec of CandidateFeatures, one per candidate, in the same order
    pub fn compute_features(
        &self,
        candidates: &[Candidate],
        user_context: &UserContext,
    ) -> Vec<CandidateFeatures> {
        candidates
            .par_iter()
            .map(|candidate| self.compute_single(candidate, user_context))
            .collect()
    }

    /// Compute features for a single candidate.
    ///
    /// This is called in parallel for each candidate.
    fn compute_single(
        &self,
        candidate: &Candidate,
        user_context: &UserContext,
    ) -> CandidateFeatures {
        let mut features = CandidateFeatures::new(candidate.movie_id);
        // Movie info
        let movie = match self.data_index.get_movie(candidate.movie_id) {
            Some(m) => m,
            None => return features, // Return default if movie not found
        };
        // Stats info
        let stats = match self.data_index.get_movie_stats(candidate.movie_id) {
            Some(s) => s,
            None => return features, // Return default if stats not found
        };

        // Genre overlap
        features.genre_overlap_score = self.compute_genre_overlap(
            candidate.movie_id,
            user_context,
        );

        // Collaborative score
        features.collaborative_score = candidate.base_score;

        // Similar users count
        features.similar_users_count = candidate.metadata.similar_users_count.unwrap_or(0);

        // Avg rating and count
        features.avg_rating = stats.avg_rating;
        features.rating_count = stats.rating_count;

        // Popularity percentile
        features.popularity_percentile = self.compute_popularity_percentile(candidate.movie_id);

        // Movie year
        features.movie_year = movie.year;

        // Year preference score
        features.year_preference_score = self.compute_year_preference(
            movie.year,
            user_context.preferred_era,
        );

        // Days since released
        features.days_since_released = if let Some(year) = movie.year {
            let current_year = 2026; // Assume current year
            ((current_year - year) * 365) as f32
        } else {
            0.0
        };
        features
    }

    /// Compute genre overlap score (Jaccard similarity).
    ///
    /// ## Algorithm
    /// Jaccard similarity = |intersection| / |union|
    /// Compare movie's genres with user's top genres
    fn compute_genre_overlap(
        &self,
        movie_id: MovieId,
        user_context: &UserContext,
    ) -> f32 {
        // Get user's top genres
        let user_top_genres = user_context.top_genres(3)
            .into_iter()
            .collect::<std::collections::HashSet<_>>(); 

        // Early return if movie not found
        let movie = match self.data_index.get_movie(movie_id) {
            Some(m) => m,
            None => return 0.0,
        };

        // Make genre sets
        let movie_genres = movie.genres.iter().cloned().collect::<std::collections::HashSet<_>>();
        
        let intersection = user_top_genres.intersection(&movie_genres).count() as f32;
        let union = user_top_genres.union(&movie_genres).count() as f32;
        if union == 0.0 {
            0.0
        } else {
            intersection / union
        }
    }

    /// Compute year preference score.
    ///
    /// ## Algorithm
    /// - If no preferred era: return 0.5 (neutral)
    /// - Otherwise: 1.0 - (|movie_year - preferred_era| / max_distance)
    /// - Max distance could be 50 years
    fn compute_year_preference(
        &self,
        movie_year: Option<u16>,
        preferred_era: Option<u16>,
    ) -> f32 {
        const MAX_DISTANCE: f32 = 50.0;

        match (movie_year, preferred_era) {
            (Some(my), Some(pe)) => {
                let distance = (my as i32 - pe as i32).abs() as f32;
                let score = 1.0 - (distance / MAX_DISTANCE);
                score.clamp(0.0, 1.0)
            }
            _ => 0.5, // Neutral score
        }
    }

    /// Compute popularity percentile.
    ///
    /// ## Algorithm
    /// Rank movies by rating_count, return percentile (0.0 to 1.0)
    /// This requires knowing the distribution across all movies
    fn compute_popularity_percentile(
        &self,
        movie_id: MovieId,
    ) -> f32 {
        let count = match self.data_index.get_movie_stats(movie_id) {
            Some(stats) => stats.rating_count,
            None => 0,
        };
        (count as f32 / 500.0).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_loader::{Genre, Movie, Rating};
    use sources::{Candidate, CandidateSource};

    fn create_test_index() -> DataIndex {
        let mut index = DataIndex::new();

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

        // Add some ratings for stats
        for i in 0..20 {
            index.insert_rating(Rating {
                user_id: i,
                movie_id: 1,
                rating: 4.5,
                timestamp: 1000000,
            });
        }

        for i in 0..10 {
            index.insert_rating(Rating {
                user_id: i + 100,
                movie_id: 2,
                rating: 4.0,
                timestamp: 1000000,
            });
        }

        index
    }

    #[test]
    fn test_feature_computation() {
        let index = Arc::new(create_test_index());
        let engineer = FeatureEngineer::new(index);

        let mut context = UserContext::new(1);
        context.genre_preferences.insert(Genre::Action, 4.5);
        context.genre_preferences.insert(Genre::Adventure, 4.0);
        context.preferred_era = Some(2000);

        let candidates = vec![
            Candidate::new(1, CandidateSource::Thunder, 0.9),
        ];

        let features = engineer.compute_features(&candidates, &context);

        assert_eq!(features.len(), 1);
        assert_eq!(features[0].movie_id, 1);
    }
}
