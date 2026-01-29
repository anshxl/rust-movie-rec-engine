//! Pipeline for filtering and feature engineering of movie candidates.
//!
//! This crate provides:
//! - Filter trait and implementations for candidate filtering
//! - FilterPipeline for composing filters
//! - FeatureEngineer for computing ML features
//!
//! ## Architecture
//! The pipeline processes candidates in stages:
//! 1. Filters remove unwanted candidates (already watched, wrong genre, low quality)
//! 2. FeatureEngineer computes features for remaining candidates
//! 3. Features are sent to ML service for scoring
//!
//! ## Example Usage
//! ```ignore
//! use pipeline::{FilterPipeline, FeatureEngineer};
//! use pipeline::filters::*;
//!
//! // Build the filter pipeline
//! let pipeline = FilterPipeline::new()
//!     .add_filter(AlreadyWatchedFilter)
//!     .add_filter(MinimumRatingFilter::new(index.clone(), 3.5, 10))
//!     .add_filter(GenrePreferenceFilter::new(index.clone(), 3));
//!
//! // Apply filters
//! let filtered = pipeline.apply(candidates, &context)?;
//!
//! // Compute features
//! let engineer = FeatureEngineer::new(index.clone());
//! let features = engineer.compute_features(&filtered, &context);
//! ```

pub mod traits;
pub mod filters;
pub mod filter_pipeline;
pub mod features;

// Re-export main types
pub use traits::Filter;
pub use filter_pipeline::FilterPipeline;
pub use features::{CandidateFeatures, FeatureEngineer};
