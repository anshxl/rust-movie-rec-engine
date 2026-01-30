//! # Recommendation Orchestrator
//!
//! This module coordinates the entire recommendation pipeline:
//! 1. Fetch user context
//! 2. Generate candidates (Thunder + Phoenix in parallel)
//! 3. Merge and deduplicate candidates
//! 4. Apply filters
//! 5. Compute features
//! 6. Call ML service for scoring
//! 7. Post-process and rank
//! 8. Return top N recommendations
//!
//! ## Learning Goals
//!
//! This component teaches you:
//! - Async coordination with tokio::join!
//! - Using spawn_blocking for CPU-bound work
//! - Error handling across async boundaries
//! - Instrumentation and timing
//! - Combining multiple components into a pipeline

use std::collections::HashMap;
// use std::fmt::format;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use tracing::{info, warn};

use data_loader::{DataIndex, MovieId, UserId};
use sources::{Candidate, CandidateSource, PhoenixSource, ThunderSource, UserContext};
use pipeline::{FeatureEngineer, FilterPipeline};
use pipeline::filters::{AlreadyWatchedFilter, MinimumRatingFilter, GenrePreferenceFilter};
use ml_client::MLScorerClient;

/// Final recommendation returned to the user
#[derive(Debug, Clone)]
pub struct MovieRecommendation {
    pub movie_id: MovieId,
    pub title: String,
    pub genres: Vec<String>,
    pub year: Option<u16>,
    pub score: f32,
    pub source: CandidateSource,
    pub explanation: String,
}

/// Main orchestrator that coordinates the recommendation pipeline
#[derive(Clone)]
pub struct RecommendationOrchestrator {
    data_index: Arc<DataIndex>,
    thunder: ThunderSource,
    phoenix: PhoenixSource,
    filter_pipeline: Arc<FilterPipeline>,
    feature_engineer: FeatureEngineer,
    ml_client: MLScorerClient,
}

impl RecommendationOrchestrator {
    /// Create a new orchestrator with all components initialized
    ///
    /// # Arguments
    /// * `data_index` - Shared reference to the data index
    /// * `ml_service_addr` - Address of the Python ML service (e.g., "http://localhost:50051")
    /// 1. Create ThunderSource::new(data_index.clone())
    /// 2. Create PhoenixSource::new(data_index.clone())
    /// 3. Create FilterPipeline and add filters:
    ///    - AlreadyWatchedFilter
    ///    - MinimumRatingFilter (min_rating: 3.5, min_count: 10)
    ///    - GenrePreferenceFilter (top_n: 3)
    /// 4. Create FeatureEngineer::new(data_index.clone())
    /// 5. Connect to ML service: MLScorerClient::connect(ml_service_addr).await?
    /// 6. Return the orchestrator
    ///
    /// Note: This is an async function because connecting to ML service requires await
    pub async fn new(
        data_index: Arc<DataIndex>,
        ml_service_addr: impl Into<String>,
    ) -> Result<Self> {
        let thunder = ThunderSource::new(data_index.clone());
        let phoenix = PhoenixSource::new(data_index.clone());
        let filter_pipeline = Arc::new(
            FilterPipeline::new()
                .add_filter(AlreadyWatchedFilter)
                .add_filter(MinimumRatingFilter::new(data_index.clone(), 3.5, 10))
                .add_filter(GenrePreferenceFilter::new(data_index.clone(), 3))
        );
        let feature_engineer = FeatureEngineer::new(data_index.clone());
        let ml_client = MLScorerClient::connect(ml_service_addr).await?;
        Ok(Self {
            data_index,
            thunder,
            phoenix,
            filter_pipeline,
            feature_engineer,
            ml_client,
        })
    }

    /// Main entry point: Get recommendations for a user
    ///
    /// # Arguments
    /// * `user_id` - The user to generate recommendations for
    /// * `limit` - Number of recommendations to return (e.g., 20)
    ///
    /// # Returns
    /// Vector of MovieRecommendation sorted by score (highest first)
    pub async fn get_recommendations(
        &self,
        user_id: UserId,
        limit: usize,
    ) -> Result<Vec<MovieRecommendation>> {
        // Start timing
        let start_time = Instant::now();

        // Build user context
        let context = self.build_user_context(user_id)?;
        info!("Built user context for user {}", user_id);

        // Generate candidates in parallel
        let (thunder_candidates, phoenix_candidates) = 
            self.generate_candidates_parallel(&context).await?;
        info!(
            "Generated {} thunder candidates and {} phoenix candidates",
            thunder_candidates.len(),
            phoenix_candidates.len()
        );

        // Merge candidates
        let merged_candidates = self.merge_candidates(thunder_candidates, phoenix_candidates);
        info!(
            "Merged candidates, total after deduplication: {}",
            merged_candidates.len()
        );

        // Apply filters
        let filtered_candidates = self.apply_filters(merged_candidates, &context)?;
        info!(
            "Applied filters, candidates remaining: {}",
            filtered_candidates.len()
        );

        // Compute features
        let features = self.compute_features(&filtered_candidates, &context);
        info!(
            "Computed features for {} candidates",
            features.len()
        );

        // Score with ML
        let scores = self.score_with_ml(user_id, &features).await?;
        info!(
            "Scored {} candidates with ML service",
            scores.len()
        );

        // Rank and select top N
        let recommendations = self.rank_and_select(
            filtered_candidates,
            features,
            scores,
            limit,
        )?;
        info!(
            "Selected top {} recommendations for user {}",
            recommendations.len(),
            user_id
        );

        // Log total time
        let elapsed = start_time.elapsed();
        info!(
            "Total time to get recommendations for user {}: {:.2?}",
            user_id,
            elapsed
        );
        Ok(recommendations)
    }

    /// Build user context from the data index
    /// 1. Use sources::user_context::build_user_context(&self.data_index, user_id)?
    /// 2. Add error context if user not found
    /// 3. Return the UserContext
    fn build_user_context(&self, user_id: UserId) -> Result<UserContext> {
        sources::user_context::build_user_context(&self.data_index, user_id)
            .context("Failed to build user context")
    }

    /// Generate candidates from Thunder and Phoenix in parallel
    async fn generate_candidates_parallel(
        &self,
        context: &UserContext,
    ) -> Result<(Vec<Candidate>, Vec<Candidate>)> {
        // Use tokio::join! to run both candidate sources in parallel
        let (thunder_result, phoenix_result) = tokio::join!(
            tokio::task::spawn_blocking({
                let thunder = self.thunder.clone();
                let context = context.clone();
                move || thunder.get_candidates(&context, 300)
            }),
            tokio::task::spawn_blocking({
                let phoenix = self.phoenix.clone();
                let context = context.clone();
                move || phoenix.get_candidates(&context, 200)
            })
        );

        // Unwrap the spawn_blocking results, then the inner Vec<Candidate>
        let thunder_candidates = thunder_result.context("Thunder task panicked")?;
        let phoenix_candidates = phoenix_result.context("Phoenix task panicked")?;
        Ok((thunder_candidates, phoenix_candidates))
    }

    /// Merge candidates from both sources and deduplicate by MovieId
    fn merge_candidates(
        &self,
        thunder_candidates: Vec<Candidate>,
        phoenix_candidates: Vec<Candidate>,
    ) -> Vec<Candidate> {

        // Use a HashMap to deduplicate candidates by MovieId
        let mut map: HashMap<MovieId, Candidate> = HashMap::new();
        
        // Preserve lengths
        let thunder_len = thunder_candidates.len();
        let phoenix_len = phoenix_candidates.len();

        // Process thunder candidates
        for candidate in thunder_candidates {
            map.entry(candidate.movie_id).and_modify(|existing| {
                if candidate.base_score > existing.base_score {
                    *existing = candidate.clone();
                }
            }).or_insert(candidate);
        }

        // Process phoenix candidates
        for candidate in phoenix_candidates {
            map.entry(candidate.movie_id).and_modify(|existing| {
                if candidate.base_score > existing.base_score {
                    *existing = candidate.clone();
                }
            }).or_insert(candidate);
        }

        // Convert the map values into a Vec<Candidate>
        let merged_candidates: Vec<Candidate> = map.into_values().collect();

        // Log statistics
        info!(
            "Merged candidates: thunder={}, phoenix={}, total_after_dedup={}",
            thunder_len,
            phoenix_len,
            merged_candidates.len()
        );

        merged_candidates
    }

    /// Apply the filter pipeline to candidates
    fn apply_filters(
        &self,
        candidates: Vec<Candidate>,
        context: &UserContext,
    ) -> Result<Vec<Candidate>> {
        info!(
            "Applying filters to {} candidates",
            candidates.len()
        );
        // Call the filter pipeline
        let filtered_candidates = self.filter_pipeline.apply(candidates, context)
            .context("Failed to apply filters")?;
        info!(
            "Filtering complete, {} candidates remain",
            filtered_candidates.len()
        );
        Ok(filtered_candidates)
    }

    /// Compute features for all candidates
    fn compute_features(
        &self,
        candidates: &[Candidate],
        context: &UserContext,
    ) -> Vec<pipeline::CandidateFeatures> {
        // Call self.feature_engineer.compute_features(&candidates, context)
        let features = self.feature_engineer.compute_features(candidates, context);

        // Verify lengths match
        if features.len() != candidates.len() {
            warn!(
                "Feature computation mismatch: candidates={}, features={}",
                candidates.len(),
                features.len()
            );
        } else {
            info!(
                "Computed features for {} candidates",
                features.len()
            );
        }

        features
    }

    /// Score candidates using the ML service
    async fn score_with_ml(
        &self,
        user_id: UserId,
        features: &[pipeline::CandidateFeatures],
    ) -> Result<Vec<f32>> {
        // Convert pipeline::CandidateFeatures to ml_client::recommendations::CandidateFeatures
        let proto_features: Vec<ml_client::recommendations::CandidateFeatures> = features
            .iter()
            .map(|f| ml_client::recommendations::CandidateFeatures {
                movie_id: f.movie_id,
                genre_overlap_score: f.genre_overlap_score,
                genre_diversity_score: f.genre_diversity_score,
                collaborative_score: f.collaborative_score,
                similar_users_count: f.similar_users_count,
                avg_rating: f.avg_rating,
                rating_count: f.rating_count,
                popularity_percentile: f.popularity_percentile,
                movie_year: f.movie_year.map(|y| y as u32),
                year_preference_score: f.year_preference_score,
                days_since_released: f.days_since_released,
            })
            .collect();
        // Call the ML client to get scores
        let scores = self.ml_client.score_candidates(user_id, proto_features).await?;

        // Return the scores
        Ok(scores)
    }

    /// Rank candidates by score and select top N
    fn rank_and_select(
        &self,
        candidates: Vec<Candidate>,
        _features: Vec<pipeline::CandidateFeatures>,
        scores: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MovieRecommendation>> {
        // Combine candidates, features, and scores into a single struct for sorting
        let mut scored: Vec<(Candidate, f32)> = candidates
            .into_iter()
            .zip(scores)
            .collect();

        // Sort by score DESC
        scored.sort_by(
            |a, b| 
            b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal));
        
        // Take top N
        scored.truncate(limit);

        // Convert to MovieRecommendation
        let recommendations: Vec<MovieRecommendation> = scored
            .into_iter()
            .filter_map(|(candidate, score)| {
                let movie = self.data_index.get_movie(candidate.movie_id)?;
                Some(MovieRecommendation {
                    movie_id: candidate.movie_id,
                    title: movie.title.clone(),
                    genres: movie.genres.iter().map(|g| format!("{:?}", g)).collect(),
                    year: movie.year,
                    score,
                    source: candidate.source,
                    explanation: format!("Score: {:.2}, Source: {:?}", score, candidate.source),
                })
            })
            .collect();

        Ok(recommendations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_loader::{Genre, Movie, User, Rating, AgeGroup, Gender, Occupation};
    use ml_client::recommendations::ml_scorer_server::{MlScorer, MlScorerServer};
    use ml_client::recommendations::{ScoreRequest, ScoreResponse};
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::{Request, Response, Status};
    use tonic::transport::Server;

    // ============================================================================
    // Test Fixtures
    // ============================================================================

    /// Create a minimal test DataIndex with a few movies, users, and ratings
    fn build_test_data_index() -> Arc<DataIndex> {
        let mut data_index = DataIndex::new();

        // Add test movies
        data_index.insert_movie(Movie {
            id: 1,
            title: "The Matrix (1999)".to_string(),
            year: Some(1999),
            genres: vec![Genre::Action, Genre::SciFi],
        });
        data_index.insert_movie(Movie {
            id: 2,
            title: "Toy Story (1995)".to_string(),
            year: Some(1995),
            genres: vec![Genre::Animation, Genre::Comedy, Genre::Children],
        });
        data_index.insert_movie(Movie {
            id: 3,
            title: "Pulp Fiction (1994)".to_string(),
            year: Some(1994),
            genres: vec![Genre::Crime, Genre::Drama],
        });
        data_index.insert_movie(Movie {
            id: 4,
            title: "Forrest Gump (1994)".to_string(),
            year: Some(1994),
            genres: vec![Genre::Drama, Genre::Romance],
        });
        data_index.insert_movie(Movie {
            id: 5,
            title: "The Shawshank Redemption (1994)".to_string(),
            year: Some(1994),
            genres: vec![Genre::Drama],
        });

        // Add test users
        data_index.insert_user(User {
            id: 1,
            gender: Gender::Male,
            age: AgeGroup::Age25To34,
            occupation: Occupation::Technician,
            zipcode: "94043".to_string(),
        });
        data_index.insert_user(User {
            id: 2,
            gender: Gender::Female,
            age: AgeGroup::Age18To24,
            occupation: Occupation::CollegeStudent,
            zipcode: "02139".to_string(),
        });

        // Add test ratings
        data_index.insert_rating(Rating {
            user_id: 1,
            movie_id: 1,
            rating: 5.0,
            timestamp: 978300760,
        });
        data_index.insert_rating(Rating {
            user_id: 1,
            movie_id: 2,
            rating: 4.0,
            timestamp: 978300761,
        });
        data_index.insert_rating(Rating {
            user_id: 1,
            movie_id: 3,
            rating: 5.0,
            timestamp: 978300762,
        });

        Arc::new(data_index)
    }

    // ============================================================================
    // Mock ML Service
    // ============================================================================

    /// Mock ML scorer that returns deterministic scores for testing
    #[derive(Default)]
    struct MockMlScorer;

    #[tonic::async_trait]
    impl MlScorer for MockMlScorer {
        async fn score_candidates(
            &self,
            request: Request<ScoreRequest>,
        ) -> Result<Response<ScoreResponse>, Status> {
            let features = request.get_ref().features.clone();

            // Return scores based on movie_id for determinism
            // Higher movie_id = higher score (for testing purposes)
            let scores: Vec<f32> = features
                .iter()
                .map(|f| {
                    // Simple scoring: movie_id * 0.1 + base features
                    let base = f.movie_id as f32 * 0.1;
                    let feature_score = f.genre_overlap_score * 0.3
                        + f.collaborative_score * 0.3
                        + f.popularity_percentile * 0.2;
                    (base + feature_score).min(1.0)
                })
                .collect();

            Ok(Response::new(ScoreResponse { scores }))
        }
    }

    /// Start a mock ML service on a random port
    async fn start_mock_ml_service() -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind mock ML service");

        let addr = listener.local_addr().expect("Failed to get local address");
        let service = MlScorerServer::new(MockMlScorer::default());

        let handle = tokio::spawn(async move {
            Server::builder()
                .add_service(service)
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await
                .expect("Mock ML service failed");
        });

        (format!("http://{}", addr), handle)
    }

    /// Create an orchestrator for testing with mock ML service
    async fn build_test_orchestrator() -> (RecommendationOrchestrator, tokio::task::JoinHandle<()>) {
        let data_index = build_test_data_index();
        let (addr, handle) = start_mock_ml_service().await;

        let orchestrator = RecommendationOrchestrator::new(data_index, addr)
            .await
            .expect("Failed to create orchestrator");

        (orchestrator, handle)
    }

    // ============================================================================
    // Unit Tests: merge_candidates
    // ============================================================================

    #[tokio::test]
    async fn test_merge_candidates_deduplicates_by_movie_id() {
        let (orchestrator, handle) = build_test_orchestrator().await;

        let thunder = vec![
            Candidate::new(1, CandidateSource::Thunder, 0.8),
            Candidate::new(2, CandidateSource::Thunder, 0.7),
            Candidate::new(3, CandidateSource::Thunder, 0.6),
        ];
        let phoenix = vec![
            Candidate::new(1, CandidateSource::Phoenix, 0.5), // Duplicate, lower score
            Candidate::new(4, CandidateSource::Phoenix, 0.9),
            Candidate::new(5, CandidateSource::Phoenix, 0.4),
        ];

        let merged = orchestrator.merge_candidates(thunder, phoenix);

        // Should have 5 unique movies
        assert_eq!(merged.len(), 5, "Should have 5 unique movies after merge");

        // Verify movie 1 kept the higher score from Thunder
        let movie_1 = merged.iter().find(|c| c.movie_id == 1).expect("Movie 1 should exist");
        assert_eq!(movie_1.base_score, 0.8, "Should keep higher score from Thunder");
        assert_eq!(movie_1.source, CandidateSource::Thunder, "Should keep Thunder source");

        handle.abort();
    }

    #[tokio::test]
    async fn test_merge_candidates_keeps_highest_score_on_duplicate() {
        let (orchestrator, handle) = build_test_orchestrator().await;

        let thunder = vec![
            Candidate::new(1, CandidateSource::Thunder, 0.3),
        ];
        let phoenix = vec![
            Candidate::new(1, CandidateSource::Phoenix, 0.9), // Higher score
        ];

        let merged = orchestrator.merge_candidates(thunder, phoenix);

        assert_eq!(merged.len(), 1, "Should have 1 movie");
        let movie = &merged[0];
        assert_eq!(movie.base_score, 0.9, "Should keep higher Phoenix score");
        assert_eq!(movie.source, CandidateSource::Phoenix, "Should keep Phoenix source");

        handle.abort();
    }

    #[tokio::test]
    async fn test_merge_candidates_handles_empty_inputs() {
        let (orchestrator, handle) = build_test_orchestrator().await;

        // Both empty
        let merged = orchestrator.merge_candidates(vec![], vec![]);
        assert_eq!(merged.len(), 0, "Empty inputs should return empty result");

        // Only thunder
        let thunder = vec![Candidate::new(1, CandidateSource::Thunder, 0.5)];
        let merged = orchestrator.merge_candidates(thunder, vec![]);
        assert_eq!(merged.len(), 1, "Should handle phoenix empty");

        // Only phoenix
        let phoenix = vec![Candidate::new(2, CandidateSource::Phoenix, 0.7)];
        let merged = orchestrator.merge_candidates(vec![], phoenix);
        assert_eq!(merged.len(), 1, "Should handle thunder empty");

        handle.abort();
    }

    #[tokio::test]
    async fn test_merge_candidates_preserves_all_unique_movies() {
        let (orchestrator, handle) = build_test_orchestrator().await;

        let thunder = vec![
            Candidate::new(1, CandidateSource::Thunder, 0.5),
            Candidate::new(2, CandidateSource::Thunder, 0.6),
        ];
        let phoenix = vec![
            Candidate::new(3, CandidateSource::Phoenix, 0.7),
            Candidate::new(4, CandidateSource::Phoenix, 0.8),
        ];

        let merged = orchestrator.merge_candidates(thunder, phoenix);

        assert_eq!(merged.len(), 4, "Should have all 4 unique movies");

        // Verify all movie IDs are present
        let movie_ids: Vec<_> = merged.iter().map(|c| c.movie_id).collect();
        assert!(movie_ids.contains(&1));
        assert!(movie_ids.contains(&2));
        assert!(movie_ids.contains(&3));
        assert!(movie_ids.contains(&4));

        handle.abort();
    }

    // ============================================================================
    // Unit Tests: rank_and_select
    // ============================================================================

    #[tokio::test]
    async fn test_rank_and_select_sorts_by_score_descending() {
        let (orchestrator, handle) = build_test_orchestrator().await;

        let candidates = vec![
            Candidate::new(1, CandidateSource::Thunder, 0.1),
            Candidate::new(2, CandidateSource::Phoenix, 0.5),
            Candidate::new(3, CandidateSource::Thunder, 0.3),
        ];
        let scores = vec![0.2, 0.9, 0.5];

        let recommendations = orchestrator
            .rank_and_select(candidates, vec![], scores, 10)
            .expect("rank_and_select failed");

        // Should be sorted by score: movie 2 (0.9), movie 3 (0.5), movie 1 (0.2)
        assert_eq!(recommendations.len(), 3);
        assert_eq!(recommendations[0].movie_id, 2, "Highest score should be first");
        assert_eq!(recommendations[0].score, 0.9);
        assert_eq!(recommendations[1].movie_id, 3, "Second highest score");
        assert_eq!(recommendations[1].score, 0.5);
        assert_eq!(recommendations[2].movie_id, 1, "Lowest score should be last");
        assert_eq!(recommendations[2].score, 0.2);

        handle.abort();
    }

    #[tokio::test]
    async fn test_rank_and_select_truncates_to_limit() {
        let (orchestrator, handle) = build_test_orchestrator().await;

        let candidates = vec![
            Candidate::new(1, CandidateSource::Thunder, 0.1),
            Candidate::new(2, CandidateSource::Phoenix, 0.2),
            Candidate::new(3, CandidateSource::Thunder, 0.3),
            Candidate::new(4, CandidateSource::Phoenix, 0.4),
            Candidate::new(5, CandidateSource::Thunder, 0.5),
        ];
        let scores = vec![0.1, 0.2, 0.3, 0.4, 0.5];

        let recommendations = orchestrator
            .rank_and_select(candidates, vec![], scores, 3)
            .expect("rank_and_select failed");

        // Should only return top 3
        assert_eq!(recommendations.len(), 3, "Should truncate to limit of 3");
        assert_eq!(recommendations[0].movie_id, 5);
        assert_eq!(recommendations[1].movie_id, 4);
        assert_eq!(recommendations[2].movie_id, 3);

        handle.abort();
    }

    #[tokio::test]
    async fn test_rank_and_select_enriches_with_movie_metadata() {
        let (orchestrator, handle) = build_test_orchestrator().await;

        let candidates = vec![
            Candidate::new(1, CandidateSource::Thunder, 0.5),
        ];
        let scores = vec![0.8];

        let recommendations = orchestrator
            .rank_and_select(candidates, vec![], scores, 10)
            .expect("rank_and_select failed");

        assert_eq!(recommendations.len(), 1);
        let rec = &recommendations[0];

        // Verify movie metadata was enriched
        assert_eq!(rec.movie_id, 1);
        assert_eq!(rec.title, "The Matrix (1999)");
        assert_eq!(rec.year, Some(1999));
        assert!(rec.genres.contains(&"Action".to_string())
                || rec.genres.iter().any(|g| g.contains("Action")));
        assert_eq!(rec.score, 0.8);
        assert_eq!(rec.source, CandidateSource::Thunder);
        assert!(rec.explanation.contains("Score"));

        handle.abort();
    }

    #[tokio::test]
    async fn test_rank_and_select_filters_missing_movies() {
        let (orchestrator, handle) = build_test_orchestrator().await;

        let candidates = vec![
            Candidate::new(1, CandidateSource::Thunder, 0.5),
            Candidate::new(999, CandidateSource::Phoenix, 0.9), // Doesn't exist
            Candidate::new(2, CandidateSource::Thunder, 0.3),
        ];
        let scores = vec![0.5, 0.9, 0.3];

        let recommendations = orchestrator
            .rank_and_select(candidates, vec![], scores, 10)
            .expect("rank_and_select failed");

        // Should only return 2 movies (1 and 2), skipping 999
        assert_eq!(recommendations.len(), 2, "Should filter out missing movie 999");
        assert_eq!(recommendations[0].movie_id, 1);
        assert_eq!(recommendations[1].movie_id, 2);

        handle.abort();
    }

    #[tokio::test]
    async fn test_rank_and_select_handles_empty_input() {
        let (orchestrator, handle) = build_test_orchestrator().await;

        let recommendations = orchestrator
            .rank_and_select(vec![], vec![], vec![], 10)
            .expect("rank_and_select failed");

        assert_eq!(recommendations.len(), 0, "Empty input should return empty result");

        handle.abort();
    }

    #[tokio::test]
    async fn test_rank_and_select_handles_nan_scores() {
        let (orchestrator, handle) = build_test_orchestrator().await;

        let candidates = vec![
            Candidate::new(1, CandidateSource::Thunder, 0.5),
            Candidate::new(2, CandidateSource::Phoenix, 0.3),
        ];
        let scores = vec![f32::NAN, 0.8];

        let recommendations = orchestrator
            .rank_and_select(candidates, vec![], scores, 10)
            .expect("rank_and_select failed");

        // NaN should be handled gracefully (doesn't panic)
        // With unwrap_or(Equal), NaN order is undefined, so we just check both are present
        assert_eq!(recommendations.len(), 2, "Both movies should be present");

        // Verify both movie IDs are in the result (order may vary with NaN)
        let movie_ids: Vec<_> = recommendations.iter().map(|r| r.movie_id).collect();
        assert!(movie_ids.contains(&1), "Movie 1 should be present");
        assert!(movie_ids.contains(&2), "Movie 2 should be present");

        handle.abort();
    }

    // ============================================================================
    // Integration Tests
    // ============================================================================

    #[tokio::test]
    async fn test_orchestrator_construction() {
        let data_index = build_test_data_index();
        let (addr, handle) = start_mock_ml_service().await;

        let result = RecommendationOrchestrator::new(data_index, addr).await;
        assert!(result.is_ok(), "Orchestrator construction should succeed");

        handle.abort();
    }

    #[tokio::test]
    async fn test_build_user_context_for_valid_user() {
        let (orchestrator, handle) = build_test_orchestrator().await;

        let result = orchestrator.build_user_context(1);
        assert!(result.is_ok(), "Should build context for valid user");

        let context = result.unwrap();
        assert_eq!(context.user_id, 1);

        handle.abort();
    }

    #[tokio::test]
    async fn test_build_user_context_for_missing_user() {
        let (orchestrator, handle) = build_test_orchestrator().await;

        let result = orchestrator.build_user_context(9999);
        assert!(result.is_err(), "Should fail for missing user");

        handle.abort();
    }
}
