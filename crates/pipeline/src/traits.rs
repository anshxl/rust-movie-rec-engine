//! Core traits for the filtering pipeline.
//!
//! This module defines the Filter trait that allows composable,
//! extensible filters to be applied to candidate sets.

use anyhow::Result;
use sources::{Candidate, UserContext};

/// Core trait for filtering candidates.
///
/// All filters must implement this trait to be used in the FilterPipeline.
///
/// ## Design Note
/// - `Send + Sync` allows filters to be used in concurrent contexts
/// - Filters take ownership of the Vec<Candidate> and return a filtered Vec
/// - This allows for efficient transformations without unnecessary cloning
pub trait Filter: Send + Sync {
    /// Returns the name of this filter (for logging/debugging)
    fn name(&self) -> &str;

    /// Apply this filter to a set of candidates.
    ///
    /// # Arguments
    /// * `candidates` - The candidates to filter (takes ownership)
    /// * `context` - User context containing preferences and history
    ///
    /// # Returns
    /// * `Ok(Vec<Candidate>)` - The filtered candidates
    /// * `Err` - If filtering fails
    fn apply(
        &self,
        candidates: Vec<Candidate>,
        context: &UserContext,
    ) -> Result<Vec<Candidate>>;
}
