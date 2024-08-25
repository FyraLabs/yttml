# YTT (YouTube Timed Text) Format, version 3 (SRV3)

SRV3 is a proprietary captioning format used by
YouTube for storing closed captions. It is based on the
[TTML](https://www.w3.org/TR/ttml1/) standard, but with
major modifications. The format is XML based.

The structure of a YTT file is similar to that of HTML,
but with a few key differences. As the format is designed
specifically for closed captions and not general web
content, it has a few unique elements and attributes.

The structure of a YTT file is as follows:

- Header tag (`<head>`)
  - Pens (`<pen>`)
    A pen is a variable to store a style for text. It can
    be referenced by a span to apply the style to the text, similar to
    the HTML `class` attribute. Pens are defined in the header section
    of the file, and referenced in the body inside a span as `p="<pen_id>"`.
  - Window styles (`<ws>`)
    A window style is a variable to store a style for the
    text "window" (the box that contains the text).
  - Window positions (`<wp>`)
    A window position is a variable to store the position
    of the text window on the screen, anchored to a specific corner of the
    screen, with X and Y offsets defined as `(ah, av)`.
- Body tag (`<body>`)
  - Lines (`<p>`)
    A line is a block of text that appears on the screen
    at a specific time, with a specific duration, at a specific position
    and with a specific style. The text content of the line is stored
    as the inner text of the `<p>` tag.
    - Spans (`<s>`)
      A span is a block of text with a specific style
      applied to it. Spans can be used to apply different styles to
      different parts of the text within a line. You can reference a pen
      style by using the `p="<pen_id>"` attribute on the span tag.
    - Breaks (`<br>`)
      A break is a line break within a line of text. It
      is used to split a line of text into multiple lines.
        
All the above data is then wrapped inside a `<timedtext>` tag.

## Differences from TTML

- `<tt>` is replaced by `<timedtext>`.
- Pen styles are styles for text. They are referenced by spans to be applied.
- Window styles can be used to style the text window.
- Custom window positioning using window position variables.
- Ruby text support.


## TTML Tags

> [!NOTE]
> Type definitions are in the format `name: type = description`.
> Boolean types are represented as integers 0 and 1, 0 being false and 1 being true.

### timedtext

The root element of the TTML document

### head

The header section of the YTT file. Contains definitions for pens, window styles, and window positions.

### wp

Window position.

A variable declared to store a position for a window onscreen.

#### Fields

```
id: enum = Position ID
ap: enum = Anchor point
ah: int = X location
av: int  = Y location
```

### ws

A variable to store a style for the text window.

#### Fields

```
id: enum = Style ID
ju: enum = Justification ID
pd: enum = Pitch
sd: enum = Yaw/Skew
```

### pen

A pen is a variable to store styles, it is defined by an ID
and can be referenced by span to be applied to,
similar to CSS classes.

#### Fields

```
id: int = Pen ID
fs: enum = Font style (0-7)
sz: int = Font scale
of: int = offset
b: bool = bold
i: bool = italic
u: bool = underline
fc: hex = Foreground color
fo: int = Foreground opacity
bc: hex = Background color
bo: int = Background opacity
rb: enum = Ruby text (0-5)
hg: bool = Packed text
```

### body

The body section of the YTT file. Contains the lines of text to be displayed.

### p

A line of text to be displayed on the screen.


#### Fields

```
t: int = line start (in ms at video time)
d: int = line duration (in ms)
wp: enum = position ID
ws: enum = window style ID
```

## Enum values

### AnchorPoint (`ap`)

0 - Top Left
1 - Top Center
2 - Top Right
3 - Middle Left
4 - Center
5 - Middle Right
6 - Bottom Left
7 - Bottom Center
8 - Bottom Right

### Justification (`ju`)

0 - Top Left, Middle Left, Bottom Left
1 - Top Right, Middle Right, Bottom Right
2 - Top Center, Center, Bottom Center

### Ruby text (`rb`)

0 - No ruby text
1 - Base
2 - Parentheses
4 - Before text
5 - After text 

### Font style (`fs`)

1 - Courier New, Courier, Nimbus Mono 1, Cutive Mono
2 - Times New Roman, Georgia, Cambria, PT Serif Caption
3 - Lucida Console, DejaVu Sans Mono, Monaco, Consolas, PT Mono
5 - Comic Sans MS, Impact, Handlee
6 - Monotype Corsiva, URW Chancery 1, Apple Chancery, Dancing Script
7 - Carrois Gothic SC
0 - Default system font

### Pitch (`pd`) & Yaw/Skew (`sd`)

These are used to set the pitch and yaw of the text window.

2,0 - Characters above each other, columns right to left
2,1 - Characters above each other, columns left to right
3,0 - Subtitle rotated 90° CCW, columns left to right
3,1 - Subtitle rotated 90° CCW, columns right to left
