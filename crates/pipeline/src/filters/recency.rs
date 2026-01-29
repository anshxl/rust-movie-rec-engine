//! Optional filter for temporal preferences.
//!
//! Filters out movies that are too old or too new compared to
//! the user's demonstrated preferences.

use crate::traits::Filter;
use anyhow::Result;
use data_loader::DataIndex;
use sources::{Candidate, UserContext};
use std::sync::Arc;

/// Filters candidates based on user's preferred movie era.
///
/// ## Algorithm
/// 1. Use context.preferred_era as the center point
/// 2. Keep movies within +/- year_tolerance years
/// 3. If no preferred era, keep all movies
pub struct RecencyFilter {
    data_index: Arc<DataIndex>,
    year_tolerance: u16,
}

impl RecencyFilter {
    /// Create a new RecencyFilter.
    ///
    /// # Arguments
    /// * `data_index` - Shared reference to DataIndex for movie lookups
    /// * `year_tolerance` - How many years +/- from preferred era (typically 10-15)
    pub fn new(data_index: Arc<DataIndex>, year_tolerance: u16) -> Self {
        Self {
            data_index,
            year_tolerance,
        }
    }
}

impl Filter for RecencyFilter {
    fn name(&self) -> &str {
        "RecencyFilter"
    }

    fn apply(
        &self,
        candidates: Vec<Candidate>,
        context: &UserContext,
    ) -> Result<Vec<Candidate>> {
        if context.preferred_era.is_none() {
            return Ok(candidates);
        }
        let preferred_era = context.preferred_era.unwrap();
        let filtered: Vec<Candidate> = candidates
            .into_iter()
            .filter(|candidate| {
                if let Some(movie) = self.data_index.get_movie(candidate.movie_id) {
                    // Check if movie year exists. If not, keep the movie.
                    if let Some(year) = movie.year {
                        let lower_bound = preferred_era.saturating_sub(self.year_tolerance);
                        let upper_bound = preferred_era.saturating_add(self.year_tolerance);
                        year >= lower_bound && year <= upper_bound
                    } else {
                        true
                    }
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
    use data_loader::Movie;
    use sources::{Candidate, CandidateSource};

    fn create_test_index() -> DataIndex {
        let mut index = DataIndex::new();

        index.insert_movie(Movie {
            id: 1,
            title: "Old Movie (1980)".to_string(),
            year: Some(1980),
            genres: vec![],
        });

        index.insert_movie(Movie {
            id: 2,
            title: "Era Movie (2000)".to_string(),
            year: Some(2000),
            genres: vec![],
        });

        index.insert_movie(Movie {
            id: 3,
            title: "Recent Movie (2020)".to_string(),
            year: Some(2020),
            genres: vec![],
        });

        index
    }

    #[test]
    fn test_recency_filter() {
        let index = Arc::new(create_test_index());
        let mut context = UserContext::new(1);
        context.preferred_era = Some(2000);

        let candidates = vec![
            Candidate::new(1, CandidateSource::Thunder, 0.9),  // 1980 - too old
            Candidate::new(2, CandidateSource::Phoenix, 0.8),  // 2000 - perfect
            Candidate::new(3, CandidateSource::Phoenix, 0.7),  // 2020 - too new
        ];

        let filter = RecencyFilter::new(index, 10);
        let filtered = filter.apply(candidates, &context).unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].movie_id, 2);
    }

    #[test]
    fn test_recency_filter_no_preferred_era() {
        let index = Arc::new(create_test_index());
        let context = UserContext::new(1);

        let candidates = vec![
            Candidate::new(1, CandidateSource::Thunder, 0.9),
            Candidate::new(2, CandidateSource::Phoenix, 0.8),
            Candidate::new(3, CandidateSource::Phoenix, 0.7),
        ];

        let filter = RecencyFilter::new(index, 10);
        let filtered = filter.apply(candidates, &context).unwrap();

        assert_eq!(filtered.len(), 3);
    }
}
