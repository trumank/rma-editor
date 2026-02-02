//! Conversion layer between asset_ser ObjectPool and clean objects.rs types.
//!
//! This module provides loading and saving functionality:
//! - `load`: Convert ObjectPool data into clean `objects::URoomGenerator`
//! - `save`: Recreate pool objects from `objects::URoomGenerator`
//!
//! # Design
//!
//! - **No in-place updates**: When saving, pool objects are recreated from scratch
//! - **Direct property access**: Uses `uesave::Properties` directly via macros
//! - **Missing property handling**: Properties with default values may be omitted from serialization
//!
//! # Example
//!
//! ```ignore
//! use rma::convert::{load_room_generator, save_room_generator};
//!
//! // Load from pool
//! let room = load_room_generator(&pool, root_handle)?;
//!
//! // Edit the room...
//! room.base.bounds = 2000.0;
//!
//! // Save back to a new pool
//! let mut new_pool = ObjectPool::new();
//! let new_handle = save_room_generator(&mut new_pool, &room, None, "MyRoom")?;
//! ```

#[macro_use]
pub mod macros;
pub mod enums;
pub mod load;
pub mod save;

pub use load::load_room_generator;
pub use save::save_room_generator;
