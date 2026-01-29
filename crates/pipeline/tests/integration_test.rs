//! Integration tests for the pipeline.
//!
//! These tests verify that filters and feature engineering work together
//! in a realistic scenario.

use data_loader::{DataIndex, Gender, Genre, Movie, Rating, User};
use pipeline::filters::*;
use pipeline::{FeatureEngineer, FilterPipeline};
use sources::{user_context::build_user_context, Candidate, CandidateSource};
use std::sync::Arc;

fn create_test_setup() -> (Arc<DataIndex>, Vec<Candidate>) {
    let mut index = DataIndex::new();

    // Create User 1
    index.insert_user(User {
        id: 1,
        gender: Gender::Male,
        age: data_loader::AgeGroup::Age25To34,
        occupation: data_loader::Occupation::Executive,
        zipcode: "12345".to_string(),
    });
    // Add test movies with various properties
    index.insert_movie(Movie {
        id: 1,
        title: "High Rated Action (2000)".to_string(),
        year: Some(2000),
        genres: vec![Genre::Action, Genre::Adventure],
    });

    index.insert_movie(Movie {
        id: 2,
        title: "Low Rated Drama (1995)".to_string(),
        year: Some(1995),
        genres: vec![Genre::Drama],
    });

    index.insert_movie(Movie {
        id: 3,
        title: "Good SciFi (2005)".to_string(),
        year: Some(2005),
        genres: vec![Genre::SciFi, Genre::Action],
    });

    index.insert_movie(Movie {
        id: 4,
        title: "Unwatched Action (2000)".to_string(),
        year: Some(2000),
        genres: vec![Genre::Action],
    });

    // Add ratings to create stats and user history
    // Movie 1: High rated with many ratings
    for i in 0..30 {
        index.insert_rating(Rating {
            user_id: i,
            movie_id: 1,
            rating: 4.5,
            timestamp: 1000000,
        });
    }

    // Movie 2: Low rated
    for i in 0..20 {
        index.insert_rating(Rating {
            user_id: i + 100,
            movie_id: 2,
            rating: 2.5,
            timestamp: 1000000,
        });
    }

    // Movie 3: Good rating, decent count
    for i in 0..25 {
        index.insert_rating(Rating {
            user_id: i + 200,
            movie_id: 3,
            rating: 4.2,
            timestamp: 1000000,
        });
    }

    // Movie 4: Good rating, decent count
    for i in 0..20 {
        index.insert_rating(Rating {
            user_id: i + 300,
            movie_id: 4,
            rating: 4.3,
            timestamp: 1000000,
        });
    }

    // User 1 has watched movies 1 and 2, prefers Action
    index.insert_rating(Rating {
        user_id: 1,
        movie_id: 1,
        rating: 5.0,
        timestamp: 1000000,
    });
    index.insert_rating(Rating {
        user_id: 1,
        movie_id: 2,
        rating: 3.0,
        timestamp: 1000000,
    });
    // Compute stats
    index.compute_movie_stats();

    // Wrap in Arc for sharing
    let index = Arc::new(index);

    // Create candidates
    let candidates = vec![
        Candidate::new(1, CandidateSource::Thunder, 0.95),  // Watched - should be filtered
        Candidate::new(2, CandidateSource::Phoenix, 0.80),  // Low rating - should be filtered
        Candidate::new(3, CandidateSource::Phoenix, 0.85),  // SciFi - might be filtered by genre
        Candidate::new(4, CandidateSource::Thunder, 0.90),  // Action, unwatched, high rating - should pass
    ];

    (index, candidates)
}

#[test]
fn test_full_pipeline_filters_correctly() {
    let (index, candidates) = create_test_setup();

    let context = build_user_context(&index, 1).unwrap();

    let pipeline = FilterPipeline::new()
        .add_filter(AlreadyWatchedFilter)
        .add_filter(MinimumRatingFilter::new(index.clone(), 3.5, 10))
        .add_filter(GenrePreferenceFilter::new(index.clone(), 3));

    let filtered = pipeline.apply(candidates, &context).unwrap();

    // Should have filtered out:
    // - Movie 1 (already watched)
    // - Movie 2 (low rating)
    // - Movie 3 (SciFi not in user's top genres)
    // Should keep:
    // - Movie 4 (Action, unwatched, high rating)

    assert!(
        filtered.len() <= 2,
        "Pipeline should filter out most candidates"
    );
}

#[test]
fn test_feature_engineering_after_filtering() {
    let (index, candidates) = create_test_setup();

    let context = build_user_context(&index, 1).unwrap();

    let pipeline = FilterPipeline::new().add_filter(AlreadyWatchedFilter);

    let filtered = pipeline.apply(candidates, &context).unwrap();

    let engineer = FeatureEngineer::new(index.clone());
    let features = engineer.compute_features(&filtered, &context);

    assert_eq!(
        features.len(),
        filtered.len(),
        "Should have features for each filtered candidate"
    );

    for feature in &features {
        assert!(
            !context.watched_movies.contains(&feature.movie_id),
            "No features should be for watched movies"
        );
    }
}

#[test]
fn test_complete_pipeline_realistic() {
    let (index, candidates) = create_test_setup();

    let context = build_user_context(&index, 1).unwrap();

    // Full pipeline as it would be used in production
    let pipeline = FilterPipeline::new()
        .add_filter(AlreadyWatchedFilter)
        .add_filter(MinimumRatingFilter::new(index.clone(), 3.5, 10))
        .add_filter(GenrePreferenceFilter::new(index.clone(), 3));

    let filtered = pipeline.apply(candidates, &context).unwrap();

    assert!(
        !filtered.is_empty(),
        "Should have at least some candidates after filtering"
    );

    let engineer = FeatureEngineer::new(index.clone());
    let features = engineer.compute_features(&filtered, &context);

    // Verify feature values are reasonable
    for feature in &features {
        assert!(
            feature.genre_overlap_score >= 0.0 && feature.genre_overlap_score <= 1.0,
            "Genre overlap should be in [0, 1]"
        );
        assert!(feature.avg_rating >= 0.0 && feature.avg_rating <= 5.0);
        assert!(feature.rating_count > 0);
    }
}
