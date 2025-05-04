pub mod display;

// proto defitions
pub mod prototypes {
    pub mod types {
        include!(concat!(env!("OUT_DIR"), "/prototypes.types.rs"));
    }
}
