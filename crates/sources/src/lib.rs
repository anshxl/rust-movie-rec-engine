//! # Sources Crate
//!
//! This crate implements candidate generation sources for movie recommendations.
//!
//! ## Components
//!
//! ### Thunder Source (In-Network)
//! Collaborative filtering based on similar users:
//! - "Users who liked what you liked also liked..."
//! - Finds ~300 candidates from user's social network
//!
//! ### Phoenix Source (Out-of-Network)
//! Discovery through multiple strategies:
//! - Genre-based: Movies in user's preferred genres
//! - Popularity-based: Generally well-liked movies
//! - Temporal: Movies from user's preferred era
//! - Finds ~200 candidates through exploration
//!
//! ## Example Usage
//!
//! ```ignore
//! use sources::{ThunderSource, PhoenixSource, user_context::build_user_context};
//! use data_loader::DataIndex;
//! use std::sync::Arc;
//!
//! // Load data
//! let data_index = Arc::new(DataIndex::load_from_files("data/ml-1m")?);
//!
//! // Build user context
//! let context = build_user_context(&data_index, user_id)?;
//!
//! // Generate candidates
//! let thunder = ThunderSource::new(data_index.clone());
//! let phoenix = PhoenixSource::new(data_index.clone());
//!
//! let thunder_candidates = thunder.get_candidates(&context, 300);
//! let phoenix_candidates = phoenix.get_candidates(&context, 200);
//! ```
//!
//! ## Learning Goals - Phase 3
//!
//! This phase teaches:
//!
//! 1. **Algorithm Implementation**: Translating recommendation algorithms into Rust
//! 2. **HashMap Operations**: Aggregation, counting, scoring with entry() API
//! 3. **HashSet Usage**: O(1) lookups for filtering and deduplication
//! 4. **Arc for Sharing**: Sharing DataIndex across sources without copying
//! 5. **Builder Pattern**: Configurable sources with method chaining
//! 6. **Instrumentation**: Using tracing for observability
//! 7. **Multiple Strategies**: Combining different recommendation approaches
//!
//! ## Performance Targets
//!
//! - Thunder candidates: <5ms for 300 candidates
//! - Phoenix candidates: <3ms for 200 candidates
//! - Both can run in parallel

// Public modules
pub mod types;
pub mod user_context;
pub mod thunder;
pub mod phoenix;

// Re-export commonly used types
pub use types::{Candidate, CandidateMetadata, CandidateSource, UserContext};
pub use thunder::ThunderSource;
pub use phoenix::PhoenixSource;

#[cfg(test)]
mod tests {
    use super::*;
    use data_loader::{AgeGroup, DataIndex, Gender, Movie, Occupation, Rating, User};
    use std::sync::Arc;

    fn create_test_index() -> DataIndex {
        let mut index = DataIndex::new();

        // Add test user
        index.insert_user(User {
            id: 1,
            gender: Gender::Male,
            age: AgeGroup::Age25To34,
            occupation: Occupation::Programmer,
            zipcode: "12345".to_string(),
        });

        // Add test movie
        index.insert_movie(Movie {
            id: 1,
            title: "Test Movie (2000)".to_string(),
            year: Some(2000),
            genres: vec![data_loader::Genre::Action],
        });

        // Add test rating
        index.insert_rating(Rating {
            user_id: 1,
            movie_id: 1,
            rating: 5.0,
            timestamp: 1000000,
        });

        index
    }

    #[test]
    fn test_thunder_source_creation() {
        let index = create_test_index();
        let _thunder = ThunderSource::new(Arc::new(index));
        // Just verify it compiles and can be created
        assert!(true);
    }

    #[test]
    fn test_phoenix_source_creation() {
        let index = create_test_index();
        let _phoenix = PhoenixSource::new(Arc::new(index));
        // Just verify it compiles and can be created
        assert!(true);
    }

    #[test]
    fn test_candidate_creation() {
        let candidate = Candidate::new(1, CandidateSource::Thunder, 0.85);
        assert_eq!(candidate.movie_id, 1);
        assert_eq!(candidate.source, CandidateSource::Thunder);
        assert_eq!(candidate.base_score, 0.85);
    }
}
