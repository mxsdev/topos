# topos

## TODO

 - [x] Layout (`Placer` struct, flexbox)
 - [x] Layout Text
 - [x] Custom fonts
 - [x] Clip rects
 - [x] Use linear color space
 - [x] Platform output (cursor, window stuff)
 - [x] Lines, Bezier, Fills (maybe just linearize/tessellate for now)
 - [x] Move LayoutEngine into SceneResources (so that elements can cache taffy nodes)
 - [x] Investigate title bar height growing/shrinking
 - [x] Focus
 - [x] InputBoundary struct (egui `Response` equivalent)
 - [ ] Encapsulate all libraries (namely euclid, cosmic_text, taffy engine)
 - [ ] Adopt builder pattern across codebase
 - [ ] Better buffer support (do scale factor as part of render pass, not in component; support spans)
 - [ ] Better glyph support
 - [ ] Move everything into a single shader (draw call optimization)
 - [ ] Affine transformations
 - [ ] Render engine throughput optimizations (store & diff buffers by widget)
 - [ ] Sharp box strokes
 - [ ] Layers
 - [ ] Proper framepacing
 - [ ] Multi-window support
 - [x] Accessibility
 - [x] Native Context Menus
 - [x] Native System Menus

## not priority 

 - [ ] Image support
 - [ ] Rounded clip rects
 - [ ] Move away from MSAA
 - [ ] Move away from Tessellation for glyphs/strokes
 - [ ] Improve framepacing (better statistical determination of render times)
 - [ ] WASM
 - [ ] Rewrite layout logic (without taffy)
 - [ ] Border color macos

## Components
 - [ ] Scrolling
 - [ ] Text Editing