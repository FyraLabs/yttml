use std::str::FromStr;
use hex_color::HexColor;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "timedtext")]
/// The TimedText struct is the root of the XML file.
/// It contains the head and body of the XML file.
pub struct TimedText {
    #[serde(rename = "format")]
    pub format: Option<String>,
    pub head: Option<Head>,
    pub body: Body,
}
impl TimedText {
    pub fn from_str(s: &str) -> Result<Self, quick_xml::DeError> {
        quick_xml::de::from_str(s)
    }
}

impl FromStr for TimedText {
    type Err = quick_xml::DeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        quick_xml::de::from_str(s)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Head {
    #[serde(rename = "pen")]
    pub pen: Vec<Pen>,
    #[serde(rename = "wp")]
    pub wp: Vec<WindowPosition>,
    #[serde(rename = "ws")]
    pub ws: Vec<WindowStyle>,
}

#[derive(Debug, Serialize, Deserialize)]
/// Pen is a variable that contains the style of the text
/// similar to CSS classes
pub struct Pen {
    /// ID of pen.
    /// 
    /// The ID must be an incrementing integer starting from 1.
    /// If the ID is not ordered, YouTube will re-order the IDs
    /// causing body element assignments to be out of order.
    #[serde(rename = "@id")]
    pub id: u32,

    // --- Text styles ---
    #[serde(rename = "@b")]
    #[serde(default)]
    /// Toggle bold style
    pub bold: Option<bool>,

    #[serde(rename = "@i")]
    #[serde(default)]
    /// Toggle italic style
    pub italic: Option<bool>,

    #[serde(rename = "@u")]
    #[serde(default)]
    /// Toggle underline style
    pub underline: Option<bool>,

    /// Foreground color of the text
    #[serde(rename = "@fc")]
    #[serde(default)]
    pub foreground_color: Option<HexColor>,

    /// Opacity of foreground color, has to be input separately
    /// because it's a separate attribute in the XML
    /// 
    /// Specifying 255 will result in attribute not being written
    /// when uploading to YouTube.
    ///
    /// If your Hex is RGBA your program should automatically separate the A value
    /// into this attribute
    /// 
    #[serde(rename = "@fo")]
    #[serde(default)]
    pub foreground_opacity: Option<u32>,

    /// Background color of the text
    #[serde(rename = "@bc")]
    #[serde(default)]
    pub background_color: Option<HexColor>,

    /// Opacity of background color, has to be input separately
    /// because it's a separate attribute in the XML.
    ///
    /// If your Hex is RGBA your program should automatically separate the A value
    /// into this attribute
    #[serde(rename = "@bo")]
    #[serde(default)]
    pub background_opacity: Option<u8>,

    /// Color of text outline/edge
    #[serde(rename = "@ec")]
    pub edge_color: Option<HexColor>,

    /// Type of edge/outline of the text
    #[serde(rename = "@et")]
    #[serde(default)]
    pub edge_type: Option<EdgeType>,

    /// Text size
    /// 
    /// The value is a virtual percentage of the default size; the real 
    /// percentage is 100 + (sz - 100) / 4, meaning 200 will result
    /// in a 125% font size.
    /// 
    /// The smallest value is 0 which results in 75% font size.
    #[serde(rename = "@sz")]
    #[serde(default)]
    pub font_size: Option<u32>,

    #[serde(rename = "@fs")]
    #[serde(default)]
    /// Font style, see `FontStyle` for more information
    pub font_style: Option<FontStyle>,

    #[serde(rename = "@rb")]
    #[serde(default)]
    /// Ruby style, see `RubyStyle` for more information
    pub ruby: Option<RubyStyle>,

    #[serde(rename = "@hg")]
    #[serde(default)]
    /// Packing of text
    /// 
    /// Only supports value of 1, not supported on mobile players.
    pub packing: Option<u32>,
    
    #[serde(rename = "@of")]
    #[serde(default)]
    /// "Offset" of text.
    /// This is actually mislabeled, since this
    /// sets the vertical offset of the text (Subscript/Superscript)
    pub text_offset: Option<TextOffset>,
    
    #[serde(rename = "@te")]
    #[serde(default)]
    /// Text emphasis, see `TextEmphasis` for more information
    /// 
    /// This is used to allow or disallow bold text from being
    /// displayed.
    pub text_emphasis: Option<TextEmphasis>,
}




#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Justify {
    Left = 0,
    Right = 1,
    Center = 2,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum TextEmphasis {
    /// Allow bold text
    Allow = 1,
    /// Disallow bold text
    Deny = 2,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum TextOffset {
    Superscript = 1,
    Subscript = 2,
}


/// Ruby text
///
/// Ruby text is a small annotation above or below the main text,
/// typically used in East Asian typography since
/// Chinese characters are logographic. Ruby text is used to clarify
/// pronounciation or meaning of these glyphs.
///
/// Sometimes called "furigana" in Japanese and "bopomofo" in Chinese.
///
/// They can also sometimes be used in English text to clarify references
/// to literary devices or other things, for example in the game Honkai Star Rail
/// which makes extensive use of ruby text to clarify references to the game's lore.
///
/// This ruby type is an enum that specifies the type of ruby text.
///
/// This can be:
/// - No ruby text
/// - Base
/// - Parentheses
/// - Before text
/// - After text
#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum RubyStyle {
    #[serde(default)]
    None = 0,
    Base = 1,
    Parenthesis = 2,
    BeforeText = 4,
    AfterText = 5,
}

/// Type of edge/outline of text
#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum EdgeType {
    None = 0,
    HardShadow = 1,
    Bevel = 2,
    Glow = 3,
    SoftShadow = 4,
}


#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum FontStyle {
    #[serde(default)]
    /// Default font style.
    /// Implicitly defaults to Proportional Sans (Roboto)
    Default = 0, 
    /// Monospace Serif.
    /// Courier New
    MonoSerif = 1, 
    /// Proportional Serif.
    /// Times New Roman
    ProportionalSerif = 2,
    /// Monospace Sans.
    /// Lucida Console
    MonoSans = 3,
    /// Proportional Sans.
    /// Roboto, same as default setting but explicit
    ProportionalSans = 4,
    /// Casual.
    /// Comic Sans
    Casual = 5,
    /// Cursive.
    /// Monotype Corsiva
    Cursive = 6,
    /// Small Capitals.
    /// Arial with `font-variant: small-caps`
    SmallCaps = 7
}

impl Default for FontStyle {
    fn default() -> Self {
        FontStyle::Default
    }
}

/// Point to anchor text to
/// ```dot
/// 0 ======== 1 ======== 2
/// |          |          |
/// 3          4          5
/// |          |          |
/// 6 ======== 7 ======== 8
/// ```
#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum AnchorPoint {
    TopLeft = 0,
    TopCenter = 1,
    TopRight = 2,
    MiddleLeft = 3,
    Center = 4,
    MiddleRight = 5,
    BottomLeft = 6,
    BottomCenter = 7,
    BottomRight = 8,
}

#[derive(Debug, Serialize, Deserialize)]
/// Window position of the text
#[serde(rename = "wp")]
pub struct WindowPosition {
    /// ID of window position
    #[serde(rename = "@id")]
    pub id: u32,
    #[serde(rename = "@ap")]
    /// References to an anchor point.
    /// Is an enum which specify which corner of the screen the text is anchored to.
    pub anchor_point: Option<AnchorPoint>,

    // The server only accepts integers for "ah" and "av", even though the player accepts floating point.
    // In theater mode, the width covered by "ah" includes the black bars on the sides, meaning the
    // subtitles move towards the sides and even out of the video.
    // The player transforms the coordinates according to effectiveCoord = (specifiedCoord * 0.96) + 2,
    // meaning subtitles don't appear *quite* where you want them to
    
    /// X position from anchor point
    #[serde(rename = "@ah")]
    pub horizontal_offset: Option<i32>,

    /// Y position from anchor point
    #[serde(rename = "@av")]
    pub vertical_offset: Option<i32>,

    #[serde(default, rename = "@rc")]
    pub rows_total: Option<u8>,
    /// Each column has en-dash width
    #[serde(default, rename = "@cc")]
    pub columns_total: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "ws")]
pub struct WindowStyle {
    /// ID of window style
    #[serde(rename = "@id")]
    pub id: u32,
    #[serde(rename = "@ju")]
    /// Justification of text
    pub justify: Option<Justify>,

    // todo: Both of these should be an enum of 0-3 but I don't know what to name them
    /// Pitch direction of text (vertical tilt)
    #[serde(rename = "@pd")]
    pub pitch_direction: Option<Rotation>,

    /// Yaw (skew) direction of text (horizontal tilt)
    #[serde(rename = "@sd")]
    pub skew_direction: Option<Rotation>,
    
    #[serde(default, rename = "@mh")]
    pub mode_hint: ModeHint,
    #[serde(default, rename = "@wfc")]
    pub fill_color: Option<String>,
    #[serde(default, rename = "@wfo")]
    pub fill_opacity: Option<u8>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ModeHint {
  #[default]
  None = 0,
  Default = 1,
  Scroll = 2,
}


#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u32)]
pub enum Rotation {
    Zero = 0,
    Ninety = 1,
    OneEighty = 2,
    TwoSeventy = 3,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Body {
    // todo: parse this properly into a list of enums or something
    #[serde(rename = "$value")]
    pub elements: Vec<BodyElement>,
}

// --- The body ---
// Finally, the body of the XML
// the actual data we want to really parse
//
// It's a bunch of <p> tags with some attributes and inner text, kinda like HTML
#[derive(Debug, Serialize, Deserialize)]
pub struct Paragraph {
    // The actual text inside the <p> tag
    // <p>text</p>
    #[serde(rename = "$value")]
    // Always treat the inner text as string
    pub inner: Vec<BodyElement>,
    /// Timestamp on when to display the caption in milliseconds
    /// 
    /// A timestamp of 0 should be avoided as on Android the positioning will be ignored
    /// and even not displayed at all in some cases.
    #[serde(rename = "@t")]
    pub timestamp: u64,
    /// Duration to display the caption for
    #[serde(rename = "@d")]
    pub duration: u64,

    /// Reference to a window position ID
    #[serde(rename = "@wp")]
    pub window_position: Option<u32>,

    /// Reference to a window style ID
    #[serde(rename = "@ws")]
    pub window_style: Option<u32>,
}

/// Trait for nested elements in the body
trait ElementExt {
    /// Concatenates text elements into a single string
    ///
    /// This is useful for converting the text back into a plain text format
    fn text(&self) -> String;
    // default implementation
    /// Concatenates text elements into a single string, but with markup applied
    ///
    /// This is meant to be used for debugging purposes,
    /// all serialization should be done through serde, not this method
    fn text_markup(&self) -> String {
        self.text()
    }
}

impl ElementExt for Vec<BodyElement> {
    fn text(&self) -> String {
        // add newlines between paragraphs
        self.iter()
            .fold(String::new(), |acc, elem| acc + &elem.text() + "\n").trim().to_string()
    }
}

impl ElementExt for String {
    fn text(&self) -> String {
        self.clone()
    }
}

// todo: maybe implicitly make use of the trait anyway?
impl ElementExt for BodyElement {
    fn text(&self) -> String {
        match self {
            Self::Text(t) => t.text(),
            Self::Br(x) => x.text(),
            BodyElement::Div(x) => x.text(),
            BodyElement::Span(x) => x.text(),
            Self::Paragraph(x) => x.text(),
        }
    }
}

impl ElementExt for Paragraph {
    fn text(&self) -> String {
        self.inner
            .iter()
            .fold(String::new(), |acc, elem| match elem {
                BodyElement::Text(t) => acc + t,
                BodyElement::Br(_) => acc + "\n",
                _ => acc,
            })
    }
    fn text_markup(&self) -> String {
        self.inner
            .iter()
            .fold(String::new(), |acc, elem| match elem {
                BodyElement::Text(t) => acc + t,
                BodyElement::Br(_) => acc + "<br>",
                _ => acc,
            })
    }
}

impl Paragraph {
    /// Concatenates text elements into a single string
    ///
    /// This is useful for converting the text back into a plain text format
    ///
    /// However, this function does not handle and remove zero-width spaces (U+200B) which are used in YTT
    /// to prevent some text rendering issues. If you want to remove ZWSPs, use `text_no_zwsp` instead.
    pub fn text(&self) -> String {
        <Self as ElementExt>::text(self)
    }

    pub fn text_markup(&self) -> String {
        <Self as ElementExt>::text_markup(self)
    }

    /// Concatenates text elements into a single string, but removes zero-width spaces (U+200B)
    ///
    /// ZWSP is used in YTT as a workaround to prevent some text rendering issues, but when converting back to
    /// plain text, we don't want to include them.
    ///
    /// For conversions from other formats, ZWSPs should be added back in.
    pub fn text_no_zwsp(&self) -> String {
        self.text().replace("\u{200B}", "")
    }
}

// todo: make the thing like HTML

#[derive(Debug, Serialize, Deserialize)]
pub struct Span {
    #[serde(rename = "$value")]
    pub inner: Option<Vec<BodyElement>>,
    /// Reference to a pen style ID
    #[serde(rename = "p")]
    pub pen: Option<u32>,
    // nested inside is more spans or something
    // this is a recursive structure
}

impl ElementExt for Span {
    fn text(&self) -> String {
        self.inner
            .as_ref()
            .map(|inner_elements| inner_elements.text())
            .unwrap_or_default()
    }

    // text_markup: wrap in <s> tags
    fn text_markup(&self) -> String {
        quick_xml::se::to_string(&self).unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Br; // todo: Somehow convert this into a newline

impl ElementExt for Br {
    fn text(&self) -> String {
        "\n".to_string()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Div {
    #[serde(rename = "$value")]
    pub elements: Vec<BodyElement>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BodyElement {
    #[serde(rename = "$text")]
    Text(String),
    #[serde(rename = "p")]
    Paragraph(Paragraph),
    #[serde(rename = "s")]
    Span(Span),
    #[serde(rename = "br")]
    Br(Br),
    #[serde(rename = "div")]
    Div(Vec<Self>),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn test_parse(s: &str) {
        let parse = TimedText::from_str(s).unwrap();
        println!("\n{:#?}\n", parse);
        let t = parse.body.elements.text();
        println!("{}", t);
        
        let xml = quick_xml::se::to_string(&parse).unwrap();
        println!("```xml\n{}\n```", xml);
        
    }

    // TODO: So formatted files with <s> tags inside the <p> tags are not parsed correctly
    // We want to treat them literally as <s> tags, not as a separate element
    #[test]
    fn test_parse_file() {
        let file = include_str!("../test/aishite.srv3");
        test_parse(file);
    }

    #[test]
    fn test_parse_file_unformatted() {
        let file = include_str!("../test/mesmerizer.srv3.xml");
        test_parse(file);
    }
}
