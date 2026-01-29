//! The FilterPipeline orchestrates multiple filters.
//!
//! This module provides the main FilterPipeline struct that chains
//! multiple filters together using the builder pattern.

use crate::traits::Filter;
use anyhow::Result;
use sources::{Candidate, UserContext};
use tracing;

/// Chains multiple filters together into a processing pipeline.
///
/// ## Usage
/// ```ignore
/// let pipeline = FilterPipeline::new()
///     .add_filter(AlreadyWatchedFilter)
///     .add_filter(MinimumRatingFilter::new(index.clone(), 3.5, 10))
///     .add_filter(GenrePreferenceFilter::new(index.clone(), 3));
///
/// let filtered = pipeline.apply(candidates, &context)?;
/// ```
pub struct FilterPipeline {
    filters: Vec<Box<dyn Filter>>,
}

impl FilterPipeline {
    /// Create a new empty FilterPipeline.
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    /// Add a filter to the pipeline (builder pattern).
    ///
    /// # Arguments
    /// * `filter` - Any type implementing the Filter trait
    ///
    /// # Returns
    /// Self for method chaining
    pub fn add_filter(mut self, filter: impl Filter + 'static) -> Self {
        self.filters.push(Box::new(filter));
        self
    }

    /// Apply all filters in sequence to the candidates.
    ///
    /// ## Algorithm
    /// 1. Start with the input candidates
    /// 2. For each filter in order:
    ///    a. Log filter name and input count
    ///    b. Apply the filter
    ///    c. Log output count
    /// 3. Return final filtered set
    ///
    /// # Arguments
    /// * `candidates` - The candidates to filter
    /// * `context` - User context for filtering decisions
    ///
    /// # Returns
    /// * `Ok(Vec<Candidate>)` - The filtered candidates after all filters
    /// * `Err` - If any filter fails
    pub fn apply(
        &self,
        candidates: Vec<Candidate>,
        context: &UserContext,
    ) -> Result<Vec<Candidate>> {
        let mut current = candidates;
        for filter in &self.filters {
            tracing::debug!(
                "Applying filter: {} (input count: {})",
                filter.name(),
                current.len()
            );
            current = filter.apply(current, context)?;
            tracing::debug!(
                "Filter applied: {} (output count: {})",
                filter.name(),
                current.len()
            );
        }
        Ok(current)
    }
}

impl Default for FilterPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filters::AlreadyWatchedFilter;
    use sources::{Candidate, CandidateSource};

    #[test]
    fn test_empty_pipeline() {
        let pipeline = FilterPipeline::new();
        let context = UserContext::new(1);

        let candidates = vec![
            Candidate::new(1, CandidateSource::Thunder, 0.9),
            Candidate::new(2, CandidateSource::Phoenix, 0.8),
        ];

        let filtered = pipeline.apply(candidates.clone(), &context).unwrap();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_single_filter() {
        let mut context = UserContext::new(1);
        context.watched_movies.insert(1);

        let pipeline = FilterPipeline::new()
            .add_filter(AlreadyWatchedFilter);

        let candidates = vec![
            Candidate::new(1, CandidateSource::Thunder, 0.9),
            Candidate::new(2, CandidateSource::Phoenix, 0.8),
        ];

        let filtered = pipeline.apply(candidates, &context).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].movie_id, 2);
    }
}
