# ML Scoring Service - Implementation Guide

This directory contains the Python ML scoring service for ReelRecs. All skeleton files have been created with clear TODOs for you to implement.

## 📁 Project Structure

```
python/
├── generated/              # Auto-generated gRPC stubs (don't edit)
│   ├── __init__.py
│   ├── recommendations_pb2.py
│   └── recommendations_pb2_grpc.py
├── model.py               # ML model wrapper (TODO: implement)
├── ml_service.py          # gRPC service (TODO: implement)
├── train_model.py         # Model training script (TODO: implement)
├── pyproject.toml         # uv project configuration
└── README.md             # This file
```

## 🚀 Getting Started

### 1. Install Dependencies

The project uses `uv` for dependency management:

```bash
cd python
uv sync  # Install dependencies from pyproject.toml
```

### 2. Implementation Order

Follow this order for best learning experience:

#### **Step 1: Train the Model** (`train_model.py`)

Implement the training script first so you have a model to load in the service.

**Functions to implement:**
- `load_ratings()` - Load MovieLens ratings.dat file
- `create_user_item_matrix()` - Build the user-item rating matrix
- `train_collaborative_filtering_model()` - Train sklearn NearestNeighbors
- `save_model_artifacts()` - Save model, matrix, and mappings
- `main()` - Wire it all together

**Run it:**
```bash
uv run python train_model.py
```

**Expected output:**
- `models/cf_model.pkl` - Trained NearestNeighbors model
- `models/user_item_matrix.pkl` - User-item rating matrix
- `models/movie_id_to_idx.pkl` - Movie ID to matrix index mapping

---

#### **Step 2: Implement Model Wrapper** (`model.py`)

Implement the `CollaborativeFilteringModel` class that wraps the trained model.

**Methods to implement:**
- `load()` - Load model and matrix from disk using joblib
- `compute_cf_score()` - Compute CF score for a single movie
- `batch_score()` - (Optional) Vectorized scoring for efficiency

**Test it:**
```python
from model import CollaborativeFilteringModel
from pathlib import Path

# Load the model
cf_model = CollaborativeFilteringModel(
    Path("models/cf_model.pkl"),
    Path("models/user_item_matrix.pkl")
)
cf_model.load()

# Test scoring
score = cf_model.compute_cf_score(user_id=1, movie_id=1193)
print(f"CF Score: {score}")
```

---

#### **Step 3: Implement gRPC Service** (`ml_service.py`)

Implement the gRPC server that receives scoring requests from Rust.

**Methods to implement:**
- `MLScorerService.__init__()` - Load the model on startup
- `MLScorerService.ScoreCandidates()` - Score candidates (main logic)
- `serve()` - Start the gRPC server

**Scoring formula (example weights):**
```python
final_score = (
    cf_score * 0.40 +                    # Collaborative filtering
    genre_overlap_score * 0.25 +         # Genre similarity
    collaborative_score * 0.20 +         # Thunder source score
    popularity_percentile * 0.10 +       # Popularity
    year_preference_score * 0.05         # Year preference
)
```

**Run the service:**
```bash
uv run python ml_service.py
```

---

## 🧪 Testing

### Test Training Script

```bash
# Should create models/ directory with .pkl files
uv run python train_model.py

# Verify files were created
ls -lh models/
```

### Test gRPC Service (Manual)

1. Start the service:
```bash
uv run python ml_service.py
```

2. In another terminal, use `grpcurl` to test:
```bash
# List services
grpcurl -plaintext localhost:50051 list

# Call ScoreCandidates (you'll need to craft a proper request)
grpcurl -plaintext localhost:50051 recommendations.MLScorer/ScoreCandidates
```

### Test with Rust Client

Once both Python service and Rust client are implemented:

```bash
# Terminal 1: Start Python service
cd python
uv run python ml_service.py

# Terminal 2: Run Rust integration test
cargo test -p ml-client -- --ignored
```

---

## 🎓 Learning Tips

### Understanding Collaborative Filtering

The NearestNeighbors approach:
1. Each user is represented by their rating vector (row in user-item matrix)
2. To predict user U's rating for movie M:
   - Find users similar to U (using cosine similarity)
   - See how those similar users rated movie M
   - Weighted average = predicted rating for U on M

### Key sklearn APIs

```python
from sklearn.neighbors import NearestNeighbors

# Training
model = NearestNeighbors(n_neighbors=20, metric='cosine')
model.fit(user_item_matrix)  # Shape: (n_users, n_movies)

# Inference
user_vector = user_item_matrix[user_idx].reshape(1, -1)
distances, indices = model.kneighbors(user_vector, n_neighbors=20)
# indices = array of similar user indices
# distances = array of similarity scores
```

### Handling Missing Ratings

Not all users have rated all movies. Common strategies:
1. **Fill with 0** - Treat missing as "no interest"
2. **Fill with user mean** - Normalize by user's average rating
3. **Leave as NaN** - Use sparse matrices (more memory efficient)

For MovieLens 1M (6,040 users × 3,883 movies), a dense matrix is fine.

---

## 📊 Expected Performance

**Training:**
- Load ratings: < 1 second
- Create matrix: < 2 seconds
- Train model: < 5 seconds
- Total: < 10 seconds

**Serving:**
- Model loading: < 1 second
- Scoring 200 candidates: < 10ms
- gRPC overhead: < 5ms
- Total per request: < 20ms

---

## 🐛 Common Issues

### Issue: `ModuleNotFoundError: No module named 'pkg_resources'`
**Solution:** Already fixed! `setuptools` is in `pyproject.toml`

### Issue: `UnicodeDecodeError` when reading ratings.dat
**Solution:** Use `encoding='latin-1'` or `encoding='ISO-8859-1'`

### Issue: Model file not found
**Solution:** Make sure you run `train_model.py` first to create the models

### Issue: gRPC connection refused from Rust
**Solution:**
- Check Python service is running: `lsof -i :50051`
- Check the port matches in both Python and Rust (default: 50051)

---

## 🎯 Success Criteria

**Training Script:**
- ✅ Loads all 1M ratings from ratings.dat
- ✅ Creates 6040×3883 user-item matrix
- ✅ Trains NearestNeighbors model
- ✅ Saves model artifacts to models/
- ✅ Prints summary statistics

**Model Wrapper:**
- ✅ Loads model and matrix successfully
- ✅ Computes CF scores in < 1ms per movie
- ✅ Handles missing movie IDs gracefully

**gRPC Service:**
- ✅ Starts and listens on port 50051
- ✅ Loads model on startup
- ✅ Scores 200 candidates in < 20ms
- ✅ Returns correct number of scores
- ✅ Handles errors gracefully (logs + default scores)
- ✅ Can be called from Rust client

---

## 🔗 Next Steps

After implementing the Python service:

1. **Implement Rust gRPC Client** (`crates/ml-client/src/lib.rs`)
   - Connect to Python service
   - Send candidate features
   - Receive scores

2. **Integrate into Pipeline**
   - Call ML client from orchestrator
   - Combine scores with post-processing
   - Return final recommendations

3. **Test End-to-End**
   - Start Python service
   - Run Rust pipeline
   - Verify recommendations make sense

---

## 📚 Resources

- **gRPC Python Docs:** https://grpc.io/docs/languages/python/
- **sklearn NearestNeighbors:** https://scikit-learn.org/stable/modules/neighbors.html
- **MovieLens Dataset:** https://grouplens.org/datasets/movielens/1m/
- **Collaborative Filtering Tutorial:** https://realpython.com/build-recommendation-engine-collaborative-filtering/

---

Happy coding! Remember: Focus on getting it working first, then optimize. 🚀
