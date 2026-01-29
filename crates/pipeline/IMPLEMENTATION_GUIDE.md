# Pipeline Implementation Guide

## What's Been Set Up

Phase 4 skeleton is now complete! Here's what you have:

### 📁 Project Structure

```
crates/pipeline/
├── Cargo.toml              ✅ Dependencies configured
├── src/
│   ├── lib.rs              ✅ Module exports configured
│   ├── traits.rs           ✅ Filter trait defined
│   ├── filter_pipeline.rs  ⚠️  FilterPipeline struct (TODO: implement apply)
│   ├── features.rs         ⚠️  FeatureEngineer (TODO: implement)
│   └── filters/
│       ├── mod.rs          ✅ Module exports
│       ├── already_watched.rs     ⚠️  (TODO: implement apply)
│       ├── genre_preference.rs    ⚠️  (TODO: implement apply)
│       ├── minimum_rating.rs      ⚠️  (TODO: implement apply)
│       └── recency.rs             ⚠️  (TODO: implement apply)
└── tests/
    └── integration_test.rs ✅ Integration tests ready
```

### ✅ What's Complete

1. **Cargo.toml** - All dependencies added (data-loader, sources, anyhow, rayon, tracing)
2. **Filter Trait** - Core trait with `name()` and `apply()` methods
3. **Type Definitions** - CandidateFeatures struct with all fields
4. **Test Skeletons** - Unit tests for each component, integration tests
5. **Documentation** - Rustdoc comments explaining each component

### ⚠️ What You Need to Implement

All functions marked with `todo!()` - here's the list:

## 🔧 Implementation Tasks

### 1. AlreadyWatchedFilter (`filters/already_watched.rs`)

**Function:** `apply()`

**Algorithm:**
- Filter the candidates Vec
- Keep only candidates where `!context.watched_movies.contains(&candidate.movie_id)`
- Return the filtered Vec

**Hints:**
- Use `into_iter().filter().collect()`
- Or `retain()` if you want to be explicit
- The HashSet lookup is O(1)

---

### 2. GenrePreferenceFilter (`filters/genre_preference.rs`)

**Function:** `apply()`

**Algorithm:**
1. Get user's top N genres: `context.top_genres(self.top_n_genres)`
2. For each candidate:
   - Look up the movie in `self.data_index.get_movie(candidate.movie_id)`
   - Check if movie has any genre in the top genres list
   - Keep if there's overlap
3. Return filtered Vec

**Hints:**
- Use `movie.genres.iter().any(|g| top_genres.contains(g))`
- Handle the case where movie might not be found (skip it)

---

### 3. MinimumRatingFilter (`filters/minimum_rating.rs`)

**Function:** `apply()`

**Algorithm:**
1. For each candidate:
   - Get stats: `self.data_index.get_movie_stats(candidate.movie_id)`
   - Check: `stats.avg_rating >= self.min_rating`
   - Check: `stats.rating_count >= self.min_count`
   - Keep if both conditions met
2. Return filtered Vec

**Hints:**
- Handle None case for stats (movie not found)
- Both conditions must be true

---

### 4. RecencyFilter (`filters/recency.rs`)

**Function:** `apply()`

**Algorithm:**
1. If `context.preferred_era.is_none()`, return all candidates unchanged
2. Otherwise, for each candidate:
   - Get movie: `self.data_index.get_movie(candidate.movie_id)`
   - Get movie year
   - If no year, keep the movie (benefit of doubt)
   - If year exists, check: `|movie_year - preferred_era| <= self.year_tolerance`
3. Return filtered Vec

**Hints:**
- Use pattern matching on Options
- `i16` conversion might be needed for subtraction

---

### 5. FilterPipeline (`filter_pipeline.rs`)

**Function:** `apply()`

**Algorithm:**
```rust
let mut current = candidates;
for filter in &self.filters {
    tracing::debug!("Applying filter: {}", filter.name());
    current = filter.apply(current, context)?;
    tracing::debug!("Candidates after {}: {}", filter.name(), current.len());
}
Ok(current)
```

**Hints:**
- Very straightforward - just loop and apply
- The `?` operator handles error propagation

---

### 6. FeatureEngineer - Main Functions (`features.rs`)

#### `compute_features()`

**Algorithm:**
```rust
candidates.par_iter()
    .map(|candidate| self.compute_single(candidate, user_context))
    .collect()
```

**Hints:**
- Import `use rayon::prelude::*;` at the top
- This is parallel iteration - Rayon handles the parallelism

---

#### `compute_single()`

**Algorithm:**
1. Create `CandidateFeatures::new(candidate.movie_id)`
2. Look up movie from `self.data_index.get_movie(candidate.movie_id)`
3. Look up stats from `self.data_index.get_movie_stats(candidate.movie_id)`
4. Populate each field:
   - `genre_overlap_score` - call `compute_genre_overlap()`
   - `collaborative_score` - from `candidate.base_score`
   - `similar_users_count` - from `candidate.metadata.similar_users_count.unwrap_or(0)`
   - `avg_rating` - from `stats.avg_rating`
   - `rating_count` - from `stats.rating_count`
   - `popularity_percentile` - call `compute_popularity_percentile()`
   - `movie_year` - from `movie.year`
   - `year_preference_score` - call `compute_year_preference()`
   - `days_since_released` - compute from year (optional, can start with 0.0)
5. Return the features

---

#### `compute_genre_overlap()`

**Algorithm (Jaccard Similarity):**
1. Get user's top genres (e.g., top 3-5)
2. Get movie's genres from DataIndex
3. Calculate intersection: genres in both sets
4. Calculate union: genres in either set
5. Return `intersection_size / union_size` (or 0.0 if union is empty)

**Hints:**
- Use HashSet for efficient intersection/union
- Watch out for division by zero

---

#### `compute_year_preference()`

**Algorithm:**
```rust
match (movie_year, preferred_era) {
    (Some(year), Some(era)) => {
        let distance = (year as i32 - era as i32).abs() as f32;
        let max_distance = 50.0; // or whatever you choose
        (1.0 - (distance / max_distance)).max(0.0)
    }
    _ => 0.5  // Neutral score if either is missing
}
```

---

#### `compute_popularity_percentile()`

**Algorithm:**
This one is trickier. You need to rank movies by their rating_count.

**Simple Approach:**
1. Get the movie's rating count
2. Return a normalized score (e.g., `rating_count as f32 / max_rating_count`)
3. For now, you could return a simple heuristic like:
   ```rust
   let count = self.data_index.get_movie_stats(movie_id)?.rating_count;
   (count as f32 / 500.0).min(1.0)  // Assuming 500 is a "very popular" threshold
   ```

**Better Approach (later):**
- Pre-compute percentiles for all movies in DataIndex
- Store in a HashMap<MovieId, f32>
- Look up during feature computation

---

## 🧪 Testing Strategy

### Run Tests As You Go

```bash
# Test individual filter
cargo test -p pipeline already_watched

# Test all filters
cargo test -p pipeline filters

# Test feature engineering
cargo test -p pipeline features

# Test full pipeline
cargo test -p pipeline filter_pipeline

# Integration tests
cargo test -p pipeline --test integration_test
```

### Debugging Tips

1. **Add logging:** Use `tracing::debug!` or `println!` to see intermediate values
2. **Run single test:** `cargo test -p pipeline test_name -- --nocapture`
3. **Check types:** If borrow checker complains, think about ownership
4. **Start simple:** Get basic version working, then optimize

---

## 📊 Expected Behavior

After implementation, here's what should happen:

### AlreadyWatchedFilter
- Input: 500 candidates
- Output: ~480 candidates (removes ~20 watched movies)

### MinimumRatingFilter
- Input: 480 candidates
- Output: ~350 candidates (removes low-quality movies)

### GenrePreferenceFilter
- Input: 350 candidates
- Output: ~200-250 candidates (keeps only preferred genres)

### FeatureEngineer
- Input: 200 candidates
- Output: 200 CandidateFeatures (one per candidate)
- Time: <10ms (with Rayon parallelism)

---

## 🎯 Learning Goals

As you implement these:

1. **Iterator patterns** - Learn `filter()`, `map()`, `collect()`
2. **Option handling** - Practice with `unwrap_or()`, `map()`, pattern matching
3. **Parallel iteration** - See Rayon's `par_iter()` in action
4. **Trait objects** - See how `Box<dyn Filter>` enables polymorphism
5. **Builder pattern** - The `add_filter()` chaining
6. **Error propagation** - Using `?` operator with Result

---

## 🚀 Getting Started

### Recommended Order:

1. **AlreadyWatchedFilter** (easiest - simple filter)
2. **MinimumRatingFilter** (practice with Option handling)
3. **GenrePreferenceFilter** (more complex logic)
4. **RecencyFilter** (optional, if you want)
5. **FilterPipeline.apply()** (straightforward once filters work)
6. **FeatureEngineer.compute_features()** (parallel iteration)
7. **FeatureEngineer.compute_single()** (main feature logic)
8. **Helper functions** (genre overlap, year preference, popularity)

### When You Get Stuck:

1. Read the test cases - they show expected behavior
2. Check the type signatures - they guide the implementation
3. Look at the algorithm comments - they break down the steps
4. Remember: The docs in CLAUDE.md have the specifications

---

## ✅ Verification

You'll know you're done when:

1. ✅ `cargo test -p pipeline` passes (except the `#[ignore]` test)
2. ✅ `cargo clippy -p pipeline` has no warnings (or only minor ones)
3. ✅ All filters remove expected candidates
4. ✅ Features have reasonable values (0.0-1.0 for scores, etc.)
5. ✅ Integration test works end-to-end

---

Good luck! This is where you really get to practice Rust's ownership model, iterators, and trait system. Take your time and enjoy the process! 🦀
