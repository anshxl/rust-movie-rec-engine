//! Filter implementations for the candidate pipeline.
//!
//! This module contains all the concrete filter implementations
//! that can be composed into a FilterPipeline.

pub mod already_watched;
pub mod genre_preference;
pub mod minimum_rating;
pub mod recency;

// Re-export for convenience
pub use already_watched::AlreadyWatchedFilter;
pub use genre_preference::GenrePreferenceFilter;
pub use minimum_rating::MinimumRatingFilter;
pub use recency::RecencyFilter;
