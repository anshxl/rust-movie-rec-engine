# Server - Phase 6: Integration

This crate contains the `RecommendationOrchestrator` that ties together all components of the recommendation pipeline.

## Structure

```
server/
├── src/
│   ├── lib.rs              # Public exports
│   ├── main.rs             # Test harness
│   └── orchestrator.rs     # Main implementation (YOUR WORK HERE!)
├── IMPLEMENTATION_GUIDE.md # Step-by-step guide
├── Cargo.toml              # Dependencies
└── README.md               # This file
```

## Your Tasks

Implement the 9 methods in `orchestrator.rs` marked with `todo!()`:

1. ✅ **Phase 6.1**: `new()` - Constructor
2. ✅ **Phase 6.2**: `get_recommendations()` - Main flow
3. ✅ **Phase 6.3**: `build_user_context()` - User context
4. ✅ **Phase 6.4**: `generate_candidates_parallel()` - Parallel candidate generation
5. ✅ **Phase 6.5**: `merge_candidates()` - Deduplication
6. ✅ **Phase 6.6**: `apply_filters()` - Filter pipeline
7. ✅ **Phase 6.7**: `compute_features()` - Feature engineering
8. ✅ **Phase 6.8**: `score_with_ml()` - ML scoring (gRPC call)
9. ✅ **Phase 6.9**: `rank_and_select()` - Ranking and formatting

## Quick Start

1. **Read the implementation guide**:
   ```bash
   cat IMPLEMENTATION_GUIDE.md
   ```

2. **Open the orchestrator**:
   ```bash
   # Open crates/server/src/orchestrator.rs in your editor
   ```

3. **Implement methods one by one** following the TODO order

4. **Test as you go**:
   ```bash
   cargo check -p server  # Quick compile check
   cargo build -p server  # Full build
   ```

5. **When ready for end-to-end test**:
   - Uncomment code in `main.rs`
   - Start Python ML service: `cd python && python ml_service.py`
   - Run: `cargo run -p server`

## Key Learning Points

This phase teaches you:

- **Async coordination**: Using `tokio::join!` for parallel operations
- **CPU vs I/O**: When to use `spawn_blocking` vs direct `await`
- **Error propagation**: Using `?` and `.context()` across async boundaries
- **Type conversion**: Bridging between crate-specific types
- **Instrumentation**: Using tracing for observability
- **Arc and Clone**: Sharing data between async tasks

## Architecture Overview

```
get_recommendations(user_id)
    │
    ├─> build_user_context(user_id)
    │
    ├─> generate_candidates_parallel(&context)
    │   ├─> Thunder (spawn_blocking)  ─┐
    │   └─> Phoenix (spawn_blocking)  ─┤─> tokio::join!
    │                                   │
    ├─> merge_candidates()
    │
    ├─> apply_filters()
    │
    ├─> compute_features()
    │
    ├─> score_with_ml()  [async gRPC]
    │
    └─> rank_and_select()
        │
        └─> Vec<MovieRecommendation>
```

## Tips

- **Start simple**: Implement basic versions first, optimize later
- **Use logging**: Add `tracing::info!()` to see progress
- **Test incrementally**: Don't wait until all 9 methods are done
- **Read hints**: Each TODO has detailed hints in comments
- **Ask for help**: If stuck on a specific method, ask!

## Common Patterns You'll Use

### Async/Await
```rust
let result = self.ml_client.score_candidates(user_id, features).await?;
```

### Spawn Blocking for CPU Work
```rust
tokio::task::spawn_blocking({
    let data = self.data.clone();
    move || expensive_computation(data)
}).await?
```

### Error Context
```rust
operation().context("Failed to do X")?
```

### Parallel Execution
```rust
let (result1, result2) = tokio::join!(async_fn1(), async_fn2());
```

Good luck! Remember: the journey is the destination. Take your time and learn from each method.
