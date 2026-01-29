//! ML scoring client for communicating with the Python gRPC service.
//!
//! This crate provides a Rust client to call the Python ML scoring service
//! over gRPC. It handles:
//! - Connection management to the Python service
//! - Converting Rust types to protobuf messages
//! - Sending candidate features and receiving scores
//! - Error handling and retries

use anyhow::{Context, Result};
use thiserror::Error;
use tonic::transport::Channel;
use tracing::{debug, error, info};

// Include the generated protobuf code
pub mod recommendations {
    tonic::include_proto!("recommendations");
}

use recommendations::{
    ml_scorer_client::MlScorerClient as GrpcMlScorerClient,
    CandidateFeatures,
    ScoreRequest,
};

/// Errors that can occur when interacting with the ML service
#[derive(Error, Debug)]
pub enum MLClientError {
    #[error("Failed to connect to ML service: {0}")]
    ConnectionError(String),

    #[error("Failed to score candidates: {0}")]
    ScoringError(String),

    #[error("Invalid response from ML service: {0}")]
    InvalidResponse(String),
}

/// Client for the ML scoring service.
///
/// This wraps the auto-generated gRPC client and provides a higher-level
/// interface for scoring candidates.
pub struct MLScorerClient {
    client: GrpcMlScorerClient<Channel>,
    service_addr: String,
}

impl MLScorerClient {
    /// Connect to the ML scoring service.
    ///
    /// # Arguments
    /// * `addr` - Address of the gRPC service (e.g., "http://localhost:50051")
    ///
    /// # Returns
    /// A connected client ready to score candidates
    /// 1. Use tonic::transport::Channel::from_shared(addr)?
    /// 2. Call .connect().await to establish connection
    /// 3. Create GrpcMlScorerClient::new(channel)
    /// 4. Return MLScorerClient { client, service_addr }
    ///
    /// Error handling:
    /// - Handle connection errors gracefully
    /// - Log connection attempts
    /// - Consider adding retry logic (optional)
    pub async fn connect(addr: impl Into<String>) -> Result<Self> {
        let addr = addr.into();
        info!("Connecting to ML service at {}", addr);
        
        let channel = Channel::from_shared(addr.clone())
            .context("Creating channel from address")?
            .connect()
            .await
            .context("Connecting to ML service")?;

        let client = GrpcMlScorerClient::new(channel);
        Ok(MLScorerClient {
            client,
            service_addr: addr,
        })
    }

    /// Score a batch of candidates for a given user.
    ///
    /// # Arguments
    /// * `user_id` - The user ID to score candidates for
    /// * `features` - Vector of CandidateFeatures to score
    ///
    /// # Returns
    /// Vector of scores (f32) in the same order as the input features
    ///
    /// # TODO: Implement this method
    /// Steps:
    /// 1. Create a ScoreRequest with user_id and features
    /// 2. Call self.client.score_candidates(request).await
    /// 3. Extract the scores from the response
    /// 4. Validate that scores.len() == features.len()
    /// 5. Return the scores
    ///
    /// Error handling:
    /// - Handle gRPC errors (service down, timeout, etc.)
    /// - Validate response (correct number of scores)
    /// - Log errors with context
    pub async fn score_candidates(
        &mut self,
        user_id: u32,
        features: Vec<CandidateFeatures>,
    ) -> Result<Vec<f32>, MLClientError> {
        let expected_len = features.len();
        debug!(
            "Scoring {} candidates for user {}",
            expected_len,
            user_id
        );
        let request = tonic::Request::new(ScoreRequest {
            user_id,
            features,
        });

        let response = self.client.score_candidates(request).await.map_err(|e| {
            error!("gRPC error while scoring candidates: {}", e);
            MLClientError::ScoringError(e.to_string())
        })?;
        
        let scores = response.into_inner().scores;

        if scores.len() != expected_len {
            error!(
                "Mismatch in number of scores returned: expected {}, got {}",
                expected_len,
                scores.len()
            );
            return Err(MLClientError::InvalidResponse(
                "Number of scores does not match number of features".into(),
            ));
        }
        Ok(scores)  
    }

    /// Get the address of the ML service this client is connected to.
    pub fn service_address(&self) -> &str {
        &self.service_addr
    }
}

// CandidateFeatures is already re-exported from the use statement above

/// Helper function to create CandidateFeatures (for testing/examples)
///
/// You might want to create a builder or conversion trait for this in the
/// pipeline crate instead of constructing these manually.
pub fn create_candidate_features(
    movie_id: u32,
    genre_overlap_score: f32,
    genre_diversity_score: f32,
    collaborative_score: f32,
    similar_users_count: u32,
    avg_rating: f32,
    rating_count: u32,
    popularity_percentile: f32,
    movie_year: Option<u32>,
    year_preference_score: f32,
    days_since_released: f32,
) -> CandidateFeatures {
    CandidateFeatures {
        movie_id,
        genre_overlap_score,
        genre_diversity_score,
        collaborative_score,
        similar_users_count,
        avg_rating,
        rating_count,
        popularity_percentile,
        movie_year,
        year_preference_score,
        days_since_released,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_candidate_features() {
        let features = create_candidate_features(
            1234,      // movie_id
            0.8,       // genre_overlap_score
            0.5,       // genre_diversity_score
            0.9,       // collaborative_score
            42,        // similar_users_count
            4.2,       // avg_rating
            1500,      // rating_count
            0.85,      // popularity_percentile
            Some(1999), // movie_year
            0.7,       // year_preference_score
            8765.0,    // days_since_released
        );

        assert_eq!(features.movie_id, 1234);
        assert_eq!(features.avg_rating, 4.2);
    }

    // TODO: Add integration tests
    // You can add tests that connect to a running Python service:
    //
    #[tokio::test]
    async fn test_score_candidates_integration() {
        let mut client = MLScorerClient::connect("http://localhost:50051")
            .await
            .expect("Failed to connect");
    
        let features = vec![create_candidate_features(
            1234, 0.8, 0.5, 0.9, 42, 4.2, 1500, 0.85, Some(1999), 0.7, 8765.0,
        )];
        let scores = client.score_candidates(1, features)
            .await
            .expect("Failed to score");
    
        assert_eq!(scores.len(), 1);
        assert!(scores[0] >= 0.0 && scores[0] <= 1.0);
    }
}
