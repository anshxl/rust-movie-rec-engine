//! Filter to keep only movies matching user's preferred genres.
//!
//! This filter helps ensure recommended movies align with the user's
//! demonstrated genre preferences.

use crate::traits::Filter;
use anyhow::Result;
use data_loader::{DataIndex};
use sources::{Candidate, UserContext};
use std::sync::Arc;

/// Keeps only candidates that match the user's top N preferred genres.
///
/// ## Algorithm
/// 1. Get user's top N genres from UserContext
/// 2. For each candidate, check if movie has at least one matching genre
/// 3. Keep movies with genre overlap
pub struct GenrePreferenceFilter {
    data_index: Arc<DataIndex>,
    top_n_genres: usize,
}

impl GenrePreferenceFilter {
    /// Create a new GenrePreferenceFilter.
    ///
    /// # Arguments
    /// * `data_index` - Shared reference to DataIndex for movie lookups
    /// * `top_n_genres` - How many top genres to consider (typically 3)
    pub fn new(data_index: Arc<DataIndex>, top_n_genres: usize) -> Self {
        Self {
            data_index,
            top_n_genres,
        }
    }
}

impl Filter for GenrePreferenceFilter {
    fn name(&self) -> &str {
        "GenrePreferenceFilter"
    }

    fn apply(
        &self,
        candidates: Vec<Candidate>,
        context: &UserContext,
    ) -> Result<Vec<Candidate>> {
        // Get user's top N genres
        let top_genres = context.top_genres(self.top_n_genres);

        // For each candidate, check if movie genres overlap with top genres
        let filtered: Vec<Candidate> = candidates
            .into_iter()
            .filter(|candidate| {
                if let Some(movie) = self.data_index.get_movie(candidate.movie_id) {
                    // Check for genre overlap
                    movie
                        .genres
                        .iter()
                        .any(|genre| top_genres.contains(genre))
                } else {
                    false // Exclude if movie not found
                }
            })
            .collect();
        Ok(filtered)    
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_loader::{Movie, Genre};
    use sources::{Candidate, CandidateSource};
    // use std::collections::HashMap;

    fn create_test_index() -> DataIndex {
        let mut index = DataIndex::new();

        index.insert_movie(Movie {
            id: 1,
            title: "Action Movie".to_string(),
            year: Some(2000),
            genres: vec![Genre::Action, Genre::Adventure],
        });

        index.insert_movie(Movie {
            id: 2,
            title: "Drama Movie".to_string(),
            year: Some(1995),
            genres: vec![Genre::Drama],
        });

        index.insert_movie(Movie {
            id: 3,
            title: "Sci-Fi Movie".to_string(),
            year: Some(2005),
            genres: vec![Genre::SciFi],
        });

        index
    }

    #[test]
    fn test_genre_preference_filter() {
        let index = Arc::new(create_test_index());
        let mut context = UserContext::new(1);

        context.genre_preferences.insert(Genre::Action, 4.5);
        context.genre_preferences.insert(Genre::Drama, 3.0);
        context.genre_preferences.insert(Genre::Adventure, 4.0);

        let candidates = vec![
            Candidate::new(1, CandidateSource::Thunder, 0.9), // Action/Adventure - should match
            Candidate::new(2, CandidateSource::Phoenix, 0.8), // Drama - should match
            Candidate::new(3, CandidateSource::Phoenix, 0.7), // SciFi - should NOT match
        ];

        let filter = GenrePreferenceFilter::new(index, 3);
        let filtered = filter.apply(candidates, &context).unwrap();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|c| c.movie_id == 1));
        assert!(filtered.iter().any(|c| c.movie_id == 2));
    }
}
