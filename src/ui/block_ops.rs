//! Pure index calculations for block editing (spec-064).
//!
//! Keeping the drop/split index math outside the command handler gives the
//! edge cases (first, last, empty page, self-drop) a direct test seam.

/// The insertion index after removing the dragged block. `target` is the
/// target block's position in the remaining list, or `None` for append.
pub fn drop_index(len: usize, target: Option<usize>, before: bool) -> usize {
    let index = match target {
        Some(index) => {
            if before {
                index
            } else {
                index + 1
            }
        }
        None => len,
    };
    index.min(len)
}

/// The position for a new block created by pressing Enter in `source`, or
/// `None` when the source is out of range.
pub fn split_index(len: usize, source: usize) -> Option<usize> {
    (source < len).then_some(source + 1)
}

/// True when a block with `top`/`height` is outside the viewport plus overscan.
/// Used by the block height cache to skip unchanged offscreen blocks
/// (spec-110).
pub fn block_outside_viewport(top: f32, height: f32, viewport: (f32, f32), overscan: f32) -> bool {
    let bottom = top + height;
    bottom < viewport.0 - overscan || top > viewport.1 + overscan
}
