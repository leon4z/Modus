/**
 * Creates an overlay click handler that is safe for Tauri window dragging.
 * 
 * Using `onclick` instead of `onmousedown` because:
 * 1. Native window drag (data-tauri-drag-region) does NOT fire click events after drag
 * 2. For text selection drag concern, we check if any text is selected before closing
 * 
 * Usage in Svelte:
 *   const oc = overlayClose(closeCallback);
 *   <div class="overlay" onclick={oc}>
 * 
 * @param {() => void} closeFn - Function to call when overlay is clicked
 */
export function overlayClose(closeFn) {
  return function onClick(/** @type {MouseEvent} */ e) {
    // Only close if clicked directly on the overlay, not on children
    if (e.target !== e.currentTarget) return;
    // Don't close if user was selecting text (drag from modal to overlay)
    const sel = window.getSelection();
    if (sel && sel.toString().length > 0) {
      sel.removeAllRanges();
      return;
    }
    closeFn();
  };
}
