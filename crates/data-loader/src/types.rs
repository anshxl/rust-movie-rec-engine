//! Core domain types for the MovieLens dataset.
//!
//! This module defines the fundamental data structures used throughout the system.
//! Key Rust concepts demonstrated here:
//! - Type aliases for domain clarity (UserId, MovieId)
//! - Structs with public fields
//! - Enums for fixed sets of values
//! - Derive macros for common traits
//! - HashMap and BTreeMap for efficient lookups

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

// =============================================================================
// Type Aliases
// =============================================================================
// These make the domain clearer and prevent mixing up user IDs with movie IDs

/// Unique identifier for a user (1-6040 in MovieLens 1M)
pub type UserId = u32;

/// Unique identifier for a movie (varies in MovieLens 1M)
pub type MovieId = u32;

// =============================================================================
// User-related Types
// =============================================================================

/// Represents a user in the MovieLens dataset.
///
/// Rust concepts:
/// - `#[derive(Debug, Clone)]` automatically implements these traits
/// - `pub` makes fields accessible outside this module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub gender: Gender,
    pub age: AgeGroup,
    pub occupation: Occupation,
    pub zipcode: String,
}

/// Gender enum - demonstrates Rust enums for fixed value sets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
}

/// Age groups from the MovieLens dataset
///
/// Rust concept: Enums can represent discrete categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgeGroup {
    Under18,
    Age18To24,
    Age25To34,
    Age35To44,
    Age45To49,
    Age50To55,
    Age56Plus,
}

/// Occupation categories from MovieLens
///
/// Note: Using a simple enum here, but could be extended with associated data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Occupation {
    Other,
    Academic,
    Artist,
    Clerical,
    CollegeStudent,
    CustomerService,
    Doctor,
    Executive,
    Farmer,
    Homemaker,
    K12Student,
    Lawyer,
    Programmer,
    Retired,
    Sales,
    Scientist,
    SelfEmployed,
    Technician,
    Tradesman,
    Unemployed,
    Writer,
}

// =============================================================================
// Movie-related Types
// =============================================================================

/// Represents a movie in the dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Movie {
    pub id: MovieId,
    pub title: String,
    /// Year extracted from title (e.g., "Toy Story (1995)")
    ///
    /// Rust concept: `Option<T>` represents a value that may or may not exist
    /// - `Some(1995)` means we found a year
    /// - `None` means no year was found
    pub year: Option<u16>,
    /// List of genres for this movie
    ///
    /// Rust concept: `Vec<T>` is a growable array (like ArrayList in Java)
    pub genres: Vec<Genre>,
}

/// Movie genres from MovieLens
///
/// These are the 18 genres used in the dataset, represented as an enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Genre {
    Action,
    Adventure,
    Animation,
    Children,
    Comedy,
    Crime,
    Documentary,
    Drama,
    Fantasy,
    FilmNoir,
    Horror,
    Musical,
    Mystery,
    Romance,
    SciFi,
    Thriller,
    War,
    Western,
}

// =============================================================================
// Rating Type
// =============================================================================

/// Represents a single rating from a user for a movie
///
/// Rust concepts:
/// - Small, copyable struct (all fields are Copy)
/// - Demonstrates borrowing vs. ownership (we'll see this in usage)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rating {
    pub user_id: UserId,
    pub movie_id: MovieId,
    /// Rating value from 1.0 to 5.0
    pub rating: f32,
    /// Unix timestamp when rating was made
    pub timestamp: i64,
}

// =============================================================================
// Statistics Types
// =============================================================================

/// Precomputed statistics for a movie
///
/// These are computed once when loading data for fast lookups later
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MovieStats {
    pub avg_rating: f32,
    pub rating_count: u32,
    /// Popularity score derived from rating count and average
    pub popularity_score: f32,
}

// =============================================================================
// DataIndex - The Core In-Memory Database
// =============================================================================

/// Main data structure that holds all data and indices.
///
/// This is the heart of the data-loader crate. It provides O(1) lookups
/// for users, movies, and ratings through HashMap indices.
///
/// Rust concepts demonstrated:
/// - HashMap<K, V> for O(1) lookups (like a dictionary)
/// - BTreeMap<K, V> for sorted key access
/// - Borrowing: methods return `&T` (references) not `T` (owned values)
#[derive(Debug)]
pub struct DataIndex {
    // Primary data stores
    pub(crate) users: HashMap<UserId, User>,
    pub(crate) movies: HashMap<MovieId, Movie>,

    // Rating indices for fast lookups
    /// All ratings made by each user
    pub(crate) user_ratings: HashMap<UserId, Vec<Rating>>,
    /// All ratings received by each movie
    pub(crate) movie_ratings: HashMap<MovieId, Vec<Rating>>,

    // Secondary indices for specialized queries
    /// Movies grouped by genre (one movie can appear in multiple genre lists)
    pub(crate) genre_index: HashMap<Genre, Vec<MovieId>>,
    /// Movies grouped by release year (sorted by year)
    pub(crate) year_index: BTreeMap<u16, Vec<MovieId>>,

    // Precomputed statistics
    pub(crate) movie_stats: HashMap<MovieId, MovieStats>,
}

impl DataIndex {
    /// Creates a new, empty DataIndex
    ///
    /// Rust concept: `Self` is an alias for the type (DataIndex here)
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            movies: HashMap::new(),
            user_ratings: HashMap::new(),
            movie_ratings: HashMap::new(),
            genre_index: HashMap::new(),
            year_index: BTreeMap::new(),
            movie_stats: HashMap::new(),
        }
    }

    // Getters - Note: These return references (&T) not owned values (T)
    // This is a key Rust concept: borrowing vs. ownership

    /// Get a user by ID
    ///
    /// Returns `Option<&User>`:
    /// - `Some(&user)` if user exists (borrowing it)
    /// - `None` if user doesn't exist
    pub fn get_user(&self, id: UserId) -> Option<&User> {
        self.users.get(&id)
    }

    /// Get a movie by ID
    pub fn get_movie(&self, id: MovieId) -> Option<&Movie> {
        self.movies.get(&id)
    }

    /// Get all ratings made by a user
    ///
    /// Returns an empty slice if user has no ratings
    ///
    /// Rust concept: `&[T]` is a slice (view into an array/vector)
    pub fn get_user_ratings(&self, user_id: UserId) -> &[Rating] {
        self.user_ratings
            .get(&user_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all ratings for a movie
    pub fn get_movie_ratings(&self, movie_id: MovieId) -> &[Rating] {
        self.movie_ratings
            .get(&movie_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all movies in a specific genre
    pub fn get_movies_by_genre(&self, genre: Genre) -> &[MovieId] {
        self.genre_index
            .get(&genre)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all movies released in a specific year
    pub fn get_movies_by_year(&self, year: u16) -> &[MovieId] {
        self.year_index
            .get(&year)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get precomputed statistics for a movie
    pub fn get_movie_stats(&self, movie_id: MovieId) -> Option<&MovieStats> {
        self.movie_stats.get(&movie_id)
    }

    // Mutators - These will be used during data loading
    // Note: They take `&mut self` (mutable reference) to modify the data

    /// Insert a user into the index
    pub fn insert_user(&mut self, user: User) {
        self.users.insert(user.id, user);
    }

    /// Insert a movie into the index
    pub fn insert_movie(&mut self, movie: Movie) {
        self.movies.insert(movie.id, movie);
    }

    /// Insert a rating and update indices
    pub fn insert_rating(&mut self, rating: Rating) {
        // Add to user ratings
        self.user_ratings
            .entry(rating.user_id)
            .or_insert_with(Vec::new)
            .push(rating);

        // Add to movie ratings
        self.movie_ratings
            .entry(rating.movie_id)
            .or_insert_with(Vec::new)
            .push(rating);
    }

    /// Get counts for debugging/validation
    pub fn counts(&self) -> (usize, usize, usize) {
        let total_ratings = self.user_ratings.values().map(|v| v.len()).sum();
        (self.users.len(), self.movies.len(), total_ratings)
    }
}

// Implement Default trait for convenience
impl Default for DataIndex {
    fn default() -> Self {
        Self::new()
    }
}
