use serde::{Deserialize, Serialize};
use hex_color::HexColor;

#[derive(Debug, Serialize, Deserialize)]
pub struct Head {
    #[serde(rename = "pen")]
    pub pen: Vec<Pen>,
    #[serde(rename = "wp")]
    pub wp: Vec<Wp>,
    #[serde(rename = "ws")]
    pub ws: Vec<Ws>,
}

#[derive(Debug, Serialize, Deserialize)]
// The value names here are not really descriptive,
// but there's no documentation to go off of.
// So I'll be guessing what they mean and renaming the fields
// in code here to describe what I think they are.
// 
// tfw proprietary XML schema
// 
// todo: to anyone who knows what these fields are,
// please PLEASE let me know what they are.

// serde: do not add fields that are None
#[serde(deny_unknown_fields)]
pub struct Pen {
    pub id: u32,
    // hex code color
    #[serde(rename = "b")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_type: Option<u32>,
    #[serde(rename = "fc")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_color: Option<HexColor>,
    #[serde(rename = "ec")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_color: Option<HexColor>,
    #[serde(rename = "fo")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_opacity: Option<u32>,
    #[serde(rename = "bo")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_opacity: Option<u32>,
    #[serde(rename = "sz")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<u32>,
    #[serde(rename = "et")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_type: Option<u32>,
    #[serde(rename = "fs")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_style: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Wp {
    pub id: u32,
    #[serde(rename = "ap")]
    pub anchor_point: Option<u32>,
    #[serde(rename = "ah")]
    pub anchor_horizontal: Option<u32>,
    #[serde(rename = "av")]
    pub anchor_vertical: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ws {
    pub id: u32,
    #[serde(rename = "ju")]
    pub justify: Option<u32>,
    #[serde(rename = "pd")]
    pub print_direction: Option<u32>,
    #[serde(rename = "sd")]
    pub scroll_direction: Option<u32>,
}
