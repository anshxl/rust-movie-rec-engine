//! Thunder Source - In-network Collaborative Filtering
//!
//! Generates candidate recommendations based on collaborative filtering:
//! "Users who liked what you liked also liked these movies"
//!
//! ## Algorithm
//! 1. Find movies user rated highly (>= 4.0)
//! 2. For each highly-rated movie:
//!    - Find other users who also rated it highly
//!    - These are "similar users"
//! 3. Find what those similar users rated highly
//! 4. Score candidates by how many similar users liked them
//! 5. Return top ~300 candidates
//!
//! ## Learning Goals
//! - HashSet for deduplication and O(1) lookups
//! - HashMap for scoring aggregation
//! - Nested iterations over user-movie data
//! - Using Rayon for parallel processing (optional optimization)

use crate::types::{Candidate, CandidateSource, UserContext};
use data_loader::{DataIndex, MovieId, UserId};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, instrument};
use rayon::prelude::*;

/// Thunder source generates in-network candidates via collaborative filtering
pub struct ThunderSource {
    /// Shared reference to the data index (read-only, so no Mutex needed)
    data_index: Arc<DataIndex>,

    /// Minimum rating to consider a movie "highly rated"
    high_rating_threshold: f32,

    /// Minimum number of shared highly-rated movies to consider users similar
    min_shared_movies: usize,
}

impl ThunderSource {
    /// Create a new Thunder source
    ///
    /// ## Parameters
    /// - `data_index`: Shared reference to the loaded MovieLens data
    pub fn new(data_index: Arc<DataIndex>) -> Self {
        Self {
            data_index,
            high_rating_threshold: 4.0,
            min_shared_movies: 3,
        }
    }

    /// Configure the high rating threshold (default: 4.0)
    pub fn with_high_rating_threshold(mut self, threshold: f32) -> Self {
        self.high_rating_threshold = threshold;
        self
    }

    /// Configure minimum shared movies to consider users similar (default: 3)
    pub fn with_min_shared_movies(mut self, min: usize) -> Self {
        self.min_shared_movies = min;
        self
    }

    /// Generate candidate recommendations for a user
    #[instrument(skip(self, user_context), fields(user_id = user_context.user_id))]
    pub fn get_candidates(&self, user_context: &UserContext, limit: usize) -> Vec<Candidate> {
        debug!(
            "Generating Thunder candidates for user {} (highly rated: {})",
            user_context.user_id,
            user_context.highly_rated_movies.len()
        );
        // Step 1: Find similar users
        let similar_users = self.find_similar_users(user_context);
        debug!("Found {} similar users", similar_users.len()); // Add this!

        // Step 2: Get candidate movies from similar users
        let candidate_scores = self.get_candidate_scores(&similar_users, user_context);
        // Step 3: Convert to Candidate structs and sort
        let mut candidates: Vec<Candidate> = candidate_scores
            .into_iter()
            .map(|(movie_id, score)| {
                let mut candidate = Candidate::new(
                    movie_id,
                    CandidateSource::Thunder,
                    score as f32,
                );
                candidate.metadata.similar_users_count = Some(score);
                candidate
            })
            .collect();
        
        candidates.sort_by(|a, b| b.base_score.partial_cmp(&a.base_score).unwrap());
        candidates.truncate(limit);
        
        debug!("Generated {} Thunder candidates", candidates.len());
        candidates
        
    }

    /// Find users similar to the target user
    fn find_similar_users(&self, user_context: &UserContext) -> HashSet<UserId> {
        // let mut shared_counts: HashMap<UserId, u32> = HashMap::new();
        
        // for &movie_id in &user_context.highly_rated_movies {
        //     let ratings = self.data_index.get_movie_ratings(movie_id);
        //     for rating in ratings {
        //         if rating.user_id != user_context.user_id
        //             && rating.rating >= self.high_rating_threshold
        //         {
        //             *shared_counts.entry(rating.user_id).or_insert(0) += 1;
        //         }
        //     }
        // }
        let shared_counts = user_context.highly_rated_movies.par_iter().fold(
            || HashMap::new(),
            |mut local_counts, &movie_id| {
                let ratings = self.data_index.get_movie_ratings(movie_id);
                for rating in ratings {
                    if rating.user_id != user_context.user_id
                        && rating.rating >= self.high_rating_threshold
                    {
                        *local_counts.entry(rating.user_id).or_insert(0) += 1;
                    }
                }
                local_counts
            },
        ).reduce(
            || HashMap::new(),
            |mut acc, local_counts| {
                for (user_id, count) in local_counts {
                    *acc.entry(user_id).or_insert(0) += count;
                }
                acc
            },
        );
        
        // Convert to Vec
        let mut shared_counts_vec: Vec<(UserId, u32)> = shared_counts
            .into_iter()
            .filter(|(_user_id, count)| *count >= self.min_shared_movies as u32)
            .collect();

        // Sort by count DESC (cheap, uses the stored counts)
        shared_counts_vec.sort_unstable_by(|a, b| b.1.cmp(&a.1));

        // Take top 500 similar users and collect ids into a HashSet
        shared_counts_vec.truncate(500);
        let result: HashSet<UserId> = shared_counts_vec.into_iter().map(|(uid, _)| uid).collect();

        result
    }

    /// Get candidate movie scores from similar users
    fn get_candidate_scores(
        &self,
        similar_users: &HashSet<UserId>,
        user_context: &UserContext,
    ) -> HashMap<MovieId, u32> {
        let scores = similar_users.par_iter().fold(
            || HashMap::new(),
            |mut local_scores, &similar_user_id| {
                let ratings = self.data_index.get_user_ratings(similar_user_id);
                for rating in ratings {
                    if rating.rating >= self.high_rating_threshold
                        && !user_context.watched_movies.contains(&rating.movie_id)
                    {
                        *local_scores.entry(rating.movie_id).or_insert(0) += 1;
                    }
                }
                local_scores
            },
        ).reduce(
            || HashMap::new(),
            |mut acc, local_scores| {
                for (movie_id, count) in local_scores {
                    *acc.entry(movie_id).or_insert(0) += count;
                }
                acc
            },
        );
        scores
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user_context::build_user_context;
    use data_loader::{AgeGroup, Gender, Movie, Occupation, Rating, User};

    fn create_test_index() -> DataIndex {
        let mut index = DataIndex::new();

        // User 1 - our target user
        index.insert_user(User {
            id: 1,
            gender: Gender::Male,
            age: AgeGroup::Age25To34,
            occupation: Occupation::Programmer,
            zipcode: "12345".to_string(),
        });

        // User 2 - similar user (shares movies 1, 2, 3)
        index.insert_user(User {
            id: 2,
            gender: Gender::Female,
            age: AgeGroup::Age25To34,
            occupation: Occupation::Academic,
            zipcode: "54321".to_string(),
        });

        // User 3 - not similar (shares only movie 1)
        index.insert_user(User {
            id: 3,
            gender: Gender::Male,
            age: AgeGroup::Age35To44,
            occupation: Occupation::Technician,
            zipcode: "11111".to_string(),
        });

        // Movies
        for i in 1..=10 {
            index.insert_movie(Movie {
                id: i,
                title: format!("Movie {}", i),
                year: Some(2000),
                genres: vec![data_loader::Genre::Action],
            });
        }

        // User 1 ratings (highly rates 1, 2, 3)
        for movie_id in 1..=3 {
            index.insert_rating(Rating {
                user_id: 1,
                movie_id,
                rating: 5.0,
                timestamp: 1000000,
            });
        }

        // User 2 ratings (highly rates 1, 2, 3, 4, 5)
        for movie_id in 1..=5 {
            index.insert_rating(Rating {
                user_id: 2,
                movie_id,
                rating: 5.0,
                timestamp: 1000000,
            });
        }

        // User 3 ratings (highly rates only 1)
        index.insert_rating(Rating {
            user_id: 3,
            movie_id: 1,
            rating: 5.0,
            timestamp: 1000000,
        });

        index
    }

    #[test]
    fn test_find_similar_users() {
        let index = Arc::new(create_test_index());
        let thunder = ThunderSource::new(Arc::clone(&index));
        let context = build_user_context(&index, 1).unwrap();

        let similar = thunder.find_similar_users(&context);

        // User 2 shares 3 movies (1, 2, 3) -> similar
        // User 3 shares 1 movie (1) -> not similar (need 3)
        assert!(similar.contains(&2));
        assert!(!similar.contains(&3));
        assert_eq!(similar.len(), 1);
    }

    #[test]
    fn test_get_candidate_scores() {
        let index = Arc::new(create_test_index());
        let thunder = ThunderSource::new(Arc::clone(&index));
        let context = build_user_context(&index, 1).unwrap();

        let similar_users: HashSet<UserId> = [2].into_iter().collect();
        let scores = thunder.get_candidate_scores(&similar_users, &context);

        // User 2 rated movies 4 and 5 highly (movies 1-3 already watched by user 1)
        assert!(scores.contains_key(&4));
        assert!(scores.contains_key(&5));
        assert_eq!(scores[&4], 1); // 1 similar user liked it
        assert_eq!(scores[&5], 1);

        // Movies 1-3 should NOT appear (already watched)
        assert!(!scores.contains_key(&1));
        assert!(!scores.contains_key(&2));
        assert!(!scores.contains_key(&3));
    }

    #[test]
    fn test_get_candidates() {
        let index = Arc::new(create_test_index());
        let thunder = ThunderSource::new(Arc::clone(&index));
        let context = build_user_context(&index, 1).unwrap();

        let candidates = thunder.get_candidates(&context, 10);

        // Should get movies 4 and 5 as candidates
        assert!(candidates.len() >= 2);

        let movie_ids: Vec<MovieId> = candidates.iter().map(|c| c.movie_id).collect();
        assert!(movie_ids.contains(&4));
        assert!(movie_ids.contains(&5));

        // All should be from Thunder source
        for candidate in &candidates {
            assert_eq!(candidate.source, CandidateSource::Thunder);
        }
    }
}
