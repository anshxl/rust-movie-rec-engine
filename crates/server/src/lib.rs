//! Server crate for the ReelRecs recommendation engine.
//!
//! This crate contains the orchestrator that coordinates all components
//! of the recommendation pipeline.

pub mod orchestrator;

pub use orchestrator::{MovieRecommendation, RecommendationOrchestrator};
