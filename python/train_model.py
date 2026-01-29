"""Script to train the collaborative filtering model.

This script:
1. Loads the MovieLens ratings data
2. Creates a user-item matrix (sparse or dense)
3. Trains a NearestNeighbors model for collaborative filtering
4. Saves the model and matrix to disk
"""

import pandas as pd
import numpy as np
from pathlib import Path
from sklearn.neighbors import NearestNeighbors
import joblib
from typing import Tuple, Dict
from scipy.sparse import lil_matrix


def load_ratings(data_dir: Path) -> pd.DataFrame:
    """Load ratings from the MovieLens dataset.

    Args:
        data_dir: Path to the ml-1m directory containing ratings.dat

    Returns:
        DataFrame with columns: user_id, movie_id, rating, timestamp

    TODO: Implement this method
    - Read ratings.dat file (delimiter is '::')
    - Parse into DataFrame
    - Column names: ['user_id', 'movie_id', 'rating', 'timestamp']
    - File format: userId::movieId::rating::timestamp
    - Encoding: ISO-8859-1 (or try 'latin-1')

    Hint: pd.read_csv() with sep='::', engine='python', encoding='latin-1'
    """
    df = pd.read_csv(
        data_dir / "ratings.dat",
        sep="::",
        engine="python",
        names=["user_id", "movie_id", "rating", "timestamp"],
        encoding="latin-1",
    )
    return df


def create_user_item_matrix(
    ratings_df: pd.DataFrame,
) -> Tuple[lil_matrix, Dict[int, int], Dict[int, int]]:
    """Create a user-item matrix from ratings.

    Args:
        ratings_df: DataFrame with user_id, movie_id, rating columns

    Returns:
        Tuple of:
        - user_item_matrix: numpy array of shape (n_users, n_movies)
        - user_id_to_idx: dict mapping user_id -> row index
        - movie_id_to_idx: dict mapping movie_id -> column index

    TODO: Implement this method
    Steps:
    1. Get unique user_ids and movie_ids
    2. Create mappings from id -> index (0-based)
    3. Create a matrix initialized with zeros or NaN
       - Shape: (num_users, num_movies)
       - You can use np.zeros() or a sparse matrix
    4. Fill in the ratings:
       - For each rating, find user_idx and movie_idx
       - Set matrix[user_idx, movie_idx] = rating
    5. Handle missing values (not all users rate all movies)
       - You might want to fill with 0 or leave as NaN

    Note: For 6,040 users and 3,883 movies, this is a 6040x3883 matrix
          which is manageable in memory (~180MB for float64)

    Alternative: Use scipy.sparse for memory efficiency
    """
    unique_user_ids = ratings_df["user_id"].unique()
    unique_movie_ids = ratings_df["movie_id"].unique()

    user_id_to_idx = {user_id: idx for idx, user_id in enumerate(unique_user_ids)}
    movie_id_to_idx = {movie_id: idx for idx, movie_id in enumerate(unique_movie_ids)}

    n_users = len(unique_user_ids)
    n_movies = len(unique_movie_ids)

    user_item_matrix = lil_matrix((n_users, n_movies), dtype=np.float32)

    # Map IDs to indices using vectorized operations
    user_indices = ratings_df["user_id"].map(user_id_to_idx).values
    movie_indices = ratings_df["movie_id"].map(movie_id_to_idx).values
    ratings = ratings_df["rating"].values

    # Fill the matrix
    user_item_matrix[user_indices, movie_indices] = ratings

    user_item_matrix = user_item_matrix.tocsr()  # Convert to CSR format for efficiency

    return user_item_matrix, user_id_to_idx, movie_id_to_idx


def train_collaborative_filtering_model(
    user_item_matrix: np.ndarray,
    n_neighbors: int = 20,
    metric: str = "cosine",
) -> NearestNeighbors:
    """Train a NearestNeighbors model for collaborative filtering.

    Args:
        user_item_matrix: User-item rating matrix
        n_neighbors: Number of nearest neighbors to find
        metric: Distance metric ('cosine', 'euclidean', etc.)

    Returns:
        Trained NearestNeighbors model

    TODO: Implement this method
    Steps:
    1. Create a NearestNeighbors instance with parameters
    2. Fit it on the user_item_matrix
       - The model learns the user similarity structure
    3. Return the fitted model

    Example:
        model = NearestNeighbors(n_neighbors=n_neighbors, metric=metric)
        model.fit(user_item_matrix)
        return model

    Note: You might want to normalize the matrix first (e.g., center by user mean)
    """

    model = NearestNeighbors(n_neighbors=n_neighbors, metric=metric)
    model.fit(user_item_matrix)
    return model



def save_model_artifacts(
    model: NearestNeighbors,
    user_item_matrix: np.ndarray,
    user_id_to_idx: Dict[int, int],
    movie_id_to_idx: Dict[int, int],
    output_dir: Path,
) -> None:
    """Save the trained model and associated data.

    Args:
        model: Trained NearestNeighbors model
        user_item_matrix: The user-item matrix
        user_id_to_idx: User ID to index mapping
        movie_id_to_idx: Movie ID to index mapping
        output_dir: Directory to save artifacts

    TODO: Implement this method
    Steps:
    1. Create output_dir if it doesn't exist
    2. Save the model: joblib.dump(model, output_dir / "cf_model.pkl")
    3. Save the matrix: joblib.dump(user_item_matrix, output_dir / "user_item_matrix.pkl")
    4. Save the mappings: joblib.dump(movie_id_to_idx, output_dir / "movie_id_to_idx.pkl")
    5. Optionally save user_id_to_idx if needed

    The model file will be loaded by the gRPC service.
    """
    output_dir.mkdir(parents=True, exist_ok=True)
    joblib.dump(model, output_dir / "cf_model.pkl")
    joblib.dump(user_item_matrix, output_dir / "user_item_matrix.pkl")
    joblib.dump(movie_id_to_idx, output_dir / "movie_id_to_idx.pkl")
    joblib.dump(user_id_to_idx, output_dir / "user_id_to_idx.pkl")


def main():
    """Main training pipeline.

    Steps:
    1. Define paths
       - data_dir: Path to ml-1m/ directory
       - output_dir: Where to save trained model (e.g., 'models/')
    2. Load ratings data
    3. Create user-item matrix
    4. Train the model
    5. Save everything
    6. Print statistics:
       - Number of users
       - Number of movies
       - Matrix sparsity (% of missing values)
       - Model file sizes

    You can add command-line arguments if you want, or hardcode paths.
    """
    print("=" * 60)
    print("ReelRecs - Collaborative Filtering Model Training")
    print("=" * 60)

    data_dir = Path("../data/ml-1m")
    output_dir = Path("models/collaborative_filtering")

    ratings_df = load_ratings(data_dir)
    user_item_matrix, user_id_to_idx, movie_id_to_idx = create_user_item_matrix(ratings_df)
    model = train_collaborative_filtering_model(user_item_matrix)
    save_model_artifacts(model, user_item_matrix, user_id_to_idx, movie_id_to_idx, output_dir)

    n_users = user_item_matrix.shape[0]
    n_movies = user_item_matrix.shape[1]
    
    # For sparse matrices, use .nnz (number of non-zeros)
    total_elements = user_item_matrix.shape[0] * user_item_matrix.shape[1]
    n_nonzero = user_item_matrix.nnz
    sparsity = 100.0 * (total_elements - n_nonzero) / total_elements

    print(f"Number of users: {n_users}")
    print(f"Number of movies: {n_movies}")
    print(f"Matrix sparsity: {sparsity:.2f}%")

    model_size = (output_dir / "cf_model.pkl").stat().st_size / (1024 * 1024)
    matrix_size = (output_dir / "user_item_matrix.pkl").stat().st_size / (1024 * 1024)
    print(f"Model file size: {model_size:.2f} MB")
    print(f"User-item matrix file size: {matrix_size:.2f} MB")


if __name__ == "__main__":
    main()
