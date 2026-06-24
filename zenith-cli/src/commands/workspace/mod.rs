//! Pure logic for `zenith workspace scratch`, `zenith workspace candidate`,
//! and `zenith workspace promote`.
//!
//! Submodules:
//! - [`scratch`] — `zenith workspace scratch new/list/show`
//! - [`candidate`] — `zenith workspace candidate` (set lifecycle status)
//! - [`promote`] — `zenith workspace promote` (merge a selected candidate into a page)

mod candidate;
mod promote;
pub(crate) mod scratch;

pub use candidate::{candidate_set_status, candidate_set_status_in};
pub use promote::{promote, promote_in};
pub use scratch::{
    scratch_list, scratch_list_in, scratch_new, scratch_new_in, scratch_show, scratch_show_in,
};
