# topos

## TODO

 - [x] Layout (`Placer` struct, flexbox)
 - [x] Layout Text
 - [x] Custom fonts
 - [x] Clip rects
 - [x] Use linear color space
 - [x] Platform output (cursor, window stuff)
 - [x] Lines, Bezier, Fills (maybe just linearize/tessellate for now)
 - [ ] Move LayoutEngine into SceneResources (so that elements can cache taffy nodes)
 - [ ] Investigate title bar height growing/shrinking
 - [ ] Border color macos (AppKit `backgroundColor` on `NSWindow`)
 - [ ] Proper framepacing
 - [ ] Focus
 - [ ] InputBoundary struct (egui `InputRect` equivalent)
 - [ ] Sharp box strokes
 - [ ] Encapsulate all libraries (namely euclid)
 - [ ] Adopt builder pattern across codebase
 - [x] Accessibility
 - [x] Native Context Menus
 - [x] Native System Menus
 - [ ] Pixel Alignment (PhysicalRect as `u32`)
 - [ ] Multi-window support
 - [ ] Rounded clip rects
 - [ ] Move away from MSAA
 - [ ] Move away from Tessellation for glyphs/strokes
 - [ ] Improve framepacing (better statistical determination of render times)
 - [ ] WASM
 - [ ] Rewrite layout logic (without taffy)

## Components
 - [ ] Scrolling
 - [ ] Text Editing