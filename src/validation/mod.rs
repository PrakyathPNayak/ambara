//! Validation module for pre-execution checking.
//!
//! The validation pipeline runs before execution to catch errors early.

pub mod pipeline;
pub mod stages;

pub use pipeline::ValidationPipeline;
pub use stages::{
    StructuralValidation, TypeValidation, ConstraintValidation,
    CustomValidation, ResourceValidation, ValidationStage,
};
