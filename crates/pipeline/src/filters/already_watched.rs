//! Filter to remove movies the user has already watched.
//!
//! This is typically the first filter in the pipeline, as there's no
//! point in recommending movies the user has already seen.

use crate::traits::Filter;
use anyhow::Result;
use sources::{Candidate, UserContext};

/// Removes candidates that the user has already rated.
///
/// ## Algorithm
/// Uses the HashSet in UserContext.watched_movies for O(1) lookups.
pub struct AlreadyWatchedFilter;

impl Filter for AlreadyWatchedFilter {
    fn name(&self) -> &str {
        "AlreadyWatchedFilter"
    }

    fn apply(
        &self,
        candidates: Vec<Candidate>,
        context: &UserContext,
    ) -> Result<Vec<Candidate>> {
        let filtered: Vec<Candidate> = candidates
            .into_iter()
            .filter(|candidate| ! context.watched_movies.contains(&candidate.movie_id))
            .collect();
        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use data_loader::MovieId;
    use sources::{Candidate, CandidateSource};
    // use std::collections::HashSet;

    #[test]
    fn test_already_watched_filter() {
        let mut context = UserContext::new(1);
        context.watched_movies.insert(100);
        context.watched_movies.insert(200);

        let candidates = vec![
            Candidate::new(100, CandidateSource::Thunder, 0.9),
            Candidate::new(101, CandidateSource::Thunder, 0.8),
            Candidate::new(200, CandidateSource::Phoenix, 0.7),
            Candidate::new(300, CandidateSource::Phoenix, 0.6),
        ];

        let filter = AlreadyWatchedFilter;
        let filtered = filter.apply(candidates, &context).unwrap();

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].movie_id, 101);
        assert_eq!(filtered[1].movie_id, 300);
    }
}
