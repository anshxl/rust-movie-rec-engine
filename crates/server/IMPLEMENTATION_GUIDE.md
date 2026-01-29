# Phase 6 Implementation Guide

This guide will help you implement the orchestrator step by step.

## Prerequisites

Before you start, make sure:
1. The Python ML service is running: `cd python && python ml_service.py`
2. All previous phases are complete (data-loader, sources, pipeline, ml-client)

## Implementation Order

Follow the TODO numbers in `orchestrator.rs`:

### Step 1: Constructor (Phase 6.1)
**File**: `orchestrator.rs::new()`

What you need:
- Create ThunderSource and PhoenixSource
- Build FilterPipeline with these filters (in order):
  1. `AlreadyWatchedFilter`
  2. `MinimumRatingFilter::new(data_index.clone(), 3.5, 10)`
  3. `GenrePreferenceFilter::new(data_index.clone(), 3)`
- Create FeatureEngineer
- Connect to ML service (async!)

Import path examples:
```rust
use pipeline::filters::{AlreadyWatchedFilter, MinimumRatingFilter, GenrePreferenceFilter};
```

### Step 2: Main Flow (Phase 6.2)
**File**: `orchestrator.rs::get_recommendations()`

This is the glue that connects everything. Call methods in this order:
1. `build_user_context(user_id)?`
2. `generate_candidates_parallel(&context).await?`
3. `merge_candidates(thunder_cands, phoenix_cands)`
4. `apply_filters(merged, &context)?`
5. `compute_features(&filtered, &context)`
6. `score_with_ml(user_id, features).await?`
7. `rank_and_select(filtered, features, scores, limit)?`

Add timing and logging between steps!

### Step 3: Build User Context (Phase 6.3)
**File**: `orchestrator.rs::build_user_context()`

Simple wrapper:
```rust
sources::user_context::build_user_context(&self.data_index, user_id)
    .context("Failed to build user context")
```

### Step 4: Parallel Candidate Generation (Phase 6.4)
**File**: `orchestrator.rs::generate_candidates_parallel()`

This is the trickiest part! You need:
- `tokio::join!` to run both sources in parallel
- `tokio::task::spawn_blocking` for CPU-bound work
- Handle the double Result: `Result<Vec<Candidate>, JoinError>`

Template:
```rust
let (thunder_result, phoenix_result) = tokio::join!(
    tokio::task::spawn_blocking({
        let thunder = self.thunder.clone();
        let context = context.clone();
        move || thunder.get_candidates(&context, 300)
    }),
    // ... similar for phoenix with limit 200
);

// Unwrap the spawn_blocking Result, then the inner Vec<Candidate>
let thunder_candidates = thunder_result.context("Thunder task panicked")??;
let phoenix_candidates = phoenix_result.context("Phoenix task panicked")??;
```

### Step 5: Merge Candidates (Phase 6.5)
**File**: `orchestrator.rs::merge_candidates()`

Use HashMap for deduplication:
```rust
use std::collections::HashMap;

let mut map: HashMap<MovieId, Candidate> = HashMap::new();

// For each candidate, insert or update if score is higher
// Hint: Use entry API with and_modify()
```

### Step 6: Apply Filters (Phase 6.6)
**File**: `orchestrator.rs::apply_filters()`

Simple delegation:
```rust
self.filter_pipeline.apply(candidates, context)
    .context("Failed to apply filters")
```

### Step 7: Compute Features (Phase 6.7)
**File**: `orchestrator.rs::compute_features()`

Even simpler:
```rust
self.feature_engineer.compute_features(candidates, context)
```

### Step 8: Score with ML (Phase 6.8)
**File**: `orchestrator.rs::score_with_ml()`

You need to convert types. The pipeline has `pipeline::CandidateFeatures` but ml-client expects `ml_client::recommendations::CandidateFeatures`.

They have identical fields, but different types. Use map:
```rust
let proto_features: Vec<_> = features
    .into_iter()
    .map(|f| ml_client::recommendations::CandidateFeatures {
        movie_id: f.movie_id,
        genre_overlap_score: f.genre_overlap_score,
        // ... map all fields
        movie_year: f.movie_year.map(|y| y as u32), // u16 -> u32
        // ...
    })
    .collect();

self.ml_client.score_candidates(user_id, proto_features).await
    .context("Failed to score candidates with ML service")
```

### Step 9: Rank and Select (Phase 6.9)
**File**: `orchestrator.rs::rank_and_select()`

Steps:
1. Zip candidates with scores: `candidates.into_iter().zip(scores)`
2. Sort by score (descending)
3. Take top N
4. Convert to MovieRecommendation:
   - Look up movie from data_index
   - Convert genres from enum to String
   - Create explanation string

```rust
let mut scored: Vec<_> = candidates
    .into_iter()
    .zip(scores)
    .collect();

// Sort by score descending
scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

// Take top N
scored.truncate(limit);

// Convert to MovieRecommendation
scored
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
            explanation: format!("Score: {:.3}, Source: {:?}", score, candidate.source),
        })
    })
    .collect()
```

## Testing

Once you've implemented all methods:

1. **Test the binary compiles**:
   ```bash
   cargo build -p server
   ```

2. **Start the Python ML service** (in another terminal):
   ```bash
   cd python
   source .venv/bin/activate  # or .venv\Scripts\activate on Windows
   python ml_service.py
   ```

3. **Run the test harness** (uncomment code in main.rs first):
   ```bash
   cargo run -p server
   ```

4. **Check the output**:
   - Should see logging from each stage
   - Should get 20 recommendations
   - Should see titles, scores, and sources

## Common Issues

### Issue: "Thunder/Phoenix doesn't implement Clone"
**Solution**: Add `#[derive(Clone)]` to ThunderSource and PhoenixSource in the sources crate.

### Issue: "Can't move self in spawn_blocking"
**Solution**: Clone what you need before the move:
```rust
let thunder = self.thunder.clone();
move || thunder.get_candidates(...)
```

### Issue: "ML service connection refused"
**Solution**: Make sure Python service is running on port 50051.

### Issue: "Type mismatch for CandidateFeatures"
**Solution**: You have two types:
- `pipeline::CandidateFeatures` (from feature engineer)
- `ml_client::recommendations::CandidateFeatures` (protobuf)

Convert between them by mapping each field.

### Issue: "partial_cmp returns Option"
**Solution**: When sorting f32, use:
```rust
.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal))
```

## Next Steps

After Phase 6 works:
- Phase 7: Build a proper CLI (clap-based)
- Phase 8: Add benchmarking
- Phase 9: Add more sophisticated post-processing (diversity, etc.)

## Getting Help

If you get stuck on a specific method:
1. Read the hints in the TODO comment
2. Check similar code in other crates (e.g., how filters work)
3. Look at the CLAUDE.md architecture diagrams
4. Ask for help on the specific method you're stuck on

Remember: The goal is to learn by doing! Take your time with each method.
