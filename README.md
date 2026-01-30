# ReelRecs - Movie Recommendation Engine

A production-grade movie recommendation system built in Rust with Python ML integration, inspired by X's (Twitter's) open-sourced recommendation algorithm architecture.

## Overview

ReelRecs is a **weekend learning project** designed to master Rust through building a real-world recommendation system. It processes the MovieLens 1M dataset (1M ratings, 6K users, 4K movies) and generates personalized movie recommendations using collaborative filtering and content-based techniques.

### Key Features

- 🦀 **High-Performance Rust Pipeline**: Parallel data processing with Rayon, async orchestration with Tokio
- 🤖 **Python ML Integration**: Scikit-learn collaborative filtering via gRPC
- 🎯 **Multi-Source Recommendations**: Combines in-network (Thunder) and out-of-network (Phoenix) candidate generation
- 🔧 **Extensible Architecture**: Trait-based filter pipeline for easy customization
- ⚡ **Fast**: ~240ms dataset loading, ~760ms end-to-end recommendation generation
- 🖥️ **CLI Interface**: Simple command-line tool for testing and benchmarking

## Architecture

```
┌─────────────────────────────────────────────┐
│  CLI Client (Rust)                          │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│  Orchestrator (Rust)                        │
│  ├─ Thunder Source (collaborative)          │
│  ├─ Phoenix Source (genre/popularity)       │
│  ├─ Filter Pipeline (trait-based)           │
│  ├─ Feature Engineering (parallel)          │
│  └─ Post-Processing                         │
└─────────────────┬───────────────────────────┘
                  │ gRPC
┌─────────────────▼───────────────────────────┐
│  ML Scoring Service (Python)                │
│  └─ Scikit-learn Collaborative Filtering    │
└─────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Rust 1.70+ ([install](https://rustup.rs/))
- Python 3.8+ with `uv` ([install](https://docs.astral.sh/uv/))
- MovieLens 1M dataset

### 1. Clone and Setup

```bash
git clone <your-repo-url>
cd reel-recs

# Download MovieLens dataset
mkdir -p data && cd data
wget https://files.grouplens.org/datasets/movielens/ml-1m.zip
unzip ml-1m.zip
cd ..
```

### 2. Start the Python ML Service

```bash
cd python
uv venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate
uv pip sync requirements.txt

# Train the model (first time only)
python train_model.py

# Start the gRPC service
python ml_service.py
```

The service will start on `localhost:50051`.

### 3. Run the CLI

In a new terminal:

```bash
# Build in release mode
cargo build --release -p cli

# Get recommendations for user 1
cargo run -p cli --release -- recommend --user-id 1

# Get recommendations with explanations
cargo run -p cli --release -- recommend --user-id 1 --explain

# View user profile
cargo run -p cli --release -- user --user-id 1

# Search for movies
cargo run -p cli --release -- search --title "matrix"

# Run performance benchmark
cargo run -p cli --release -- benchmark --requests 50 --concurrent 10
```

## CLI Commands

### `recommend` - Get movie recommendations

```bash
cargo run -p cli --release -- recommend --user-id <USER_ID> [OPTIONS]

Options:
  --user-id <USER_ID>    User ID (1-6040)
  --limit <LIMIT>        Number of recommendations [default: 20]
  --explain              Show detailed explanations
```

**Example output:**
```
Movie Recommendations:
1. The Matrix (1999) [Action, Sci-Fi, Thriller] - Score: 0.95
2. The Shawshank Redemption (1994) [Drama] - Score: 0.93
3. Inception (2010) [Action, Sci-Fi, Thriller] - Score: 0.91
...
```

### `user` - View user profile

```bash
cargo run -p cli --release -- user --user-id <USER_ID>
```

Shows:
- User demographics (age, gender, occupation)
- Rating statistics
- Top-rated movies
- Genre preferences

### `search` - Find movies by title

```bash
cargo run -p cli --release -- search --title <QUERY>
```

Case-insensitive substring search with relevance ranking.

### `benchmark` - Performance testing

```bash
cargo run -p cli --release -- benchmark [OPTIONS]

Options:
  --requests <N>      Number of requests [default: 100]
  --concurrent <N>    Concurrent requests [default: 10]
```

Shows P50/P95/P99 latencies and throughput.

## Project Structure

```
reel-recs/
├── crates/
│   ├── cli/              # Command-line interface
│   ├── data-loader/      # Dataset parsing & indexing
│   ├── sources/          # Thunder & Phoenix candidate sources
│   ├── pipeline/         # Filters & feature engineering
│   ├── server/           # Orchestrator
│   └── ml-client/        # gRPC client for Python service
├── python/
│   ├── ml_service.py     # gRPC scoring service
│   └── train_model.py    # Model training script
└── data/
    └── ml-1m/            # MovieLens dataset
```

## How It Works

1. **Data Loading**: Parse MovieLens dataset into in-memory indices (HashMaps, BTrees) with Rayon parallelism
2. **Candidate Generation**:
   - **Thunder Source**: Find movies liked by similar users (collaborative filtering)
   - **Phoenix Source**: Discover new movies based on genre preferences and popularity
3. **Filtering**: Apply trait-based filters (already watched, genre preference, minimum rating)
4. **Feature Engineering**: Compute 8 features per candidate (genre overlap, popularity, temporal, etc.)
5. **ML Scoring**: Send features to Python service for collaborative filtering scores
6. **Post-Processing**: Re-rank with diversity penalties, select top N

## Performance

- **Data Loading**: ~240ms (4.26M ratings/second)
- **Recommendation Generation**: ~760ms P50 per request
- **Throughput**: 0.2+ req/s with concurrent requests
- **Memory**: ~100MB for loaded dataset

## Learning Goals Achieved

This project was built to learn Rust through real-world usage:

✅ Ownership, borrowing, and lifetimes
✅ Trait-based design patterns
✅ Parallel processing with Rayon
✅ Async/await with Tokio
✅ gRPC integration (Rust ↔ Python)
✅ Error handling with anyhow/thiserror
✅ CLI building with clap
✅ When to use custom implementations vs libraries

See [CLAUDE.md](CLAUDE.md) for detailed architecture and learning notes.

## Technology Stack

**Rust:**
- `rayon` - Data parallelism
- `tokio` - Async runtime
- `tonic` - gRPC framework
- `clap` - CLI parsing
- `anyhow/thiserror` - Error handling

**Python:**
- `scikit-learn` - Collaborative filtering
- `grpcio` - gRPC server
- `numpy` - Matrix operations

## Future Extensions

- [ ] Implement Polars-based data loader for comparison
- [ ] Add caching layer for user contexts
- [ ] A/B testing framework
- [ ] Explainability features
- [ ] Docker deployment
- [ ] REST API alongside gRPC
- [ ] Real-time model updates

## Inspiration

Architecture inspired by [X's (Twitter's) open-sourced For You algorithm](https://github.com/xai-org/x-algorithm), which uses a similar multi-stage pipeline with Rust for data processing and Python for ML inference.

## License

MIT

## Acknowledgments

- MovieLens dataset provided by [GroupLens Research](https://grouplens.org/)
- Architecture inspired by X's recommendation system
