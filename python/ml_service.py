"""gRPC service for ML-based candidate scoring.

This service receives candidate features from the Rust pipeline and returns
ML scores for ranking.
"""

import grpc
from concurrent import futures
from pathlib import Path
import logging
from grpc_reflection.v1alpha import reflection 

from generated import (
    MLScorerServicer,
    ScoreRequest,
    ScoreResponse,
    add_MLScorerServicer_to_server,
)
from model import CollaborativeFilteringModel


# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class MLScorerService(MLScorerServicer):
    """Implementation of the MLScorer gRPC service.

    This service:
    1. Loads the collaborative filtering model on startup
    2. Receives ScoreRequest with user_id and candidate features
    3. Computes ML scores combining CF and other features
    4. Returns scores array
    """

    def __init__(self, model_path: Path, matrix_path: Path):
        """Initialize the ML scoring service.

        Args:
            model_path: Path to the trained model file
            matrix_path: Path to the user-item matrix file
        """
        logger.info("Initializing ML Scorer Service...")
        self.cf_model = CollaborativeFilteringModel(model_path, matrix_path)

        # Load the model on initialization
        self.cf_model.load()    

        logger.info("✓ ML Scorer Service initialized")

    def ScoreCandidates(
        self,
        request: ScoreRequest,
        context: grpc.ServicerContext,
    ) -> ScoreResponse:
        """Score a batch of candidate movies for a user.

        Args:
            request: ScoreRequest containing user_id and list of CandidateFeatures
            context: gRPC context

        Returns:
            ScoreResponse containing a list of scores (one per candidate)

        Steps:
        1. Extract user_id and features list from request
        2. For each CandidateFeatures:
           a. Compute CF score using self.cf_model.compute_cf_score()
           b. Combine with other features using weighted formula
           c. Example weights (adjust as needed):
              - cf_score: 40%
              - genre_overlap_score: 25%
              - collaborative_score: 20%
              - popularity_percentile: 10%
              - year_preference_score: 5%
        3. Return ScoreResponse with scores list

        Error handling:
        - If scoring fails, log error and return default scores (e.g., 0.5)
        - Use context.abort() for critical errors
        """
        # Extract user_id and features
        try:
            user_id = request.user_id
            features_list = request.features

            logger.debug(f"Scoring {len(features_list)} candidates for user_id={user_id}")

            if len(features_list) == 0:
                logger.warning("Received empty features list")
                return ScoreResponse(scores=[])
            
            scores = []

            # Process each candidate
            for feature in features_list:
                try:
                    # Compute CF score
                    cf_score = self.cf_model.compute_cf_score(user_id, feature.movie_id)

                    # Combine scores with weights
                    final_score = (
                        0.4 * cf_score +
                        0.25 * feature.genre_overlap_score +
                        0.2 * feature.collaborative_score +
                        0.1 * feature.popularity_percentile +
                        0.05 * feature.year_preference_score
                    )
                    # Ensure score is within [0, 1]
                    final_score = max(0.0, min(1.0, final_score))
                    scores.append(final_score)
                except Exception as e:
                    # If scoring a single candidate fails, log and assign default score
                    logger.error(f"Error scoring candidate movie_id={feature.movie_id}: {e}")
                    scores.append(0.5)  # Default score on error
                
            logger.info(
                f"Successfully scored {len(scores)} candidates for user_id={user_id}"
                f"(avg: {sum(scores)/len(scores):.3f})"
            )
            return ScoreResponse(scores=scores)
        
        except Exception as e:
            logger.critical(f"Critical error in ScoreCandidates: {e}", exc_info=True)
            context.abort(grpc.StatusCode.INTERNAL, "Internal server error")

def serve(
    port: int = 50051,
    model_path: Path = Path("models/cf_model.pkl"),
    matrix_path: Path = Path("models/user_item_matrix.pkl"),
) -> None:
    """Start the gRPC server.

    Args:
        port: Port to listen on (default: 50051)
        model_path: Path to the trained model
        matrix_path: Path to the user-item matrix

    Steps:
    1. Create a gRPC server with thread pool executor
    2. Add the MLScorerService to the server
    3. Add insecure port (or secure port with TLS)
    4. Start the server
    5. Wait for termination (handle Ctrl+C gracefully)

    Example:
        server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
        add_MLScorerServicer_to_server(MLScorerService(...), server)
        server.add_insecure_port(f'[::]:{port}')
        server.start()
        server.wait_for_termination()
    """
    # Create gRPC server with thread pool
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))

    # Add the MLScorer service
    add_MLScorerServicer_to_server(
        MLScorerService(model_path, matrix_path), server
    )
    # Enable reflection (add these lines)
    SERVICE_NAMES = (
        "recommendations.MLScorer",
        reflection.SERVICE_NAME,
    )
    reflection.enable_server_reflection(SERVICE_NAMES, server)
    
    # Bind to port
    server.add_insecure_port(f'[::]:{port}')
    logger.info(f"Starting ML Scorer Service on port {port}...")
    server.start()
    logger.info(f"ML Scorer Service started on port {port}")

    try:
        # Keep server running
        server.wait_for_termination()
    except KeyboardInterrupt:
        logger.info("Shutting down ML Scorer Service...")
        server.stop(grace=5)
        logger.info("ML Scorer Service shut down")


if __name__ == "__main__":
    """Entry point for the ML service."""
    import argparse
    
    parser = argparse.ArgumentParser(description="ML Scoring Service")
    parser.add_argument(
        "--port",
        type=int,
        default=50051,
        help="Port to listen on (default: 50051)"
    )
    parser.add_argument(
        "--model-path",
        type=Path,
        default=Path("models/collaborative_filtering/cf_model.pkl"),
        help="Path to the trained model file"
    )
    parser.add_argument(
        "--matrix-path",
        type=Path,
        default=Path("models/collaborative_filtering/user_item_matrix.pkl"),
        help="Path to the user-item matrix file"
    )
    
    args = parser.parse_args()
    
    logger.info("Starting ML Scorer Service...")
    
    try:
        serve(
            port=args.port,
            model_path=args.model_path,
            matrix_path=args.matrix_path
        )
    except Exception as e:
        logger.error(f"Failed to start service: {e}", exc_info=True)
        exit(1)
