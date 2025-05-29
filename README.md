# topos

## TODO

 - [x] Layout (`Placer` struct, flexbox)
 - [x] Layout Text
 - [x] Custom fonts
 - [x] Clip rects
 - [x] Use linear color space
 - [x] Platform output (cursor, window stuff)
 - [x] Lines, Bezier, Fills (maybe just linearize/tessellate for now)nicoburns
 - [x] Move LayoutEngine into SceneResources (so that elements can cache taffy nodes)
 - [x] Investigate title bar height growing/shrinking
 - [x] Focus
 - [x] InputBoundary struct (egui `Response` equivalent)
 - [x] Encapsulate all libraries (namely euclid, cosmic_text, taffy engine)
 - [x] Adopt builder pattern across codebase
 - [x] Better buffer support (do scale factor as part of render pass, not in component; support spans)
 - [x] Better glyph support
 - [x] Move everything into a single shader (draw call optimization)
 - [x] Optimize atlas as "TextureAtlas" (finite maximum of textures on shader)
 - [x] Include clip rectangle in shader vertex data
 - [x] Affine transformations
 - [x] Move all device pixel ratio logic to the shader
 - [x] Text rendering alignment & edge clipping
 - [ ] Text culling w/ clip rects
 - [x] Image fill textures with `TextureRef`
 - [x] Text resolution
 - [x] Text render-ahead
 - [ ] Custom render pipelines (onto `TextureRef`)
 - [ ] Support multiple rounded clip rects / clip rect intersection
 - [ ] Render engine throughput optimizations (store & diff buffers by widget)
 - [ ] Sharp box strokes
 - [ ] Layers
 - [x] Proper framepacing
 - [ ] Multi-window support
 - [ ] Abstract out renderer, layout engine, platform integration, application state
 - [ ] Replace Arc<...> with nominal "Ref" types
 - [ ] Replace Mutex, RwLock with auto-unwrapping alternatives (and bring to a crate, too)
 - [x] Accessibility
 - [x] Native Context Menus
 - [x] Native System Menus

## not priority 

 - [x] Image support
 - [x] Rounded clip rects
 - [ ] Move away from MSAA
 - [ ] Move away from Tessellation for glyphs/strokes
 - [x] Improve framepacing (better statistical determination of render times)
 - [ ] WASM
 - [x] Border color macos

## Components
 - [ ] Scrolling
 - [ ] Text Editing