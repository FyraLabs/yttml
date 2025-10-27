# YTTML Project Copilot Instructions

## Project Overview

YTTML is a Rust toolkit for working with YouTube's proprietary SRV3 (Server 3) subtitle format, which is YouTube's enhanced version of TTML that supports advanced formatting features like karaoke timing, custom fonts, text effects, and positioning.

## Architecture & Key Components

### Workspace Structure
- `crates/srv3-ttml/`: Core SRV3 parser library using serde for XML deserialization
- `crates/srv3tovtt-crate/`: Format conversion library (SRV3 → VTT/SRT/ASS)
- `src/main.rs`: CLI tool for parsing and converting SRV3 files
- `reference/YTSubConverter`: Original reference code as a Git submodule
- `tests/ass`: SRV3-ASS Conversion tests
- Root-level SRV3 samples for testing (Japanese titles, multiple languages)

### Core Domain Model

**SRV3 Format Structure:**
- `TimedText` (root) → `Head` + `Body`
- `Head` contains style definitions: `Pen` (text styles), `WindowPosition` (positioning), `WindowStyle` (layout)
- `Body` contains `BodyElement` enum: `Paragraph`, `Span`, `Text`, `Br`, `Window`
- Pens reference by ID, similar to CSS classes: `<s p="1">styled text</s>`

**Critical SRV3 Concepts:**
- Font sizes use YouTube's formula: `real_percentage = 100 + (sz - 100) / 4`
- Zero-width spaces (`\u{200B}`) are used as rendering workarounds - strip for plain text, preserve for conversions
- Colors stored as hex, but ASS format requires BGR order: `&H{BB}{GG}{RR}`
- Mobile platforms (Android/iOS) have limited formatting support

## Development Patterns

### Error Handling
Use `Result<T, Box<dyn std::error::Error>>` for main functions, `std::io::Result` for file operations.

### Text Processing
- Implement `ElementExt` trait for consistent text extraction across element types
- Use `.text()` for plain text, `.text_no_zwsp()` when removing zero-width spaces
- ASS conversion hardcodes resolution (1280x720) and font size (38pt base)

### Format Conversion Workflows
1. Parse SRV3 → `TimedText` struct
2. Extract formatting from `Head.pen` styles
3. Map SRV3 positioning to target format coordinates  
4. Handle platform limitations (mobile vs desktop YouTube)

## Build & Test Commands

```bash
# Build entire workspace
cargo build

# Run CLI tool
cargo run -- parse input.srv3 --format vtt --save file

# Test specific crate
cargo test -p srv3-ttml
cargo test test_hex_to_ass_color  # Test color conversion

# Test with sample files
cargo run -- parse "crates/srv3-ttml/test/aishite.srv3" --format ass
```

## Code Conventions

### Module Organization
- Keep serde derives with XML attribute mappings: `#[serde(rename = "@id")]`
- Use repr enums for SRV3 numeric constants: `#[repr(u8)]`
- Document YouTube-specific behaviors in code comments

### Testing Strategy
- Include real SRV3 samples in `test/` directories
- Test both formatted (with pens) and unformatted subtitle files
- Test round-trip parsing: SRV3 → struct → XML should match

### Performance Notes
- ASS conversion optimizes for file size by omitting default styling when no pens exist
- Extensive pen lists (300+ styles) are common in karaoke files - handle efficiently

## Integration Points

**External Dependencies:**
- `aspasia`: Subtitle format structs and timestamp handling
- `quick-xml`: SRV3 XML parsing with serde integration
- `hex_color`: Color parsing and manipulation
- `clap`: CLI argument parsing

**Data Sources:**
- Download SRV3 files via yt-dlp: `--sub-format=srv3`
- Direct API: `https://www.youtube.com/api/timedtext?v={video_id}&lang={lang}&fmt=srv3`

## Reference C# Implementation (YTSubConverter)

This project aims to fully port the unmaintained C# YTSubConverter to modern Rust. Key features to replicate:

### Custom Override Tags (YouTube-specific ASS extensions)
The C# implementation supports YouTube-specific tags that must be preserved:
- `{\ytsub}`, `{\ytsup}`, `{\ytsur}` - Subscript/superscript/regular (PC only)
- `{\ytruby}`, `{\ytruby8}`, `{\ytruby2}` - Ruby text (furigana/bopomofo) with positioning
- `{\ytvert9}`, `{\ytvert7}`, `{\ytvert1}`, `{\ytvert3}` - Vertical text orientations (PC only)
- `{\ytdir4}` - Right-to-left text direction
- `{\ytpack1}`, `{\ytpack0}` - Text packing for vertical mode (PC only)
- `{\ytshake}`, `{\ytshake(radius)}`, `{\ytshake(radiusX,radiusY,t1,t2)}` - Shake animation
- `{\ytchroma}`, `{\ytchroma(intime,outtime)}`, `{\ytchroma(offsetX,offsetY,intime,outtime)}`, `{\ytchroma(color1,color2...,alpha,offsetX,offsetY,intime,outtime)}` - Chromatic aberration effect
- `{\ytkt}` - Advanced karaoke types:
  - `{\ytktFade}` - Fading karaoke
  - `{\ytktGlitch}` - Glitching text karaoke (generates random chars)
  - `{\ytkt(Cursor,text)}`, `{\ytkt(LCursor,text)}` - Cursor positioning
  - `{\ytkt(Cursor,interval,tags1,text1,tags2,text2,...)}` - Animated cursor

### Advanced Animation System
C# implementation uses a sophisticated animation framework (all under `Animations/` namespace):
- **Base Animation class**: `Animation.cs` with interpolation methods (linear, accelerated)
- **Animator**: `Animator.cs` - Expands single ASS lines into multiple frames based on animations
- **Animation Types**:
  - `FadeAnimation.cs` - Simple and complex fades
  - `MoveAnimation.cs` - Position transitions
  - `ColorAnimation.cs`, `ForeColorAnimation.cs`, `SecondaryColorAnimation.cs`, `ShadowColorAnimation.cs`, `BackColorAnimation.cs` - Color interpolation
  - `ScaleAnimation.cs` - Font size animations
  - `ShakeAnimation.cs` - Random position offsets
  - `GlitchingCharAnimation.cs` - Character replacement for karaoke effects

**Animation Workflow**:
1. Parse ASS tags to create Animation objects
2. Group overlapping animations into clusters
3. Generate frame-by-frame subtitle lines (typically 2-frame steps)
4. Apply interpolated values at each frame using `Animation.Apply(line, section, t)`

### Karaoke Type System
Located in `Formats/Ass/KaraokeTypes/`:
- `IKaraokeType` interface
- `SimpleKaraokeType.cs` - Basic syllable highlighting
- `FadeKaraokeType.cs` - Smooth fade-in/fade-out per syllable
- `GlitchKaraokeType.cs` - Random character generation for Latin/CJK
- `CursorKaraokeType.cs` - Text/visual markers following sung word

### Tag Handler Architecture
C# uses a tag handler pattern (`Formats/Ass/Tags/`):
- `AssTagHandlerBase.cs` - Base class with parsing utilities
- Each tag has dedicated handler: `AssBoldTagHandler`, `AssItalicTagHandler`, etc.
- Custom tags: `AssShakeTagHandler`, `AssChromaTagHandler`, `AssRubyTagHandler`, `AssVerticalTypeTagHandler`, etc.
- Handlers populate `AssTagContext` which builds final SRV3 output

### Conversion Pipeline (ASS → SRV3)
1. Parse ASS document into `AssDocument` with `AssLine` objects
2. Process override tags via tag handlers
3. Expand animations into frame sequences
4. Apply karaoke type transformations
5. Generate SRV3 XML with proper pen/window style references

## Common Tasks

### Adding New Output Format
1. Add variant to `OutputFormat` enum in `main.rs`
2. Implement conversion function in `srv3tovtt-crate`
3. Add match arm in `main()` format handling
4. Test with both simple and complex karaoke files

### Implementing Custom Tags
1. Create tag handler module following `AssTagHandlerBase` pattern
2. Parse tag arguments (use helper methods for float/color lists)
3. Add to SRV3 output or create Animation object
4. Test with real ASS files using the tag

### Adding Animation Support
1. Define animation struct implementing interpolation logic
2. Add animation parsing in tag handlers
3. Implement frame expansion in conversion pipeline
4. Test timing accuracy (YouTube uses ~30fps granularity)

### SRV3 Format Changes
1. Update structs in `srv3-ttml/src/lib.rs` with serde attributes
2. Add documentation referencing `srv3-format.md`
3. Update format documentation if needed
4. Test parsing with real YouTube samples

### Performance Issues
- Profile with complex karaoke files (1000+ paragraphs, 300+ pens)
- Animation expansion can create 100+ lines from one ASS line
- Consider streaming parser for very large files
- ASS output can be memory-intensive due to positioning calculations

Focus on achieving feature parity with YTSubConverter while modernizing the codebase and improving maintainability.