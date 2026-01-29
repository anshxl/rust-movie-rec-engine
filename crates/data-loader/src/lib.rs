//! # Data Loader Crate
//!
//! This crate handles loading and indexing the MovieLens 1M dataset.
//!
//! ## Main Components
//!
//! - **types**: Core domain types (User, Movie, Rating, DataIndex)
//! - **parser**: Parse .dat files into Rust structs
//! - **index**: Build efficient indices for fast lookups
//! - **error**: Error types for data loading
//!
//! ## Example Usage
//!
//! ```ignore
//! use data_loader::DataIndex;
//! use std::path::Path;
//!
//! // Load the entire dataset
//! let index = DataIndex::load_from_files(Path::new("data/ml-1m"))?;
//!
//! // Query data
//! let user = index.get_user(1).unwrap();
//! let movie = index.get_movie(1193).unwrap();
//! let ratings = index.get_user_ratings(1);
//!
//! println!("User {} rated {} movies", user.id, ratings.len());
//! ```
//!
//! ## Learning Goals
//!
//! This crate demonstrates several key Rust concepts:
//!
//! 1. **Ownership and Borrowing**: DataIndex owns the data, methods return references
//! 2. **Error Handling**: Using Result<T> and custom error types
//! 3. **Type Safety**: Type aliases (UserId, MovieId) prevent mixing up IDs
//! 4. **Collections**: HashMap and BTreeMap for efficient lookups
//! 5. **Traits**: Implementing Display, Debug, Error, etc.
//! 6. **Modules**: Organizing code into logical units
//! 7. **Parallel Processing**: Using Rayon for data-parallel operations

// Public modules
pub mod error;
pub mod types;
pub mod parser;
pub mod index;

// Re-export commonly used types for convenience
pub use error::{DataLoadError, Result};
pub use types::{
    // Type aliases
    UserId,
    MovieId,
    // Core types
    User,
    Movie,
    Rating,
    DataIndex,
    MovieStats,
    // Enums
    Gender,
    AgeGroup,
    Occupation,
    Genre,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_index_creation() {
        // Test that we can create an empty DataIndex
        let index = DataIndex::new();
        let (users, movies, ratings) = index.counts();

        assert_eq!(users, 0);
        assert_eq!(movies, 0);
        assert_eq!(ratings, 0);
    }

    #[test]
    fn test_insert_user() {
        let mut index = DataIndex::new();

        let user = User {
            id: 1,
            gender: Gender::Male,
            age: AgeGroup::Age25To34,
            occupation: Occupation::Programmer,
            zipcode: "12345".to_string(),
        };

        index.insert_user(user.clone());

        let retrieved = index.get_user(1).unwrap();
        assert_eq!(retrieved.id, 1);
        assert_eq!(retrieved.zipcode, "12345");
    }

    #[test]
    fn test_insert_movie() {
        let mut index = DataIndex::new();

        let movie = Movie {
            id: 1,
            title: "Toy Story (1995)".to_string(),
            year: Some(1995),
            genres: vec![Genre::Animation, Genre::Children, Genre::Comedy],
        };

        index.insert_movie(movie.clone());

        let retrieved = index.get_movie(1).unwrap();
        assert_eq!(retrieved.id, 1);
        assert_eq!(retrieved.year, Some(1995));
        assert_eq!(retrieved.genres.len(), 3);
    }

    #[test]
    fn test_insert_rating() {
        let mut index = DataIndex::new();

        let rating = Rating {
            user_id: 1,
            movie_id: 1193,
            rating: 5.0,
            timestamp: 978300760,
        };

        index.insert_rating(rating);

        let user_ratings = index.get_user_ratings(1);
        assert_eq!(user_ratings.len(), 1);
        assert_eq!(user_ratings[0].rating, 5.0);

        let movie_ratings = index.get_movie_ratings(1193);
        assert_eq!(movie_ratings.len(), 1);
    }

    #[test]
    fn test_empty_queries() {
        let index = DataIndex::new();

        // Querying non-existent data should return None or empty slices
        assert!(index.get_user(999).is_none());
        assert!(index.get_movie(999).is_none());
        assert!(index.get_user_ratings(999).is_empty());
        assert!(index.get_movie_ratings(999).is_empty());
        assert!(index.get_movies_by_genre(Genre::Action).is_empty());
    }
}
