"""ML model wrapper for collaborative filtering."""

import joblib
import numpy as np
from pathlib import Path
from typing import Optional
from sklearn.neighbors import NearestNeighbors


class CollaborativeFilteringModel:
    """Wrapper for the collaborative filtering model used in recommendations.

    This class handles:
    - Loading the trained sklearn NearestNeighbors model
    - Loading the user-item matrix
    - Computing collaborative filtering scores for candidates
    """

    def __init__(self, model_path: Path, matrix_path: Path):
        """Initialize the model wrapper.

        Args:
            model_path: Path to the saved sklearn model (.pkl)
            matrix_path: Path to the saved user-item matrix (.pkl)
        """
        self.model_path = model_path
        self.matrix_path = matrix_path
        self.model: Optional[NearestNeighbors] = None
        self.user_item_matrix: Optional[np.ndarray] = None
        self.user_id_to_idx: Optional[dict] = None  # Maps user_id -> matrix row index
        self.movie_id_to_idx: Optional[dict] = None  # Maps movie_id -> matrix column index

    def load(self) -> None:
        """Load the model and user-item matrix from disk.

        - Load the sklearn model using joblib
        - Load the user-item matrix using joblib
        - Load the movie_id_to_idx mapping if saved separately

        Raises:
            FileNotFoundError: If model files don't exist
        """
        self.model = joblib.load(self.model_path)
        self.user_item_matrix = joblib.load(self.matrix_path)
        # Assuming movie_id_to_idx is saved alongside the matrix
        movie_id_to_idx_path = self.matrix_path.parent / "movie_id_to_idx.pkl"
        self.movie_id_to_idx = joblib.load(movie_id_to_idx_path)
        user_id_to_idx_path = self.matrix_path.parent / "user_id_to_idx.pkl"
        self.user_id_to_idx = joblib.load(user_id_to_idx_path)

    def compute_cf_score(self, user_id: int, movie_id: int) -> float:
        """Compute collaborative filtering score for a single movie.

        This should use the NearestNeighbors model to find similar users
        and predict how likely this user is to rate the movie highly.

        Args:
            user_id: The user ID (maps to a row in user_item_matrix)
            movie_id: The movie ID (needs to be mapped to matrix column)

        Returns:
            Float score between 0.0 and 1.0 indicating predicted affinity

        - Get the user's rating vector from user_item_matrix
        - Use the model to find k nearest neighbors
        - Compute the predicted rating based on neighbors' ratings
        - Normalize to 0-1 range
        - Handle cases where movie_id is not in the matrix (return 0.0?)
        """
        # Get user's rating vector
        if self.user_item_matrix is None or self.movie_id_to_idx is None or self.model is None:
            raise ValueError("Model and matrix must be loaded before scoring.")
        user_idx = self.user_id_to_idx.get(user_id, None)
        if user_idx is None or user_idx >= self.user_item_matrix.shape[0]:
            return 0.0  # User not in matrix
        user_vector = self.user_item_matrix[user_idx, :].toarray().reshape(1, -1)

        # Find nearest neighbors
        distances, indices = self.model.kneighbors(user_vector)
        distances = distances.flatten()

        # Get movie ratings from neighbors
        movie_idx = self.movie_id_to_idx.get(movie_id, None)
        if movie_idx is None:
            return 0.0  # Movie not in matrix
        
        neighbor_ratings = self.user_item_matrix[indices.flatten(), movie_idx].toarray().flatten()

        # Filter valid ratings and their corresponding distances
        valid_mask = neighbor_ratings > 0
        valid_ratings = neighbor_ratings[valid_mask]
        valid_distances = distances[valid_mask]
        if len(valid_ratings) == 0:
            return 0.0  # No valid ratings from neighbors
        
        # Convert cosine distance to similarity weights
        # similarity = 1 - distance (ranges from 1 to -1, but usually 0 to 1)
        similarities = 1 - valid_distances

        # Handle edge case: if all similarities are 0 or negative
        if similarities.sum() <= 0:
            # Fall back to simple average
            predicted_rating = np.mean(valid_ratings)
        else:
            # Weighted average by similarity
            predicted_rating = np.average(valid_ratings, weights=similarities)
        
        # Normalize to 0-1 range
        normalized_score = (predicted_rating - 1) / 4  # Assuming ratings are from 1 to 5
        return normalized_score

    def batch_score(self, user_id: int, movie_ids: list[int]) -> np.ndarray:
        """Vectorized batch scoring."""
        if self.user_item_matrix is None or self.movie_id_to_idx is None:
            raise ValueError("Model must be loaded first")
        
        user_idx = self.user_id_to_idx.get(user_id)
        if user_idx is None:
            return np.zeros(len(movie_ids))
        
        user_vector = self.user_item_matrix[user_idx, :].toarray().reshape(1, -1)
        
        # Find neighbors once
        distances, indices = self.model.kneighbors(user_vector)
        neighbor_indices = indices.flatten()
        distances = distances.flatten()
        
        # Convert distances to similarities
        similarities = 1 - distances
        
        scores = []
        for movie_id in movie_ids:
            movie_idx = self.movie_id_to_idx.get(movie_id)
            if movie_idx is None:
                scores.append(0.0)
                continue
            
            neighbor_ratings = self.user_item_matrix[neighbor_indices, movie_idx].toarray().flatten()
            
            # Filter valid ratings and corresponding similarities
            valid_mask = neighbor_ratings > 0
            valid_ratings = neighbor_ratings[valid_mask]
            valid_similarities = similarities[valid_mask]
            
            if len(valid_ratings) == 0:
                scores.append(0.0)
            else:
                # Weighted average
                if valid_similarities.sum() <= 0:
                    predicted_rating = np.mean(valid_ratings)
                else:
                    predicted_rating = np.average(valid_ratings, weights=valid_similarities)
                
                normalized_score = (predicted_rating - 1) / 4
                scores.append(normalized_score)
        
        return np.array(scores)

# if __name__ == "__main__":
#     # Load the model
#     cf_model = CollaborativeFilteringModel(
#         Path("models/collaborative_filtering/cf_model.pkl"),
#         Path("models/collaborative_filtering/user_item_matrix.pkl")
#     )
#     cf_model.load()

#     # Test scoring
#     score = cf_model.compute_cf_score(user_id=1, movie_id=1193)
#     print(f"CF Score: {score}")