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
 - [ ] Focus
 - [ ] InputBoundary struct (egui `InputRect` equivalent)
 - [ ] Sharp box strokes
 - [ ] Encapsulate all libraries (namely euclid)
 - [ ] Adopt builder pattern across codebase
 - [ ] Proper framepacing
 - [x] Accessibility
 - [x] Native Context Menus
 - [x] Native System Menus
 - [ ] Pixel Alignment (PhysicalRect as `u32`)
 - [ ] Multi-window support
 - [ ] Rounded clip rects
 - [ ] Move away from MSAA
 - [ ] Move away from Tessellation for glyphs/strokes
 - [ ] Improve framepacing (better statistical determination of render times)
 - [ ] Move everything into a single shader
 - [ ] WASM
 - [ ] Rewrite layout logic (without taffy)
 - [ ] Border color macos

## Components
 - [ ] Scrolling
 - [ ] Text Editing