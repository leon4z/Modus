import '@testing-library/jest-dom/vitest';

if (typeof Range !== "undefined") {
  const emptyClientRects = /** @type {DOMRectList} */ ({
    length: 0,
    item: () => null,
    [Symbol.iterator]: () => [][Symbol.iterator](),
  });

  Range.prototype.getClientRects ||= () => ({
    ...emptyClientRects,
  });
  Range.prototype.getBoundingClientRect ||= () => ({
    bottom: 0,
    height: 0,
    left: 0,
    right: 0,
    top: 0,
    width: 0,
    x: 0,
    y: 0,
    toJSON: () => {},
  });
}
