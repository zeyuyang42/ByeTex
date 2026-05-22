### Typst State Management with `state`, `get`, and `update`

Source: https://typst.app/docs/reference/introspection/state

This example shows the correct way to manage state in Typst using the `state` function, which exposes `get` and `update` methods. State updates are guaranteed to occur in layout order.

```typst
#let star = state("star", 0)
#let compute(expr) = {
  star.update(old =>
    eval(expr.replace("⭐", str(old)))
  )
  [New value is #context star.get().]
}

#compute("10") \
#compute("⭐ + 3") \
#compute("⭐ * 2") \
#compute("⭐ - 5")

```

--------------------------------

### Basic Tiling Example

Source: https://typst.app/docs/reference/visualize/tiling

A simple example demonstrating how to create and apply a basic tiling pattern to a rectangle.

```APIDOC
## Basic Tiling Example

```typ
#let pat = tiling(size: (30pt, 30pt))[
  #place(line(start: (0%, 0%), end: (100%, 100%)))
  #place(line(start: (0%, 100%), end: (100%, 0%)))
]

#rect(fill: pat, width: 100%, height: 60pt, stroke: 1pt)
```
```

--------------------------------

### Tiling Constructor Usage Example

Source: https://typst.app/docs/reference/visualize/tiling

An example showcasing the direct usage of the tiling constructor with various parameters.

```APIDOC
## Tiling Constructor Usage Example

```typ
#let pat = tiling(
  size: (20pt, 20pt),
  relative: "parent",
  place(
    dx: 5pt,
    dy: 5pt,
    rotate(45deg, square(
      size: 5pt,
      fill: black,
    )),
  ),
)

#rect(width: 100%, height: 60pt, fill: pat)
```
```

--------------------------------

### Tiling on Text Example

Source: https://typst.app/docs/reference/visualize/tiling

Example showing how to apply tiling to text, with relativity set to 'parent'.

```APIDOC
## Tiling on Text Example

```typ
#let pat = tiling(
  size: (30pt, 30pt),
  relative: "parent",
  square(
    size: 30pt,
    fill: gradient.conic(..color.map.rainbow),
  )
)

#set text(fill: pat)
#lorem(10)
```
```

--------------------------------

### Equation Syntax and Examples

Source: https://typst.app/docs/reference/math/equation

Demonstrates the usage of the equation element with different configurations and provides examples of inline and block-level equations.

```APIDOC
## Equation Syntax and Examples

### Inline Equation
Equations can be displayed inline with text by enclosing the mathematical markup within single dollar signs.

```typst
Let $a$, $b$, and $c$ be the side lengths of a right-angled triangle.
Then, we know that: $ a^2 + b^2 = c^2 $
```

### Block-Level Equation
An equation becomes block-level by including whitespace after the opening dollar sign and whitespace before the closing dollar sign. By default, block-level equations will not break across pages.

```typst
Prove by induction:
$ sum_(k=1)^n k = (n(n+1)) / 2 $
```

### Customizing Block Equations
To allow block-level equations to break across pages, use a `set` rule:

```typst
#set math.equation(block: true, breakable: true)

$ a + b + c + d + e + f + g + h + i + j + k + l + m + n + o + p + q + r + s + t + u + v + w + x + y + z $
```

### Numbering Block Equations
Block-level equations can be numbered using the `numbering` parameter. The `number-align` parameter controls the alignment of the numbering.

```typst
#set math.equation(numbering: "(1)", number-align: bottom)

We define:
$ phi.alt := (1 + sqrt(5)) / 2 $ <ratio>

With @ratio, we get:
$ F_n = floor(1 / sqrt(5) phi.alt^n) $
```

### Equation Supplement
A supplement can be added to equations using the `supplement` parameter, which is useful for references.

```typst
#set math.equation(numbering: "(1)", supplement: [Eq.])

We define:
$ phi.alt := (1 + sqrt(5)) / 2 $ <ratio>

With @ratio, we get:
$ F_n = floor(1 / sqrt(5) phi.alt^n) $
```

### Alternative Text for Accessibility
Provide an alternative description for equations using the `alt` parameter for assistive technologies.

```typst
#math.equation(
  alt: "integral from 1 to infinity of a x squared plus b with respect to x",
  block: true,
  $ integral_1^oo a x^2 + b dif x $
)
```
```

--------------------------------

### Ref Element Syntax and Examples

Source: https://typst.app/docs/reference/model/ref

Demonstrates the basic syntax for creating references using the `@` symbol and provides examples of its usage with different forms and supplements.

```APIDOC
## Example
```
#set page(numbering: "1")
#set heading(numbering: "1.")
#set math.equation(numbering: "(1)")

= Introduction <intro>
Recent developments in
typesetting software have
rekindled hope in previously
frustrated researchers. @distress
As shown in @results (see
#ref(<results>, form: "page")),
we ...

= Results <results>
We discuss our approach in
comparison with others.

== Performance <perf>
@slow demonstrates what slow
software looks like.
$ T(n) = O(2^n) $ <slow>

#bibliography("works.bib")

```

## Syntax
This function also has dedicated syntax: A `"normal"` reference to a label can be created by typing an `@` followed by the name of the label (e.g. `= Introduction <intro>` can be referenced by typing `@intro`).
To customize the supplement, add content in square brackets after the reference: `@intro[Chapter]`.
```

--------------------------------

### Get start alignment of direction

Source: https://typst.app/docs/reference/layout/direction

Returns the starting alignment point for a given direction using the '.start()' method. This is useful for positioning.

```typst
#ltr.start()
#rtl.start()
#ttb.start()
#btt.start()
```

--------------------------------

### HTML Video Example

Source: https://typst.app/docs/reference/html/typed

Example demonstrating the usage of the html.video function to create a video element with controls and dimensions.

```APIDOC
## HTML Video Element Example

### Description
This example shows how to create an HTML video element using the `html.video` function, specifying attributes like `controls`, `width`, `height`, and `src`.

### Code
```typc
#html.video(
  controls: true,
  width: 1280,
  height: 720,
  src: "sunrise.mp4",
)[
  Your browser does not support the video tag.
]
```
```

--------------------------------

### Basic Raw Text Example

Source: https://typst.app/docs/reference/text/raw

Demonstrates embedding raw text, including code examples within code examples, and handling of backticks and spaces.

```typst
Adding `rbx` to `rcx` gives
the desired result.

What is ```rust fn main()``` in Rust
would be ```c int main()``` in C.

```rust
fn main() {
    println!("Hello World!");
}
```

This has ``` `backticks` ``` in it
(but the spaces are trimmed). And
``` here``` the leading space is
also trimmed.


```

--------------------------------

### Enum Element Examples

Source: https://typst.app/docs/reference/model/enum

Provides code examples for various `enum` parameter configurations.

```APIDOC
## Enum Element Examples

### `tight` parameter example:
```typ
+ If an enum has a lot of text, and
  maybe other inline content, it
  should not be tight anymore.

+ To make an enum wide, simply
  insert a blank line between the
  items.
```

### `numbering` parameter examples:
```typ
#set enum(numbering: "1.a)")
+ Different
+ Numbering
  + Nested
  + Items
+ Style

#set enum(numbering: n => super[#n])
+ Superscript
+ Numbering!
```

### `start` parameter example:
```typ
#enum(
  start: 3,
  [Skipping],
  [Ahead],
)
```

### `full` parameter example:
```typ
#set enum(numbering: "1.a)", full: true)
+ Cook
  + Heat water
  + Add ingredients
+ Eat
```

### `reversed` parameter example:
```typ
#set enum(reversed: true)
+ Coffee
+ Tea
+ Milk
```
```

--------------------------------

### direction.start()

Source: https://typst.app/docs/reference/layout/direction

The `start` definition returns the start point of a direction, represented as an alignment.

```APIDOC
## direction.start()

### Description
The start point of this direction, as an alignment.

### Syntax
`self.start() -> alignment`

### Usage Examples
```
#ltr.start()
#rtl.start()
#ttb.start()
#btt.start()
```
```

--------------------------------

### Tiling with Spacing Example

Source: https://typst.app/docs/reference/visualize/tiling

Demonstrates using the 'spacing' parameter to control the gaps between tiling elements.

```APIDOC
## Tiling with Spacing Example

```typ
#let pat = tiling(
  size: (30pt, 30pt),
  spacing: (10pt, 10pt),
  relative: "parent",
  square(
    size: 30pt,
    fill: gradient.conic(..color.map.rainbow),
  ),
)

#rect(
  width: 100%,
  height: 60pt,
  fill: pat,
)
```
```

--------------------------------

### Float Constructor and Basic Usage

Source: https://typst.app/docs/reference/foundations/float

Demonstrates how to create float values, including conversions from other types and basic examples.

```APIDOC
## Float
A floating-point number. A limited-precision representation of a real number. Typst uses 64 bits to store floats. Wherever a float is expected, you can also pass an integer.
You can convert a value to a float with this type's constructor.
NaN and positive infinity are available as `float.nan` and `float.inf` respectively.

### Example
```
#3.14 \
#1e4 \
#(10 / 4)

```

## Constructor
Converts a value to a float.
* Booleans are converted to `0.0` or `1.0`.
* Integers are converted to the closest 64-bit float. For integers with absolute value less than `calc.pow(2, 53)`, this conversion is exact.
* Ratios are divided by 100%.
* Strings are parsed in base 10 to the closest 64-bit float. Exponential notation is supported.

```
#float(false) \
#float(true) \
#float(4) \
#float(40%) \
#float("2.7") \
#float("1e5")

```

### `float(value: bool or int or float or ratio or str or decimal) -> float`
#### `value`
bool or int or float or ratio or str or decimal
Required Positional
The value that should be converted to a float.
```

--------------------------------

### Basic Line Examples

Source: https://typst.app/docs/reference/visualize/line

Demonstrates creating lines with different lengths and endpoints. The `set page` rule is used for basic page layout.

```typst
#set page(height: 100pt)

#line(length: 100%)
#line(end: (50%, 50%))
#line(
  length: 4cm,
  stroke: 2pt + maroon,
)
```

--------------------------------

### String Basics and Examples

Source: https://typst.app/docs/reference/foundations/str

Demonstrates basic string operations, including concatenation, splitting with strings and regex, and checking for pattern existence.

```APIDOC
## String Basics

A sequence of Unicode codepoints. Strings can be concatenated with `+` and multiplied with integers. They support escape sequences for special characters.

### Example
```
#"hello world!"
#"\"hello\n  world"!"
#"1 2 3".split()
#"1,2;3".split(regex("[,;]"))
#(regex("\\d+") in "ten euros")
#(regex("\\d+") in "10 euros")
```

### Escape Sequences
- `\\`: backslash
- `\"`: quote
- `\n`: newline
- `\r`: carriage return
- `\t`: tab
- `\u{1f600}`: hexadecimal Unicode escape sequence
```

--------------------------------

### Enum Starting Number

Source: https://typst.app/docs/reference/model/enum

Shows how to specify a custom starting number for an enumeration using the `start` parameter.

```typst
#enum(
  start: 3,
  [Skipping],
  [Ahead],
)
```

--------------------------------

### Typst `floor` Function Example

Source: https://typst.app/docs/reference/math/lr

Shows how to use the `floor` function to compute the floor of an expression.

```typst
$ floor(x/2) $
```

--------------------------------

### Image Source and Format Examples

Source: https://typst.app/docs/reference/visualize/image

Provides examples of using raw bytes for image sources and specifying formats, including raw pixel data.

```APIDOC
### `source` Example with Bytes
```ty
#let original = read("diagram.svg")
#let changed = original.replace(
  "#2B80FF", // blue
  green.to-hex(),
)

#image(bytes(original))
#image(bytes(changed))
```

### `format` Example with Raw Pixel Data
```ty
#image(
  read(
    "tetrahedron.svg",
    encoding: none,
  ),
  format: "svg",
  width: 2cm,
)

#image(
  bytes(range(16).map(x => x * 16)),
  format: (
    encoding: "luma8",
    width: 4,
    height: 4,
  ),
  width: 2cm,
)
```
```

--------------------------------

### Basic List Example in Typst

Source: https://typst.app/docs/reference/model/list

Demonstrates the basic usage of the `list` element for displaying text, math, and layout elements. Also shows how to create multi-line list items through indentation and how to use the `list` function with arguments.

```typst
Normal list.
- Text
- Math
- Layout
- ...

Multiple lines.
- This list item spans multiple
  lines because it is indented.

Function call.
#list(
  [Foundations],
  [Calculate],
  [Construct],
  [Data Loading],
)
```

--------------------------------

### Scripting: Enumerate Method Start Argument

Source: https://typst.app/docs/changelog/0.7.0

The `enumerate` method on arrays now accepts a `start` argument, allowing you to specify the starting index for enumeration.

```typst
let arr = ["a", "b", "c"]
let enumerated = arr.enumerate(start: 1)
```

--------------------------------

### Set Page Numbering with Dashes (Typst)

Source: https://typst.app/docs/guides/page-setup

This example shows how to add decorative characters, such as dashes, around the page number in Typst. The `numbering` argument accepts a string where non-numeric characters are output as-is.

```Typst
#set page(numbering: "— 1 —")

This is a — numbered — page.

```

--------------------------------

### Block Element Examples

Source: https://typst.app/docs/reference/layout/block

Illustrates how to use the `block` element for styling content with backgrounds and controlling element display.

```APIDOC
## Examples

With a block, you can give a background to content while still allowing it to break across multiple pages.
```typ
#set page(height: 100pt)
#block(
  fill: luma(230),
  inset: 8pt,
  radius: 4pt,
  lorem(30),
)
```

Blocks are also useful to force elements that would otherwise be inline to become block-level, especially when writing show rules.
```typ
#show heading: it => it.body
= Blockless
More text.

#show heading: it => block(it.body)
= Blocky
More text.
```
```

--------------------------------

### Typst String Constructor Examples

Source: https://typst.app/docs/reference/foundations/str

Shows how to convert different data types to strings using the `str` constructor, including base conversion for integers.

```typst
#str(10) \n#str(4000, base: 16) \n#str(2.7) \n#str(1e8) \n#str(<intro>)
```

--------------------------------

### Add Custom Headers and Footers in Typst

Source: https://typst.app/docs/guides/page-setup

Inserts custom content into the header and footer margins of every page. The example sets a header containing text and a page number, with horizontal spacing. Content can be any valid Typst markup.

```typst
#set page(header: [_Lisa Strassner's Thesis_\n#h(1fr)\nNational Academy of Sciences])

#lorem(150)
```

--------------------------------

### Curve Example

Source: https://typst.app/docs/reference/visualize/curve

An example demonstrating the usage of the `curve` element with various segments and styling.

```APIDOC
## Example
```
#curve(
  fill: blue.lighten(80%),
  stroke: blue,
  curve.move((0pt, 50pt)),
  curve.line((100pt, 50pt)),
  curve.cubic(none, (90pt, 0pt), (50pt, 0pt)),
  curve.close(),
)

```
```

--------------------------------

### Typst Math Root Examples

Source: https://typst.app/docs/reference/math/roots

Demonstrates the usage of the `sqrt` and `root` functions for mathematical expressions in Typst.

```typst
$ sqrt(3 - 2 sqrt(2)) = sqrt(2) - 1 $
```

```typst
$ root(3, x) $
```

--------------------------------

### Basic Citation Examples

Source: https://typst.app/docs/reference/model/cite

Demonstrates different ways to cite works from a bibliography using the `@` syntax and explicit `cite` calls. Ensure a bibliography is defined using `#bibliography`.

```typst
This was already noted by
pirates long ago. @arrgh

Multiple sources say ...
@arrgh @netwok.

You can also call `cite`
explicitly. #cite(<arrgh>)

#bibliography("works.bib")
```

--------------------------------

### Heading Element Usage and Syntax

Source: https://typst.app/docs/reference/model/heading

Demonstrates how to create headings using Typst's dedicated syntax and provides an example of automatic numbering.

```APIDOC
## Heading Element

A section heading used to structure documents. Headings have a level indicating their logical role.

### Syntax
Headings are created by starting a line with one or more equals signs followed by a space. The number of equals signs determines the heading's logical nesting depth.

### Example
```typ
#set heading(numbering: "1.a.")

= Introduction
In recent years, ...

== Preliminaries
To start, ...
```
```

--------------------------------

### Typst: Set Custom Page Margins

Source: https://typst.app/docs/guides/page-setup

Configures custom top, bottom, and horizontal margins for the page using a dictionary within the `page` set rule. This example sets specific top and bottom margins and a common horizontal margin.

```typst
#set page(margin: (
  top: 3cm,
  bottom: 2cm,
  x: 1.5cm,
))

#lorem(100)

```

--------------------------------

### Set Custom Footer with Page Number (Typst)

Source: https://typst.app/docs/guides/page-setup

This example demonstrates how to create a custom footer in Typst that includes both text and the page number. When a custom footer is defined using the `footer` argument, the `numbering` argument is ignored. The page counter is accessed using `counter(page).display()`.

```Typst
#set page(footer: context [
  *American Society of Proceedings*
  #h(1fr)
  #counter(page).display(
    "1/1",
    both: true,
  )
])

This page has a custom footer.

```

--------------------------------

### Typst: Configure Page Layout with Headers and Footers

Source: https://typst.app/docs/guides/page-setup

Sets the page dimensions, paper size, and defines custom headers and footers with specific alignment. This rule should ideally be placed at the beginning of the document or template.

```typst
#set rect(
  width: 100%,
  height: 100%,
  inset: 4pt,
)

#set page(
  paper: "iso-b7",
  header: rect(fill: aqua)[Header],
  footer: rect(fill: aqua)[Footer],
  number-align: center,
)

#rect(fill: aqua.lighten(40%))

```

--------------------------------

### Reset Page Counter to 1 (Typst)

Source: https://typst.app/docs/guides/page-setup

This code snippet shows how to reset the page counter to 1 in Typst. This is useful for starting page numbering from the beginning after a title page or other introductory content. It should typically be placed at the start of a page.

```Typst
#counter(page).update(1)

```

--------------------------------

### Conditionally Set Headers on Specific Pages in Typst

Source: https://typst.app/docs/guides/page-setup

Allows for dynamic header content based on page context, such as omitting the header on the first page. It uses the 'context' keyword and checks the page counter to conditionally render content. The example skips the header if the page number is not greater than one.

```typst
#set page(header: context {
  if counter(page).get().first() > 1 [
    _Lisa Strassner's Thesis_\n    #h(1fr)\n    National Academy of Sciences
  ]
})

#lorem(150)
```

--------------------------------

### Regex Usage Examples

Source: https://typst.app/docs/reference/foundations/regex

Demonstrates how to use regular expressions with string methods like `split` and with show rules.

```APIDOC
## GET /regex/examples

### Description
Provides examples of how to use the `regex` type in Typst for common operations such as splitting strings and applying styles to matching patterns.

### Method
GET

### Endpoint
/regex/examples

### Parameters
None

### Request Example
```json
{}
```

### Response
#### Success Response (200)
- **string_split_example** (string) - An example demonstrating string splitting using regex.
- **show_rule_example** (string) - An example demonstrating the use of regex in show rules.

#### Response Example
```json
{
  "string_split_example": "// Works with string methods.\n#"a,b;c".split(regex("[,;]"))",
  "show_rule_example": "// Works with show rules.\n#show regex(\"\\d+\"): set text(red)\n\nThe numbers 1 to 10."
}
```
```

--------------------------------

### Get Current Page Number After Update (Typst)

Source: https://typst.app/docs/guides/page-setup

This snippet shows how to retrieve the actual page number after manipulating the page counter in Typst. It uses `here().page()` to get the current page's number, which might differ from the counter's internal value if it has been updated.

```Typst
#counter(page).update(n => n + 5)

// This returns one even though the
// page counter was incremented by 5.
#context here().page()

```

--------------------------------

### Set Page Columns

Source: https://typst.app/docs/reference/layout/page

Configures the page to have multiple columns. This example sets the page to 2 columns and adjusts the height.

```typst
#set page(columns: 2, height: 4.8cm)
Climate change is one of the most
pressing issues of our time, with
the potential to devastate
communities, ecosystems, and
economies around the world. It's
clear that we need to take urgent
action to reduce our carbon
emissions and mitigate the impacts
of a rapidly changing climate.


```

--------------------------------

### Set Document Columns and Gutter

Source: https://typst.app/docs/guides/page-setup

Configures the document to use a specified number of columns and adjusts the space between them. This is the primary method for applying a multi-column layout to the entire document.

```typst
#set page(columns: 2)
#set columns(gutter: 12pt)

#lorem(30)
```

--------------------------------

### Adapt Headers Based on Page Labels in Typst

Source: https://typst.app/docs/guides/page-setup

Enables headers to be adapted based on the presence of specific labels on a page, such as omitting headers on pages with large tables. It uses the 'query' system to find labels and conditionally renders the header. The example omits the header if a '<big-table>' label is not found on the current page.

```typst
#set page(header: context {
  let matches = query(<big-table>)
  let current = counter(page).get()
  let has-table = matches.any(m =>
    counter(page).at(m.location()) == current
  )

  if not has-table [
    _Lisa Strassner's Thesis_\n    #h(1fr)\n    National Academy of Sciences
  ]
})

#lorem(100)
#pagebreak()

#table(
  columns: 2 * (1fr,),
  [A], [B],
  [C], [D],
) <big-table>
```

--------------------------------

### Example Usage of a Typst Template with Named Arguments

Source: https://typst.app/docs/tutorial/making-a-template

An example showing how to call a Typst template function with named arguments for title, authors, and abstract. This snippet illustrates the desired end-state usage of the `conf` template before it's refactored into a separate file.

```typst
#show: doc => conf(
  title: [
    A Fluid Dynamic Model for
    Glacier Flow
  ],
  authors: (
    (
      name: "Theresa Tungsten",
      affiliation: "Artos Institute",
      email: "tung@artos.edu",
    ),
    (
      name: "Eugene Deklan",
      affiliation: "Honduras State",
      email: "e.deklan@hstate.hn",
    ),
  ),
  abstract: lorem(80),
  doc,
)

...

```

--------------------------------

### Apply One-Off Page Settings in Typst

Source: https://typst.app/docs/guides/page-setup

Illustrates how to apply temporary page setting overrides, such as flipping the page orientation or changing margins and columns, for a specific section of content. The `page` function is called as a function with content and override arguments.

```typst
#page(flipped: true)[
  = Multiplication table

  #table(
    columns: 5 * (1fr,),
    ..for x in range(1, 10) {
      for y in range(1, 6) {
        (str(x*y),)
      }
    }
  )
]
```

--------------------------------

### Create and Use a Dictionary

Source: https://typst.app/docs/reference/foundations/dictionary

Demonstrates creating a dictionary with key-value pairs, accessing values using dot notation and `.at()`, checking for key existence, and modifying the dictionary. Use this for general dictionary manipulation.

```typst
#let dict = (
  name: "Typst",
  born: 2019,
)

#dict.name 
#dict.at("born") 
#dict.insert("city", "Berlin")
#("name" in dict)
```

--------------------------------

### Set Page Binding Direction in Typst

Source: https://typst.app/docs/guides/page-setup

Specifies the binding direction for a document, overriding Typst's default based on script direction. This is useful for books with non-standard binding, such as right-bound English manga. The example sets the binding to 'right' for a Spanish document.

```typst
#set text(lang: "es")
#set page(binding: right)
```

--------------------------------

### Typst `cases` Element Example

Source: https://typst.app/docs/reference/math/cases

Demonstrates the basic syntax and usage of the `cases` element for conditional logic in Typst.

```typst
$ f(x, y) := cases(
  1 "if" (x dot y)/2 <= 0,
  2 "if" x "is even",
  3 "if" x in NN,
  4 "else",
) $
```

--------------------------------

### Mat Element Augmentation Examples

Source: https://typst.app/docs/reference/math/mat

Provides examples of using the `augment` parameter to draw augmentation lines, including single vertical lines and multiple lines with custom strokes.

```APIDOC
### `augment` Parameter Examples

#### Single Vertical Augmentation Line

```typst
$ mat(1, 0, 1; 0, 1, 2; augment: #2) $
// Equivalent to:
$ mat(1, 0, 1; 0, 1, 2; augment: #(-1)) $
```

#### Multiple Augmentation Lines with Custom Stroke

```typst
$ mat(0, 0, 0; 1, 1, 1; augment: #(hline: 1, stroke: 2pt + green)) $
```
```

--------------------------------

### Create Two-Column Layout with Title in Typst

Source: https://typst.app/docs/guides/page-setup

Demonstrates how to create a document with a two-column main body while allowing a title or abstract to span the full width. It uses the `place` function with `float: true` and `scope: "parent"` to temporarily exit the column layout.

```typst
#set page(columns: 2)
#set par(justify: true)

#place(
  top + center,
  float: true,
  scope: "parent",
  text(1.4em, weight: "bold")[
    Impacts of Odobenidae
  ],
)

== About seals in the wild
#lorem(80)
```

--------------------------------

### Theorem Counter Example

Source: https://typst.app/docs/reference/introspection/counter

Provides an example of defining a custom counter for theorems. It shows how to step the counter before displaying it within a theorem macro, ensuring correct numbering.

```typst
#let c = counter("theorem")
#let theorem(it) = block[
  #c.step()
  *Theorem #context c.display():*
  #it
]

#theorem[$1 = 1$]
#theorem[$2 < 3$]
```

--------------------------------

### Typst Length Unit Examples

Source: https://typst.app/docs/reference/layout/length

Demonstrates the usage of different length units and arithmetic operations with lengths in Typst.

```typst
#rect(width: 20pt)
#rect(width: 2em)
#rect(width: 1in)
```

```typst
#(3em + 5pt).em \
#(20pt).em \
#(40em + 2pt).abs \
#(5em).abs
```

--------------------------------

### Get Current Document Location with `here()`

Source: https://typst.app/docs/reference/introspection/here

This example demonstrates how to retrieve the current location within a Typst document using the `here()` function and then obtain its position. It requires the `context` block to establish a scope for the `here()` function.

```typst
#context [
  I am located at
  #here().position()
]

```

--------------------------------

### Cubic Bézier with no start control point

Source: https://typst.app/docs/reference/visualize/curve

Shows how to draw cubic Bézier curves where the start control point is `none`, resulting in a curve segment that starts directly towards the end control point.

```typst
#curve(
  stroke: blue,
  curve.move((0pt, 50pt)),
  // - No start control point
  // - End control point at `(20pt, 0pt)`
  // - End point at `(50pt, 0pt)`
  curve.cubic(none, (20pt, 0pt), (50pt, 0pt)),
  // - No start control point
  // - No end control point
  // - End point at `(50pt, 0pt)`
  curve.cubic(none, none, (100pt, 50pt)),
)

```

--------------------------------

### Typst: Set Custom Page Dimensions

Source: https://typst.app/docs/guides/page-setup

Defines a custom square page size using the `width` and `height` arguments within the `page` set rule. This allows for non-standard page dimensions.

```typst
#set page(width: 12cm, height: 12cm)

This page is a square.

```

--------------------------------

### Cubic Bézier with auto start control point

Source: https://typst.app/docs/reference/visualize/curve

Demonstrates using `auto` for the `control-start` parameter in `curve.cubic`. This mirrors the previous curve's end control point, creating a smooth transition.

```typst
#curve(
  stroke: blue,
  curve.move((0pt, 50pt)),
  curve.cubic(none, (20pt, 0pt), (50pt, 0pt)),
  // Passing `auto` instead of `none` means the start control point
  // mirrors the end control point of the previous curve. Mirror of
  // `(20pt, 0pt)` w.r.t `(50pt, 0pt)` is `(80pt, 0pt)`.
  curve.cubic(auto, none, (100pt, 50pt)),
)

```

--------------------------------

### Box with Padding and Outset

Source: https://typst.app/docs/reference/layout/box

Demonstrates the use of `inset` and `outset` parameters along with `fill` and `radius` to create a visually distinct inline rectangle. This example highlights how outset can affect layout.

```typst
An inline
#box(
  fill: luma(235),
  inset: (x: 3pt, y: 0pt),
  outset: (y: 3pt),
  radius: 2pt,
)[rectangle].

```

--------------------------------

### Typst `frac` Function Syntax and Parameters

Source: https://typst.app/docs/reference/math/frac

Illustrates the direct function call syntax for `math.frac` and its parameters: `content`, `content`, and `style`. Shows examples of specifying the style parameter.

```typst
$ frac(x, y, style: "vertical") $
$ frac(x, y, style: "skewed") $
$ frac(x, y, style: "horizontal") $

```

--------------------------------

### Basic Fraction Usage in Typst

Source: https://typst.app/docs/reference/math/frac

Demonstrates the basic mathematical syntax for creating fractions in Typst using the slash operator. Also shows how to use the `frac` function directly with parameters.

```typst
$ 1/2 < (x+1)/2 $
$ ((x+1)) / 2 = frac(a, b) $

```

--------------------------------

### Get inverse direction

Source: https://typst.app/docs/reference/layout/direction

Calculates and returns the inverse of a given direction using the '.inv()' method. For example, the inverse of 'ltr' is 'rtl'.

```typst
#ltr.inv()
#rtl.inv()
#ttb.inv()
#btt.inv()
```

--------------------------------

### Typst: Ratio Multiplication Examples

Source: https://typst.app/docs/reference/layout/ratio

Illustrates various multiplication operations involving ratios with different data types in Typst scripting. These examples show how ratios can be combined with lengths, angles, integers, floats, and fractions to produce new values.

```typst
27% * 10%
```

```typst
27% * 100pt
```

```typst
27% * (10% + 100pt)
```

```typst
27% * 100deg
```

```typst
27% * 2
```

```typst
27% * 0.37037
```

```typst
27% * 3fr
```

--------------------------------

### Set Enum Number Alignment

Source: https://typst.app/docs/reference/model/enum

Customize the alignment of enum numbers using the `number-align` parameter. This example sets alignment to `start + bottom`.

```typst
#set enum(number-align: start + bottom)

Here are some powers of two:
1. One
2. Two
4. Four
8. Eight
16. Sixteen
32. Thirty two

```

--------------------------------

### Setting `overline` and `underline` Extent

Source: https://typst.app/docs/reference/text/overline

Demonstrates how to use the `extent` parameter to control how far the overline extends beyond the content. This example also shows setting the `extent` for both `overline` and `underline`.

```typst
#set overline(extent: 4pt)
#set underline(extent: 4pt)
#overline(underline[Typography Today])

```

--------------------------------

### Find Index of First Pattern Match

Source: https://typst.app/docs/reference/foundations/str

Use `position` to get the starting index of the first occurrence of a pattern (string or regex) in a string. Returns `none` if the pattern is not found.

```typst
#"hello world".position("world")
```

--------------------------------

### Typst CLI: Query Metadata by Label

Source: https://typst.app/docs/reference/introspection/query

Demonstrates how to use the Typst command-line interface to query for elements by their label. This example retrieves metadata associated with the label `<note>` from an `example.typ` file.

```bash
typst query example.typ "<note>"

```

--------------------------------

### Typst Math Delimiter Examples

Source: https://typst.app/docs/reference/math/lr

Demonstrates various ways to use Typst's math delimiter functions, including automatic scaling, custom `lr` pairings, and disabling auto-scaling.

```typst
$ [a, b/2] $
$ lr(]sum_(x=1)^n], size: #50%) x $
$ abs((x + y) / 2) $
$ \{ (x / y) \} $
#set math.lr(size: 1em)
$ { (a / b), a, b in (0; 1/2] } $
```

--------------------------------

### Fractional Spacing for Inline Alignment

Source: https://typst.app/docs/reference/layout/align

Use fractional spacing with `#h(1fr)` for alignment within the same line, as the `align` function performs block-level alignment and interrupts paragraphs. This example places content at the start and end of the line.

```typst
Start #h(1fr) End


```

--------------------------------

### Dictionary Construction and Basic Usage

Source: https://typst.app/docs/reference/foundations/dictionary

Demonstrates how to construct dictionaries using parentheses with key-value pairs, access values using dot notation or the `at` method, and check for key existence.

```APIDOC
## Dictionary Construction and Basic Usage

### Description
A map from string keys to values. You can construct a dictionary by enclosing comma-separated `key: value` pairs in parentheses. The values do not have to be of the same type. Since empty parentheses already yield an empty array, you have to use the special `(:)` syntax to create an empty dictionary.

A dictionary is conceptually similar to an array, but it is indexed by strings instead of integers. You can access and create dictionary entries with the `.at()` method. If you know the key statically, you can alternatively use field access notation (`.key`) to access the value. To check whether a key is present in the dictionary, use the `in` keyword.

You can iterate over the pairs in a dictionary using a for loop. This will iterate in the order the pairs were inserted / declared initially.

### Method
N/A (Syntax)

### Endpoint
N/A (Syntax)

### Parameters
N/A (Syntax)

### Request Example
```ty
#let dict = (
  name: "Typst",
  born: 2019,
)

#dict.name
#dict.at("born")
#("name" in dict)
```

### Response
#### Success Response (200)
N/A (Output depends on usage)

#### Response Example
```ty
Typst
2019
true
```
```

--------------------------------

### Basic Page Break Example

Source: https://typst.app/docs/reference/layout/pagebreak

Demonstrates a simple manual page break. This element should not be used inside containers.

```typst
The next page contains
more details on compound theory.
#pagebreak()

== Compound Theory
In 1984, the first ...
```

--------------------------------

### Basic `underline` Element Usage

Source: https://typst.app/docs/reference/text/underline

Demonstrates the fundamental usage of the `underline` element to emphasize text. No special setup is required.

```typst
This is #underline[important].
```

--------------------------------

### Content Type and Basic Usage

Source: https://typst.app/docs/reference/foundations/content

Explains the fundamental 'content' type in Typst, how to create it using markup or functions, and provides an example of checking the type of content.

```APIDOC
## Content
A piece of document content.
This type is at the heart of Typst. All markup you write and most functions you call produce content values. You can create a content value by enclosing markup in square brackets. This is also how you pass content to functions.

### Example
```typ
Type of *Hello!* is
#type([*Hello!*])
```

Content can be added with the `+` operator, joined together and multiplied with integers. Wherever content is expected, you can also pass a string or `none`.
```

--------------------------------

### Create Typst Versions

Source: https://typst.app/docs/reference/foundations/version

Demonstrates various ways to construct version objects using the `version` constructor with different numbers of components and input formats.

```typst
#version() \
```

```typst
#version(1) \
```

```typst
#version(1, 2, 3, 4) \
```

```typst
#version((1, 2, 3, 4)) \
```

```typst
#version((1, 2), 3)
```

--------------------------------

### Typst Link Element Examples

Source: https://typst.app/docs/reference/model/link

Provides examples of creating links to different destinations: mailto, internal document labels, and specific page coordinates. The `body` parameter is shown for internal links.

```typst
= Introduction <intro>
#link("mailto:hello@typst.app") 
#link(<intro>)[Go to intro] 
#link((page: 1, x: 0pt, y: 0pt))[
  Go to top
]
```

--------------------------------

### Set Page Numbering and Margins

Source: https://typst.app/docs/reference/layout/page

Configures page numbering format and margins. This example sets the page height, top and bottom margins, and a simple page numbering format.

```typst
#set page(
  height: 100pt,
  margin: (top: 16pt, bottom: 24pt),
  numbering: "1 / 1",
)

#lorem(48)


```

--------------------------------

### Typst `repr` Function Examples

Source: https://typst.app/docs/reference/foundations/repr

Demonstrates the string representation of various Typst values using the `repr` function, including none, strings, tuples, and content. Also shows examples for debugging purposes, illustrating how `repr` handles calculations and anonymous functions.

```typst
#none vs #repr(none) \
#"hello" vs #repr("hello") \
#(1, 2) vs #repr((1, 2)) \
#[*Hi*] vs #repr([*Hi*])

#assert(2pt / 3 < 0.67pt)
#repr(2pt / 3)

#repr(x => x + 1)
```

--------------------------------

### 2D Alignment Example

Source: https://typst.app/docs/reference/layout/alignment

Shows how to combine two alignment values using the '+' operator to achieve 2D alignment, such as top-right.

```typst
#set page(height: 3cm)
#align(center + bottom)[Hi]
```

--------------------------------

### Load and Use Plugin with Transition API

Source: https://typst.app/docs/reference/foundations/plugin

Demonstrates loading a WASM plugin and using the transition API to call a mutable function. It asserts the state of the original and mutated modules to show the effect of the transition.

```typst
#let base = plugin("hello-mut.wasm")
#assert.eq(base.get(), "[]")

#let mutated = plugin.transition(base.add, "hello")
#assert.eq(base.get(), "[]")
#assert.eq(mutated.get(), "[hello]")


```

--------------------------------

### Configure Heading Offset and Numbering

Source: https://typst.app/docs/reference/model/heading

Demonstrates how to adjust the starting depth of headings using the `offset` parameter and customize numbering. The first heading is level 1, the second is effectively level 2 due to the offset, and the third is manually set to level 4.

```typst
= Level 1

#set heading(offset: 1, numbering: "1.1")
= Level 2

#heading(offset: 2, depth: 2)[I'm level 4]
```

--------------------------------

### Typst: Set Page Size to US Letter

Source: https://typst.app/docs/guides/page-setup

Changes the default page size to US Letter format using the `page` set rule. This is useful for documents intended for US audiences.

```typst
#set page("us-letter")

This page likes freedom.

```

--------------------------------

### Set Page Numbering with Supplement

Source: https://typst.app/docs/reference/layout/page

Configures page numbering with a supplement prefix. This example sets a numbering pattern and adds 'p.' as a supplement for page references.

```typst
#set page(numbering: "1.", supplement: [p.])

= Introduction <intro>
We are on #ref(<intro>, form: "page")!


```

--------------------------------

### Plugin Resources

Source: https://typst.app/docs/reference/foundations/plugin

Information on additional resources available for developing Typst plugins, including example implementations and development wrappers.

```APIDOC
## Resources

For more resources, check out the wasm-minimal-protocol repository. It contains:
  * A list of example plugin implementations and a test runner for these examples
  * Wrappers to help you write your plugin in Rust (Zig wrapper in development)
  * A stubber for WASI
```

--------------------------------

### Version Constructor

Source: https://typst.app/docs/reference/foundations/version

Demonstrates how to create new version objects in Typst.

```APIDOC
## Version Constructor

Creates a new version. It can have any number of components (even zero).

### Usage
```
#version()
#version(1)
#version(1, 2, 3, 4)
#version((1, 2, 3, 4))
#version((1, 2), 3)
```

### Parameters
#### `components`
- `int` or `array` - Required Positional, Variadic
  The components of the version (array arguments are flattened).

### Example Use Case
Comparing the current version (`sys.version`) to a specific one:
```
Current version: #sys.version
#(sys.version >= version(0, 14, 0))
#(version(3, 2, 0) > version(4, 1, 0))
```
```

--------------------------------

### Use Columns within Nested Layouts in Typst

Source: https://typst.app/docs/guides/page-setup

Shows how to apply a column layout directly within a nested element, such as a rectangle. This method is intended for use within specific containers rather than for page-level layout.

```typst
#rect(
  width: 6cm,
  height: 3.5cm,
  columns(2, gutter: 12pt)[
    In the dimly lit gas station,
    a solitary taxi stood silently,
    its yellow paint fading with
    time. Its windows were dark,
    its engine idle, and its tires
    rested on the cold concrete.
  ]
)
```

--------------------------------

### Typst String Manipulation Examples

Source: https://typst.app/docs/reference/foundations/str

Demonstrates various string manipulation techniques including splitting, pattern matching with regex, and basic string operations.

```typst
#"hello world!" \n#"\"hello\n  world\""! \n#"1 2 3".split() \n#"1,2;3".split(regex("[,;]")) \n#(regex("\\d+") in "ten euros") \n#(regex("\\d+") in "10 euros")
```

--------------------------------

### Get Length of Bytes

Source: https://typst.app/docs/reference/foundations/bytes

Demonstrates how to get the total number of bytes in a bytes sequence using the `len()` method.

```typst
self.len()
```

--------------------------------

### Create a Basic Typst Table

Source: https://typst.app/docs/reference/model/table

Demonstrates the creation of a Typst table with specified columns, alignment, and content including images and mathematical formulas. Use this for arranging structured data.

```typst
#table(
  columns: (1fr, auto, auto),
  inset: 10pt,
  align: horizon,
  table.header(
    [], [*Volume*], [*Parameters*],
  ),
  image("cylinder.svg"),
  $ pi h (D^2 - d^2) / 4 $,
  [
    $h$: height \ 
    $D$: outer radius \ 
    $d$: inner radius
  ],
  image("tetrahedron.svg"),
  $ sqrt(2) / 12 a^3 $,
  [$a$: edge length]
)

```

--------------------------------

### Set Page Numbering with Total Pages (Typst)

Source: https://typst.app/docs/guides/page-setup

This snippet illustrates how to display both the current page number and the total number of pages in a Typst document. This is achieved by including a second number character in the `numbering` pattern string.

```Typst
#set page(numbering: "1 of 1")

This is one of many numbered pages.

```

--------------------------------

### Set Basic Page Numbering (Typst)

Source: https://typst.app/docs/guides/page-setup

This snippet demonstrates the simplest way to add page numbers to a Typst document using the `numbering` argument within the `page` set rule. It inserts a single Arabic numeral at the center of the footer.

```Typst
#set page(numbering: "1")

This is a numbered page.

```

--------------------------------

### Typst `ceil` Function Example

Source: https://typst.app/docs/reference/math/lr

Demonstrates the use of the `ceil` function to compute the ceiling of an expression.

```typst
$ ceil(x/2) $
```

--------------------------------

### Box with Inset and Text

Source: https://typst.app/docs/reference/layout/box

Provides an example of using the `inset` parameter for a box containing text, demonstrating tight padding. This uses the `rect` function which is related to box styling.

```typst
#rect(inset: 0pt)[Tight]

```

--------------------------------

### Applying `smallcaps` to Headings

Source: https://typst.app/docs/reference/text/smallcaps

Apply smallcaps formatting to all headings using a show rule. This example also centers headings and disables bold text.

```typst
#set par(justify: true)\n#set heading(numbering: "I.")\n\n#show heading: smallcaps\n#show heading: set align(center)\n#show heading: set text(\n  weight: "regular"\n)\n\n= Introduction\n#lorem(40)
```

--------------------------------

### Stroke Example

Source: https://typst.app/docs/reference/visualize/stroke

Demonstrates various ways to define and use strokes in Typst.

```APIDOC
## Example

```typ
#set line(length: 100%)
#stack(
  spacing: 1em,
  line(stroke: 2pt + red),
  line(stroke: (paint: blue, thickness: 4pt, cap: "round")),
  line(stroke: (paint: blue, thickness: 1pt, dash: "dashed")),
  line(stroke: 2pt + gradient.linear(..color.map.rainbow)),
)
```

## Dash Pattern Example

```typ
#set line(length: 100%, stroke: 2pt)
#stack(
  spacing: 1em,
  line(stroke: (dash: "dashed")),
  line(stroke: (dash: (10pt, 5pt, "dot", 5pt))),
  line(stroke: (dash: (array: (10pt, 5pt, "dot", 5pt), phase: 10pt))),
)
```
```

--------------------------------

### Increment Page Counter by a Value (Typst)

Source: https://typst.app/docs/guides/page-setup

This Typst code demonstrates how to increment the page counter by a specific value, effectively skipping page numbers. It uses a function passed to the `update` method, where `n` represents the current page counter value.

```Typst
#counter(page).update(n => n + 5)

```

--------------------------------

### Typst CLI: Query with Format and Proxy

Source: https://typst.app/docs/changelog/0.8.0

Shows how to use the `typst query` command with the `--format` argument to specify output format and how the CLI respects proxy configurations and custom CA certificates.

```bash
# Set proxy environment variables
export HTTP_PROXY=http://your-proxy.com:8080
export HTTPS_PROXY=http://your-proxy.com:8080

# Query with specified format and custom CA certificate
typst query --format json --cert /path/to/ca.crt "./path/to/your/project"
```

--------------------------------

### Typst `pad` Element Example

Source: https://typst.app/docs/reference/layout/pad

Demonstrates how to use the `pad` element in Typst to add horizontal padding around content. This example sets the horizontal padding to 16pt and applies it to an image, followed by text.

```typst
#set align(center)

#pad(x: 16pt, image("typing.jpg"))
_Typing speeds can be
 measured in words per minute._
```

--------------------------------

### Typst Numbering Examples

Source: https://typst.app/docs/reference/model/numbering

Demonstrates various ways to use the `numbering` function with different patterns and a custom function.

```typst
#numbering("1.1)", 1, 2, 3) 
#numbering("1.a.i", 1, 2) 
#numbering("I – 1", 12, 2) 
#numbering(
  (..nums) => nums
    .pos()
    .map(str)
    .join(".") + ")",
  1, 2, 3,
)
```

--------------------------------

### Set Custom Footer with Dynamic Circles (Typst)

Source: https://typst.app/docs/guides/page-setup

This Typst snippet shows an advanced custom footer where the number of circles displayed is dynamically determined by the current page number. It uses `counter(page).get()` to retrieve the page number and generates an array of circles, joined with spacing.

```Typst
#set page(footer: context [
  *Fun Typography Club*
  #h(1fr)
  #let (num,) = counter(page).get()
  #let circles = num * (
    box(circle(
      radius: 2pt,
      fill: navy,
    )),
  )
  #box(
    inset: (bottom: 1pt),
    circles.join(h(1pt))
  )
])

This page has a custom footer.

```

--------------------------------

### Link Element Syntax and Usage

Source: https://typst.app/docs/reference/model/link

Demonstrates the basic syntax for creating links using the `link` function and automatic URL detection. It also shows how to style links using `show` rules.

```APIDOC
## `link` Element

Element functions can be customized with `set` and `show` rules. Links to a URL or a location in the document. By default, links do not look any different from normal text. However, you can easily apply a style of your choice with a show rule.

### Example
```typ
#show link: underline

https://example.com \ 
#link("https://example.com") \ 
#link("https://example.com")[See example.com]
```

## Syntax
This function also has dedicated syntax: Text that starts with `http://` or `https://` is automatically turned into a link.

## Hyphenation
If you enable hyphenation or justification, by default, it will not apply to links to prevent unwanted hyphenation in URLs. You can opt out of this default via `show link: set text(hyphenate: true)`.

## Accessibility
The destination of a link should be clear from the link text itself, or at least from the text immediately surrounding it. In PDF export, Typst will automatically generate a tooltip description for links based on their destination. For links to URLs, the URL itself will be used as the tooltip.

## Links in HTML export
In HTML export, a link to a label or location will be turned into a fragment link to a named anchor point. To support this, targets without an existing ID will automatically receive an ID in the DOM. How this works varies by which kind of HTML node(s) the link target turned into:
  * If the link target turned into a single HTML element, that element will receive the ID. This is, for instance, typically the case when linking to a top-level heading (which turns into a single `<h2>` element).
  * If the link target turned into a single text node, the node will be wrapped in a `<span>`, which will then receive the ID.
  * If the link target turned into multiple nodes, the first node will receive the ID.
  * If the link target turned into no nodes at all, an empty span will be generated to serve as a link target.

If you rely on a specific DOM structure, you should ensure that the link target turns into one or multiple elements, as the compiler makes no guarantees on the precise segmentation of text into text nodes.
If present, the automatic ID generation tries to reuse the link target's label to create a human-readable ID. A label can be reused if:
  * All characters are alphabetic or numeric according to Unicode, or a hyphen, or an underscore.
  * The label does not start with a digit or hyphen.

These rules ensure that the label is both a valid CSS identifier and a valid URL fragment for linking.
As IDs must be unique in the DOM, duplicate labels might need disambiguation when reusing them as IDs. The precise rules for this are as follows:
  * If a label can be reused and is unique in the document, it will directly be used as the ID.
  * If it's reusable, but not unique, a suffix consisting of a hyphen and an integer will be added. For instance, if the label `<mylabel>` exists twice, it would turn into `mylabel-1` and `mylabel-2`.
  * Otherwise, a unique ID of the form `loc-` followed by an integer will be generated.
```

--------------------------------

### Create and Display Datetime

Source: https://typst.app/docs/reference/foundations/datetime

Create a datetime object with a specific date and display it in different formats. Use `datetime.today` to get the current date.

```typst
#let date = datetime(
  year: 2020,
  month: 10,
  day: 4,
)

#date.display() 
#date.display(
  "y:[year repr:last_two]"
)

#let time = datetime(
  hour: 18,
  minute: 2,
  second: 23,
)

#time.display() 
#time.display(
  "h:[hour repr:12][period]"
)
```

--------------------------------

### String Length

Source: https://typst.app/docs/reference/foundations/str

Demonstrates how to get the length of a string in UTF-8 bytes.

```APIDOC
## Method: len()

The length of the string in UTF-8 encoded bytes.

### Method Signature
`self.len() -> int`

### Example
```
#let s = "hello"
#s.len() // outputs 5
```
```

--------------------------------

### List Element Usage and Syntax

Source: https://typst.app/docs/reference/model/list

Demonstrates how to use the list element with different content types and explains the markup syntax for creating list items.

```APIDOC
## `list` Element

Element functions can be customized with `set` and `show` rules. 
A bullet list.
Displays a sequence of items vertically, with each item introduced by a marker.

## Example
```typ
Normal list.
- Text
- Math
- Layout
- ...

Multiple lines.
- This list item spans multiple
  lines because it is indented.

Function call.
#list(
  [Foundations],
  [Calculate],
  [Construct],
  [Data Loading],
)
```

## Syntax
This functions also has dedicated syntax: Start a line with a hyphen, followed by a space to create a list item. A list item can contain multiple paragraphs and other block-level content. All content that is indented more than an item's marker becomes part of that item.
```

--------------------------------

### Set Alternating Page Margins in Typst

Source: https://typst.app/docs/guides/page-setup

Configures different margins for inside and outside edges of pages, useful for book binding. The 'inside' margin faces the spine, and 'outside' faces the book's edge. This rule sets horizontal margins to 2.5cm for the inside and 2cm for the outside, with a y-margin of 1.75cm.

```typst
#set page(margin: (inside: 2.5cm, outside: 2cm, y: 1.75cm))
```

--------------------------------

### direction.from()

Source: https://typst.app/docs/reference/layout/direction

The `from` definition returns a direction based on a starting point (left, right, top, or bottom).

```APIDOC
## direction.from()

### Description
Returns a direction from a starting point.

### Syntax
`direction.from(alignment) -> direction`

### Parameters
#### Positional Parameters
* `alignment` (alignment) - Required - Specifies the starting point (left, right, top, or bottom).

### Usage Examples
```
#direction.from(left)
#direction.from(right)
#direction.from(top)
#direction.from(bottom)
```
```

--------------------------------

### Mat Element Set Rule Examples

Source: https://typst.app/docs/reference/math/mat

Illustrates how to use `set` rules to customize parameters like `delim`, `align`, `row-gap`, and `column-gap` for the `mat` element.

```APIDOC
### `set` Rule Examples

#### `delim` Example

```typst
#set math.mat(delim: "[")
$ mat(1, 2; 3, 4) $
```

#### `align` Example

```typst
#set math.mat(align: right)
$ mat(-1, 1, 1; 1, -1, 1; 1, 1, -1) $
```

#### `gap` Example

```typst
#set math.mat(gap: 1em)
$ mat(1, 2; 3, 4) $
```

#### `row-gap` Example

```typst
#set math.mat(row-gap: 1em)
$ mat(1, 2; 3, 4) $
```

#### `column-gap` Example

```typst
#set math.mat(column-gap: 1em)
$ mat(1, 2; 3, 4) $
```
```

--------------------------------

### Typst `abs` Function Example

Source: https://typst.app/docs/reference/math/lr

Shows how to use the `abs` function to calculate the absolute value of an expression.

```typst
$ abs(x/2) $
```

--------------------------------

### Typst: Using 'after' for Temporal Selection

Source: https://typst.app/docs/reference/foundations/selector

Explains the `after` method, which creates a selector matching elements that appear after a specified start point. The `inclusive` parameter determines if the start element is part of the match.

```typst
self.after(start: <label>, inclusive: false)

```

--------------------------------

### Content Definitions: func, has, at, fields, location

Source: https://typst.app/docs/reference/foundations/content

Documents the definitions available for content elements, including `func` to get the element function, `has` to check for fields, `at` to access fields, `fields` to list all fields, and `location` to get the content's position.

```APIDOC
## Definitions
Functions and types can have associated definitions. These are accessed by specifying the function or type, followed by a period, and then the definition's name.

### `func`
The content's element function. This function can be used to create the element contained in this content. It can be used in set and show rules for the element. Can be compared with global functions to check whether you have a specific kind of element.
```typ
self.func(
) -> function
```

### `has`
Whether the content has the specified field.
```typ
self.has(
str
) -> bool
```
#### `field`
str
Required Positional
Positional parameters are specified in order, without names.
The field to look for.

### `at`
Access the specified field on the content. Returns the default value if the field does not exist or fails with an error if no default value was specified.
```typ
self.at(
str,
default: any,
) -> any
```
#### `field`
str
Required Positional
Positional parameters are specified in order, without names.
The field to access.
#### `default`
any
A default value to return if the field does not exist.

### `fields`
Returns the fields of this content.
```typ
#rect(
  width: 10cm,
  height: 10cm,
).fields()
```

```typ
self.fields(
) -> dictionary
```

### `location`
The location of the content. This is only available on content returned by query or provided by a show rule, for other content it will be `none`. The resulting location can be used with counters, state and queries.
```typ
self.location(
) -> nonelocation
```
```

--------------------------------

### Mat Element Variadic Rows Example

Source: https://typst.app/docs/reference/math/mat

Demonstrates how to use the variadic `rows` parameter to construct a matrix from an array of arrays.

```APIDOC
### Variadic `rows` Parameter Example

```typst
#let data = ((1, 2, 3), (4, 5, 6))
#let matrix = math.mat(..data)
$ v := matrix $
```
```

--------------------------------

### Array Construction and Basic Usage

Source: https://typst.app/docs/reference/foundations/array

Demonstrates how to construct arrays, access elements, modify them, and perform common operations like finding, filtering, mapping, and joining.

```APIDOC
## Array Construction and Basic Usage

### Description
Arrays are sequences of values that can be constructed using parentheses and comma-separated items. They support various operations for accessing, modifying, and transforming elements.

### Example
```
#let values = (1, 7, 4, -3, 2)

#values.at(0) 
#(values.at(0) = 3)
#values.at(-1) 
#values.find(calc.even) 
#values.filter(calc.odd) 
#values.map(calc.abs) 
#values.rev() 
#(1, (2, 3)).flatten() 
#( ("A", "B", "C")
    .join(", ", last: " and "))
```

### Notes on Array Construction
- An array of length one requires a trailing comma: `(1,)`.
- An empty array is written as `()`.
```

--------------------------------

### Read File as Bytes and Process

Source: https://typst.app/docs/reference/foundations/bytes

Shows how to read a file's content as raw bytes using `encoding: none`. It then demonstrates slicing the bytes to extract magic bytes and a portion of the data as a string.

```typst
#let data = read(
  "rhino.png",
  encoding: none,
)

// Magic bytes.
#array(data.slice(0, 4)) \
#str(data.slice(1, 4))
```

--------------------------------

### Scripting: Direction Methods

Source: https://typst.app/docs/changelog/0.7.0

The `axis`, `start`, `end`, and `inv` methods are now available for directions, offering more control over directional properties.

```typst
let dir = left
let axis = dir.axis()
let start_point = dir.start()
let end_point = dir.end()
let inverted_dir = dir.inv()
```

--------------------------------

### Create a video player

Source: https://typst.app/docs/reference/html/typed

Use the `video` function to embed a video player. It supports various attributes for controlling playback, appearance, and sources.

```typst
html.video(autoplay: bool, controls: bool, crossorigin: str, height: int, loop: bool, muted: bool, playsinline: bool, poster: str, preload: noneautostr, src: str, width: int, content)
```

--------------------------------

### ARIA Attributes Reference

Source: https://typst.app/docs/reference/html/typed

A reference guide to ARIA attributes, their types, and descriptions.

```APIDOC
## ARIA Attributes Reference

This section provides documentation for various ARIA attributes.

### `aria-colcount`
**Type:** int
**Description:** Defines the total number of columns in a table, grid, or treegrid. See related `aria-colindex`.

### `aria-colindex`
**Type:** int
**Description:** Defines an element's column index or position with respect to the total number of columns within a table, grid, or treegrid. See related `aria-colcount` and `aria-colspan`.

### `aria-colspan`
**Type:** int
**Description:** Defines the number of columns spanned by a cell or gridcell within a table, grid, or treegrid. See related `aria-colindex` and `aria-rowspan`.

### `aria-controls`
**Type:** str or array
**Description:** Identifies the element (or elements) whose contents or presence are controlled by the current element. See related `aria-owns`.

### `aria-current`
**Type:** bool or str
**Description:** Indicates the element that represents the current item within a container or set of related elements.
**Variants:**
- `"page"`: Represents the current page within a set of pages.
- `"step"`: Represents the current step within a process.
- `"location"`: Represents the current location within an environment or context.
- `"date"`: Represents the current date within a collection of dates.
- `"time"`: Represents the current time within a set of times.

### `aria-describedby`
**Type:** str or array
**Description:** Identifies the element (or elements) that describes the object. See related `aria-labelledby`.

### `aria-details`
**Type:** str
**Description:** Identifies the element that provides a detailed, extended description for the object. See related `aria-describedby`.

### `aria-disabled`
**Type:** bool
**Description:** Indicates that the element is perceivable but disabled, so it is not editable or otherwise operable. See related `aria-hidden` and `aria-readonly`.

### `aria-errormessage`
**Type:** str
**Description:** Identifies the element that provides an error message for the object. See related `aria-invalid` and `aria-describedby`.

### `aria-expanded`
**Type:** none or bool
**Description:** Indicates whether the element, or another grouping element it controls, is currently expanded or collapsed.

### `aria-flowto`
**Type:** str or array
**Description:** Identifies the next element (or elements) in an alternate reading order of content which, at the user's discretion, allows assistive technology to override the general default of reading in document source order.

### `aria-haspopup`
**Type:** bool or str
**Description:** Indicates the availability and type of interactive popup element, such as menu or dialog, that can be triggered by an element.
**Variants:**
- `"menu"`: Indicates the popup is a menu.
- `"listbox"`: Indicates the popup is a listbox.
- `"tree"`: Indicates the popup is a tree.
- `"grid"`: Indicates the popup is a grid.
- `"dialog"`: Indicates the popup is a dialog.

### `aria-hidden`
**Type:** none or bool
**Description:** Indicates whether the element is exposed to an accessibility API. See related `aria-disabled`.

### `aria-invalid`
**Type:** bool or str
**Description:** Indicates the entered value does not conform to the format expected by the application. See related `aria-errormessage`.
**Variants:**
- `"grammar"`: A grammatical error was detected.
- `"spelling"`: A spelling error was detected.

### `aria-keyshortcuts`
**Type:** str
**Description:** Indicates keyboard shortcuts that an author has implemented to activate or give focus to an element.

### `aria-label`
**Type:** str
**Description:** Defines a string value that labels the current element. See related `aria-labelledby`.

### `aria-labelledby`
**Type:** str or array
**Description:** Identifies the element (or elements) that labels the current element. See related `aria-describedby`.

### `aria-level`
**Type:** int
**Description:** Defines the hierarchical level of an element within a structure.

### `aria-live`
**Type:** str
**Description:** Indicates that an element will be updated, and describes the types of updates the user agents, assistive technologies, and user can expect from the live region.
**Variants:**
- `"assertive"`: Indicates that updates to the region have the highest priority and should be presented the user immediately.
- `"off"`: Indicates that updates to the region should not be presented to the user unless the used is currently focused on that region.
- `"polite"`: Indicates that updates to the region should be presented at the next graceful opportunity, such as at the end of speaking the current sentence or when the user pauses typing.

### `aria-modal`
**Type:** bool
**Description:** Indicates whether an element is modal when displayed.

### `aria-multiline`
**Type:** bool
**Description:** Indicates whether a text box accepts multiple lines of input or only a single line.

### `aria-multiselectable`
**Type:** bool
**Description:** Indicates that the user may select more than one item from the current selectable descendants.

### `aria-orientation`
**Type:** str
**Description:** Indicates whether the element's orientation is horizontal, vertical, or unknown/ambiguous.
**Variants:**
- `"horizontal"`: The element is oriented horizontally.
- `"undefined"`: The element's orientation is unknown/ambiguous.
- `"vertical"`: The element is oriented vertically.

### `aria-owns`
**Type:** str or array
**Description:** Identifies an element (or elements) in order to define a visual, functional, or contextual parent/child relationship between DOM elements where the DOM hierarchy cannot be used to represent the relationship. See related `aria-controls`.
```

--------------------------------

### Typst `lr` Function Example

Source: https://typst.app/docs/reference/math/lr

Illustrates the use of the `lr` function to scale custom delimiters with content, showing how to control delimiter size.

```typst
$ lr(]sum_(x=1)^n], size: #50%) x $
```

--------------------------------

### Typst Integer Constructor Examples

Source: https://typst.app/docs/reference/foundations/int

Shows how to convert various types (boolean, float, decimal, string) to integers using the int() constructor. Note that float and decimal values are rounded towards zero.

```typst
#int(false) \
#int(true) \
#int(2.7) \
#int(decimal("3.8")) \
#(int("27") + int("4"))
```

--------------------------------

### String Pattern Matching and Trimming

Source: https://typst.app/docs/reference/foundations/str

Demonstrates how to use the `pattern` functionality for searching and trimming strings. It supports searching with a string or regex, and trimming whitespace from the start, end, or both sides.

```APIDOC
## String Pattern Matching and Trimming

### Description
Searches for a pattern within a string or trims whitespace. Supports string or regex patterns. Trimming can be applied to the start, end, or both sides.

### Method
Implicit (used within other string methods)

### Parameters
#### Positional Parameters
- **pattern** (none or str or regex) - Required - The pattern to search for or trim. If `none`, trims whitespace.
- **at** (alignment) - Optional - Specifies trimming to 'start' or 'end'. Defaults to trimming both sides.
- **repeat** (bool) - Optional - Whether to repeatedly remove matches. Defaults to `true`.

### Request Example
```json
{
  "example": "trimming whitespace from both ends"
}
```

### Response
#### Success Response (200)
- **string** (str) - The modified string after pattern matching or trimming.

#### Response Example
```json
{
  "example": "trimmed string"
}
```
```

--------------------------------

### Create a samp Element

Source: https://typst.app/docs/reference/html/typed

Use html.samp to represent sample computer output. It takes content as its only parameter.

```typst
html.samp(content,)
```

--------------------------------

### Construct Tiling with Offset and Rotation

Source: https://typst.app/docs/reference/visualize/tiling

Constructs a tiling pattern where each cell's content is offset and rotated. This example demonstrates advanced customization of individual tiling cells.

```typst
#let pat = tiling(
  size: (20pt, 20pt),
  relative: "parent",
  place(
    dx: 5pt,
    dy: 5pt,
    rotate(45deg, square(
      size: 5pt,
      fill: black,
    )),
  ),
)

#rect(width: 100%, height: 60pt, fill: pat)


```

--------------------------------

### Table Element API

Source: https://typst.app/docs/reference/model/table

This section details the `table` element's parameters and provides examples of its usage.

```APIDOC
## `table` Element

### Description
A table of items used to arrange content in cells. Cells can contain arbitrary content and are specified in row-major order.

### Method
`table()`

### Endpoint
N/A (Typst element)

### Parameters
#### Settable Parameters
- **columns** (auto or int or relative or fraction or array) - Settable - The column sizes. See the grid documentation for more information on track sizing. Default: `()`
- **rows** (auto or int or relative or fraction or array) - Settable - The row sizes. See the grid documentation for more information on track sizing. Default: `()`
- **gutter** (auto or int or relative or fraction or array) - The gaps between rows and columns. This is a shorthand for setting `column-gutter` and `row-gutter` to the same value. See the grid documentation for more information on gutters. Default: `()`
- **column-gutter** (auto or int or relative or fraction or array) - The gaps between columns.
- **row-gutter** (auto or int or relative or fraction or array) - The gaps between rows.
- **inset** (relative array dictionary function) - The padding within cells.
- **align** (auto array alignment function) - The alignment of content within cells.
- **fill** (none color gradient array tiling function) - The background fill of cells.
- **stroke** (none length color gradient array tiling dictionary function) - The stroke of cell borders.

### Request Example
```typst
#table(
  columns: (1fr, auto, auto),
  inset: 10pt,
  align: horizon,
  table.header(
    [], [*Volume*], [*Parameters*],
  ),
  image("cylinder.svg"),
  $ pi h (D^2 - d^2) / 4 $,
  [
    $h$: height \ 
    $D$: outer radius \ 
    $d$: inner radius
  ],
  image("tetrahedron.svg"),
  $ sqrt(2) / 12 a^3 $,
  [$a$: edge length]
)
```

### Response
#### Success Response (200)
- **content** (content) - The rendered table.

#### Response Example
(No specific response example provided for the element itself, as it's rendered within the Typst document.)

### Accessibility
Use `table.header` and `table.footer` to mark header and footer sections for Assistive Technology. Consider summarizing table content in a figure caption.
```

--------------------------------

### Set Text Tracking (Character Spacing)

Source: https://typst.app/docs/reference/text/text

Adjusts the space added between characters. This example increases the tracking to 1.5pt.

```typst
#set text(tracking: 1.5pt)
Distant text.
```

--------------------------------

### Set justification limits in Typst

Source: https://typst.app/docs/reference/model/par

Demonstrates setting custom justification limits for word spacing and character tracking. The first example shows default limits, while the second enables character-level justification by adjusting the `tracking` property.

```typst
#let example(name) = columns(2, gutter: 10pt)[
  #place(top, float: true, scope: "parent", strong(name))
  /* Text from https://en.wikipedia.org/wiki/Anne_Bayley */
]

#set page(width: 440pt, height: 21em, margin: 15pt)
#set par(justify: true)
#set text(size: 0.8em)

#grid(
  columns: (1fr, 1fr),
  gutter: 20pt,
  {
    // These are Typst's default limits.
    set par(justification-limits: (
      spacing: (min: 100% * 2 / 3, max: 150%),
      tracking: (min: 0em, max: 0em),
    ))
    example[Word-level justification]
  },
  {
    // These are our custom character-level limits.
    set par(justification-limits: (
      tracking: (min: -0.01em, max: 0.02em),
    ))
    example[Character-level justification]
  },
)


```

--------------------------------

### Cubic Bézier with explicit start control point

Source: https://typst.app/docs/reference/visualize/curve

Shows an alternative to `auto` for the `control-start` parameter in `curve.cubic`, by explicitly providing the mirrored control point coordinates.

```typst
#curve(
  stroke: blue,
  curve.move((0pt, 50pt)),
  curve.cubic(none, (20pt, 0pt), (50pt, 0pt)),
  // `(80pt, 0pt)` is the same as `auto` in this case.
  curve.cubic((80pt, 0pt), none, (100pt, 50pt)),
)

```

--------------------------------

### Add and Cite Bibliography Entries

Source: https://typst.app/docs/reference/model/bibliography

Demonstrates how to add a bibliography from a file and cite entries using the @key syntax. The bibliography will only display entries that are referenced in the document.

```typst
This was already noted by
pirates long ago. @arrgh

Multiple sources say ...
@arrgh @netwok.

#bibliography("works.bib")

```

--------------------------------

### LaTeX Unordered List Example

Source: https://typst.app/docs/guides/for-latex-users

Provides the LaTeX code for an unordered list using the `itemize` environment and `\item` command.

```latex
\begin{itemize}
  \item Fast
  \item Flexible
  \item Intuitive
\end{itemize}
```

--------------------------------

### State Get Method

Source: https://typst.app/docs/reference/introspection/state

Retrieves the current value of the state at the current document location.

```APIDOC
## state.get()

### Description
Retrieves the current value of the state at the current location in the document. This is equivalent to `state.at(here())`.

### Parameters
None

### Request Example
```typst
#let counter = state("count", 0)
#context counter.get()
```

### Response Example
(Returns the current value of the state)
```

--------------------------------

### Configure Floating Figure Placement

Source: https://typst.app/docs/reference/model/figure

Sets up rules to configure the `placement` and `clearance` for floating figures. This example places figures at the bottom of the page with a specified clearance.

```typst
#set page(height: 200pt)
#show figure: set place(
  clearance: 1em,
)

= Introduction
#figure(
  placement: bottom,
  caption: [A glacier],
  image("glacier.jpg", width: 60%),
)
#lorem(60)
```

--------------------------------

### Typst `round` Function Example

Source: https://typst.app/docs/reference/math/lr

Illustrates the use of the `round` function to round an expression to the nearest integer.

```typst
$ round(x/2) $
```

--------------------------------

### Display Image from Raw Pixel Data

Source: https://typst.app/docs/reference/visualize/image

Render an image directly from raw pixel data. This example shows how to provide pixel data along with its format details (encoding, width, height) and desired display width.

```typst
#image(
  bytes(range(16).map(x => x * 16)),
  format: (
    encoding: "luma8",
    width: 4,
    height: 4,
  ),
  width: 2cm,
)

```

--------------------------------

### Setting Default `frac` Style with `set` Rule

Source: https://typst.app/docs/reference/math/frac

Shows how to globally set the default style for all subsequent fractions using a `set` rule. This example changes the default style to 'skewed'.

```typst
#set math.frac(style: "skewed")
$ a / b $

```

--------------------------------

### Extract Subslice of Bytes

Source: https://typst.app/docs/reference/foundations/bytes

Illustrates extracting a portion of the bytes sequence using the `slice()` method. You can specify the start and end indices, or a start index and a count.

```typst
self.slice(int,noneint,count: int)
```

--------------------------------

### Demonstrate Content Sizing with Font Size

Source: https://typst.app/docs/reference/layout/measure

Shows how the same content's rendered size changes with different font sizes.

```typst
#let content = [Hello!]
#content
#set text(14pt)
#content

```

--------------------------------

### Getting Alignment Axis

Source: https://typst.app/docs/reference/layout/alignment

Demonstrates how to use the .axis() method to determine if an alignment is horizontal, vertical, or none.

```typst
#left.axis() 
#bottom.axis()
```

--------------------------------

### Get Color Space Constructor

Source: https://typst.app/docs/reference/visualize/color

Returns the constructor function for the color's color space. This is useful for checking or verifying the color space.

```typst
#let color = cmyk(1%, 2%, 3%, 4%)
#(color.space() == cmyk)
```

--------------------------------

### Typst `norm` Function Example

Source: https://typst.app/docs/reference/math/lr

Illustrates the use of the `norm` function to denote the norm of an expression.

```typst
$ norm(x/2) $
```

--------------------------------

### Check if String Starts With Pattern

Source: https://typst.app/docs/reference/foundations/str

Use `starts-with` to determine if a string begins with a specified pattern, which can be a string or a regular expression.

```typst
#"hello world".starts-with("hello")
```

--------------------------------

### Basic `scale` Usage

Source: https://typst.app/docs/reference/layout/scale

Demonstrates basic scaling and mirroring of content using the `scale` element. The `reflow` parameter can be set to `true` to adjust layout.

```typst
#set align(center)
#scale(x: -100%)[This is mirrored.]
#scale(x: -100%, reflow: true)[This is mirrored.]

```

--------------------------------

### Getting Inverse Alignment

Source: https://typst.app/docs/reference/layout/alignment

Shows how to obtain the inverse alignment using the .inv() method for various alignment types.

```typst
#top.inv() 
#left.inv() 
#center.inv() 
#(left + bottom).inv()
```

--------------------------------

### Array Enumerate Method

Source: https://typst.app/docs/reference/foundations/array

Returns a new array with elements paired with their indices, optionally starting from a specified index.

```APIDOC
## `enumerate`

### Description
Returns a new array with the values alongside their indices. The returned array consists of `(index, value)` pairs in the form of length-2 arrays. These can be destructured with a let binding or for loop.

### Method
`self.enumerate(start: int) -> array`

### Parameters
#### Path Parameters
None

#### Query Parameters
None

#### Request Body
None

### Request Example
```
#for (i, value) in ("A", "B", "C").enumerate() {
  [#i: #value \ ]
}

#("A", "B", "C").enumerate(start: 1)
```

### Response
#### Success Response (200)
- **array** - An array of `(index, value)` pairs.

#### Response Example
None
```

--------------------------------

### Set Heading Numbering Pattern

Source: https://typst.app/docs/reference/model/heading

Applies a specific numbering pattern to headings. This example uses '1.a.' for sections, subsections, and sub-subsections.

```typst
#set heading(numbering: "1.a.")

= A section
== A subsection
=== A sub-subsection
```

--------------------------------

### Basic `place` Element Usage

Source: https://typst.app/docs/reference/layout/place

Demonstrates basic usage of the `place` element for overlaying content and relative positioning within a container. Also shows absolute positioning on the page using `top + left` alignment and offsets.

```typst
#set page(height: 120pt)
Hello, world!

#rect(
  width: 100%,
  height: 2cm,
  place(horizon + right, square()),
)

#place(
  top + left,
  dx: -5pt,
  square(size: 5pt, fill: red),
)


```

--------------------------------

### Set Text Fill Color

Source: https://typst.app/docs/reference/text/text

Applies a fill color to the text. This example sets the text color to red.

```typst
#set text(fill: red)
This text is red.
```

--------------------------------

### Styling Links with Show Rules

Source: https://typst.app/docs/reference/model/link

Demonstrates how to apply a style, such as underlining, to all `link` elements using a `show` rule. This affects how links appear in the document.

```typst
#show link: underline

https://example.com 

#link("https://example.com") 
#link("https://example.com")[
  See example.com
]
```

--------------------------------

### Set Heading Numbering

Source: https://typst.app/docs/reference/model/heading

Configure automatic numbering for headings using a numbering pattern. This example sets up a '1.a)' style numbering for sections and subsections.

```typst
#set heading(numbering: "1.a")

= Introduction
In recent years, ...

== Preliminaries
To start, ...
```

--------------------------------

### Add Vertical Lines to Typst Tables with Start Position

Source: https://typst.app/docs/guides/tables

Illustrates adding a vertical line to a Typst table that does not extend into the first row. This is achieved using `table.vline` with the `x` argument set to `1` and the `start` argument set to `1`.

```typst
// Base template already configured tables, but we need some
// extra configuration for this table.
#{
  set table(align: (x, _) => if x == 0 { left } else { right })
  show table.cell.where(x: 0): smallcaps
  table(
    columns: (auto, 1fr, 1fr, 1fr),
    table.vline(x: 1, start: 1),
    table.header[Trainset][Top Speed][Length][Weight],
    [TGV Réseau], [320 km/h], [200m], [383t],
    [ICE 403], [330 km/h], [201m], [409t],
    [Shinkansen N700], [300 km/h], [405m], [700t],
  )
}

```

--------------------------------

### Basic Box Element Usage

Source: https://typst.app/docs/reference/layout/box

Demonstrates how to use the `box` element to size an image within a paragraph. Refer to the official documentation for more advanced configurations.

```typst
Refer to the docs
#box(
  height: 9pt,
  image("docs.svg")
)
for more information.

```

--------------------------------

### Typst: Self-Affecting Query Example

Source: https://typst.app/docs/reference/introspection/query

Illustrates a potential issue with Typst queries where a query can affect its own results, leading to an unstable output that Typst eventually stops processing. This example queries for headings and generates new ones based on the count.

```typst
= Real
#context {
  let elems = query(heading)
  let count = elems.len()
  count * [= Fake]
}


```

--------------------------------

### Typst Stroke Examples

Source: https://typst.app/docs/reference/visualize/stroke

Demonstrates various ways to define strokes using different parameters like thickness, color, cap, and dash patterns. Use this for visualizing different line styles.

```typst
#set line(length: 100%)
#stack(
  spacing: 1em,
  line(stroke: 2pt + red),
  line(stroke: (paint: blue, thickness: 4pt, cap: "round")),
  line(stroke: (paint: blue, thickness: 1pt, dash: "dashed")),
  line(stroke: 2pt + gradient.linear(..color.map.rainbow)),
)

```

--------------------------------

### Customize Heading Appearance with Show Rule

Source: https://typst.app/docs/reference/model/heading

Apply custom styling to headings based on their level using a show rule. This example makes all level 2 headings red.

```typst
#show heading.where(level: 2): set text(red)

= Level 1
== Level 2

#set heading(offset: 1)
= Also level 2
== Level 3
```

--------------------------------

### Create a Hyperlink

Source: https://typst.app/docs/reference/html/typed

The `html.a` function creates a hyperlink. Key parameters include `href` for the address and `download` for specifying a filename if the resource should be downloaded.

```typst
html.a(
  download: str,
  href: str,
  hreflang: str,
  ping: strarray,
  referrerpolicy: nonestr,
  rel: strarray,
  target: str,
  type: str,
  content,
) -> content
```

--------------------------------

### Importing and Using a Typst Template with `.with` Method

Source: https://typst.app/docs/tutorial/making-a-template

Demonstrates how to import a Typst template function (`conf`) from a separate file (`conf.typ`) and apply it using the `.with` method to pre-populate named arguments. This approach keeps the main document clean and promotes template reusability.

```typst
#import "conf.typ": conf

#set document(title: [
  A Fluid Dynamic Model for
  Glacier Flow
])

#show: conf.with(
  authors: (
    (
      name: "Theresa Tungsten",
      affiliation: "Artos Institute",
      email: "tung@artos.edu",
    ),
    (
      name: "Eugene Deklan",
      affiliation: "Honduras State",
      email: "e.deklan@hstate.hn",
    ),
  ),
  abstract: lorem(80),
)

= Introduction
#lorem(90)

== Motivation
#lorem(140)

== Problem Statement
#lorem(50)

= Related Work
#lorem(200)

```

--------------------------------

### Get Page Dimensions and Measure Text with Typst Layout

Source: https://typst.app/docs/reference/layout/layout

Demonstrates how to use the `layout` function to get the page dimensions and then use those dimensions to measure the height of a text block. It highlights that `layout` forces content into a block-level container and can be combined with `measure`.

```typst
#let text = lorem(30)
#layout(size => [
  #let (height,) = measure(
    width: size.width,
    text,
  )
  This text is #height high with
  the current page width: \
  #text
])
```

--------------------------------

### Floating Elements with `place`

Source: https://typst.app/docs/reference/layout/place

Demonstrates the use of `place` with `float: true` to position elements at the top or bottom of a container, displacing in-flow content. This example shows how to add notes to the top and bottom of the page.

```typst
#set page(height: 150pt)
#let note(where, body) = place(
  center + where,
  float: true,
  clearance: 6pt,
  rect(body),
)

#lorem(10)
#note(bottom)[Bottom 1]
#note(bottom)[Bottom 2]
#lorem(40)
#note(top)[Top]
#lorem(10)


```

--------------------------------

### Creating Custom Symbols with Variants

Source: https://typst.app/docs/reference/foundations/symbol

Illustrates how to define a custom symbol using the 'symbol' constructor, providing a base symbol and several variants with specific modifiers.

```typst
#let envelope = symbol(
  "🖂",
  ("stamped", "🖃"),
  ("stamped.pen", "🖆"),
  ("lightning", "🖄"),
  ("fly", "🖅"),
)

#envelope
#envelope.stamped
#envelope.stamped.pen
#envelope.lightning
#envelope.fly
```

--------------------------------

### Manually Enable OpenType Features

Source: https://typst.app/docs/reference/text/text

Apply raw OpenType features by providing a tuple of feature names to the `features` parameter. This example enables the `frac` feature.

```typst
#set text(features: ("frac",))
1/2
```

--------------------------------

### Styling Grid Cells with Various Methods

Source: https://typst.app/docs/reference/layout/grid

Illustrates different methods for styling a grid, including single values, arrays, and functions for alignment, inset, fill, and stroke. It also shows how to apply styling to individual cells.

```typst
#grid(
  columns: 5,

  // By a single value
  align: center,
  // By a single but more complicated value
  inset: (x: 2pt, y: 3pt),
  // By an array of values (cycling)
  fill: (rgb("#239dad50"), none),
  // By a function that returns a value
  stroke: (x, y) => if calc.rem(x + y, 3) == 0 { 0.5pt },

  ..range(5 * 3).map(n => numbering("A", n + 1))
)


```

--------------------------------

### Typst: Constructing and Querying with Selectors

Source: https://typst.app/docs/reference/foundations/selector

Demonstrates how to construct a selector by combining element functions and their properties, and then use this selector with the `query` function to find specific elements in the document. This example selects headings of level 1 or 2.

```typst
#context query(
  heading.where(level: 1)
    .or(heading.where(level: 2))
)

= This will be found
== So will this
=== But this will not.

```

--------------------------------

### Create Unnamed Function for Show Rule

Source: https://typst.app/docs/reference/foundations/function

Illustrates the creation of an unnamed function using `=>` syntax, which is useful for show rules or settable properties. This example doubles the output of 'once?'.

```typst
#show "once?": it => [#it #it]
once?
```

--------------------------------

### Enumerate Array Elements with Indices

Source: https://typst.app/docs/reference/foundations/array

Returns a new array containing (index, value) pairs. The starting index can be customized.

```typst
#for (i, value) in ("A", "B", "C").enumerate() {
  [#i: #value \ ]
}
```

```typst
#("A", "B", "C").enumerate(start: 1)
```

--------------------------------

### Link Function Parameters

Source: https://typst.app/docs/reference/model/link

Details the parameters accepted by the `link` function, including destination (`dest`) and body content, with examples for various link types.

```APIDOC
## Parameters

Parameters are the inputs to a function. They are specified in parentheses after the function name.

`link(str | label | location | dictionary, content,) -> content`

### `dest`
`str` or `label` or `location` or `dictionary`
Required Positional
Positional parameters are specified in order, without names.
The destination the link points to.
  * To link to web pages, `dest` should be a valid URL string. If the URL is in the `mailto:` or `tel:` scheme and the `body` parameter is omitted, the email address or phone number will be the link's body, without the scheme.
  * To link to another part of the document, `dest` can take one of three forms:
    * A label attached to an element. If you also want automatic text for the link based on the element, consider using a reference instead.
    * A `location` (typically retrieved from `here`, `locate` or `query`).
    * A dictionary with a `page` key of type integer and `x` and `y` coordinates of type length. Pages are counted from one, and the coordinates are relative to the page's top left corner.

```typ
= Introduction <intro>
#link("mailto:hello@typst.app") \ 
#link(<intro>)[Go to intro] \ 
#link((page: 1, x: 0pt, y: 0pt))[Go to top]
```

### `body`
`content`
Required Positional
Positional parameters are specified in order, without names.
The content that should become a link.
If `dest` is an URL string, the parameter can be omitted. In this case, the URL will be shown as the link.
```

--------------------------------

### String Constructor

Source: https://typst.app/docs/reference/foundations/str

Shows how to convert various data types into strings using the `str` constructor, with an optional base parameter for integers.

```APIDOC
## Constructor: str()

Converts a value to a string. Integers can be formatted in a specified base. Floats are formatted in base 10 without exponential notation. Negative numbers use the Unicode minus sign. Bytes are decoded as UTF-8.

### Parameters
#### `value`
int or float or str or bytes or label or decimal or version or type
Required Positional
The value that should be converted to a string.

#### `base`
int
The base (radix) to display integers in, between 2 and 36. Default: `10`

### Example
```
#str(10)
#str(4000, base: 16)
#str(2.7)
#str(1e8)
#str(<intro>)
```
```

--------------------------------

### Get Duration in Weeks

Source: https://typst.app/docs/reference/foundations/duration

Retrieve the total duration expressed in weeks as a floating-point number.

```typst
self.weeks(
) -> float

```

--------------------------------

### Get Duration in Days

Source: https://typst.app/docs/reference/foundations/duration

Retrieve the total duration expressed in days as a floating-point number.

```typst
self.days(
) -> float

```

--------------------------------

### Basic Ref Element Usage

Source: https://typst.app/docs/reference/model/ref

Demonstrates basic usage of `ref` for headings, bibliography entries, and page references. Ensure elements are labeled correctly for referencing.

```typst
#set page(numbering: "1")
#set heading(numbering: "1.")
#set math.equation(numbering: "(1)")

= Introduction <intro>
Recent developments in
typesetting software have
rekindled hope in previously
frustrated researchers. @distress
As shown in @results (see
#ref(<results>, form: "page")),
we ...

= Results <results>
We discuss our approach in
comparison with others.

== Performance <perf>
@slow demonstrates what slow
software looks like.
$ T(n) = O(2^n) $ <slow>

#bibliography("works.bib")

```

--------------------------------

### Create a Range of Numbers

Source: https://typst.app/docs/reference/foundations/array

Generates an array of numbers. Can specify start, end, and step. If only one argument is provided, it's treated as the end.

```typst
#range(5)
#range(2, 5)
#range(20, step: 4)
#range(21, step: 4)
#range(5, 2, step: -1)
```

--------------------------------

### Get Duration in Hours

Source: https://typst.app/docs/reference/foundations/duration

Retrieve the total duration expressed in hours as a floating-point number.

```typst
self.hours(
) -> float

```

--------------------------------

### Page Foreground with Overlay Text

Source: https://typst.app/docs/reference/layout/page

Places text content in the foreground of the page, overlaying the main body content. This example uses a large font size for the overlay.

```typst
#set page(foreground: text(24pt)[🤓])

Reviewer 2 has marked our paper
"Weak Reject" because they did
not understand our approach...


```

--------------------------------

### Load and Use a WebAssembly Plugin

Source: https://typst.app/docs/reference/foundations/plugin

Loads a WebAssembly module and defines a Typst function to call a plugin function. Ensure the plugin follows the specified protocol.

```typst
#let myplugin = plugin("hello.wasm")
#let concat(a, b) = str(
  myplugin.concatenate(
    bytes(a),
    bytes(b),
  )
)

#concat("hello", "world")

```

--------------------------------

### Get Duration in Minutes

Source: https://typst.app/docs/reference/foundations/duration

Retrieve the total duration expressed in minutes as a floating-point number.

```typst
self.minutes(
) -> float

```

--------------------------------

### Customizing Footnote Numbering

Source: https://typst.app/docs/reference/model/footnote

Illustrates how to customize the numbering of footnotes using the `numbering` parameter. This example uses asterisks and daggers as markers.

```typst
#set footnote(numbering: "*")

Footnotes:
#footnote[Star],
#footnote[Dagger]
```

--------------------------------

### Typst Regex Constructor with String Input

Source: https://typst.app/docs/reference/foundations/regex

Illustrates the creation of a Typst regex object from a string. It explains the constructor signature and provides guidance on handling backslash escaping for both Typst strings and regular expression syntax, including examples for common escape sequences and raw string usage.

```typst
regex(
str
)

// Example with escaping:
regex("\\b\\d")

// Example using raw string:
regex(`\d+\\.\\d+\\.\\d+`.text)
```

--------------------------------

### Typst Stack Element Example

Source: https://typst.app/docs/reference/layout/stack

Demonstrates the basic usage of the `stack` element in Typst to arrange rectangular elements vertically with a specified direction.

```typst
#stack(
  dir: ttb,
  rect(width: 40pt),
  rect(width: 120pt),
  rect(width: 90pt),
)

```

--------------------------------

### Typst `stretch` Element Examples

Source: https://typst.app/docs/reference/math/stretch

Demonstrates the usage of the `stretch` element in Typst for stretching glyphs and attachments. It shows how to apply different sizes and styles to the stretched content.

```typst
$ H stretch(=)^"define" U + p V $
$ f : X stretch(->>, size: #150%)_"surjective" Y $
$ x stretch(harpoons.ltrb, size: #3em) y
    stretch(\[, size: #150%) z $
```

--------------------------------

### List All Tables

Source: https://typst.app/docs/reference/model/outline

To generate an outline of tables, set the `target` parameter of the `outline` function to `figure.where(kind: table)`. This example demonstrates listing figures that contain tables.

```typst
#outline(
  title: [List of Tables],
  target: figure.where(kind: table),
)

#figure(
  table(
    columns: 4,
    [t], [1], [2], [3],
    [y], [0.3], [0.7], [0.5],
  ),
  caption: [Experiment results],
)
```

--------------------------------

### Load Sublime Syntax Files for Raw Block Highlighting

Source: https://typst.app/docs/changelog/0.7.0

Support for loading `.sublime-syntax` files enables syntax highlighting for raw code blocks. Ensure the file is accessible by Typst.

```typst
#raw(lang: "sublime-syntax")
```

--------------------------------

### Get Duration in Seconds

Source: https://typst.app/docs/reference/foundations/duration

Retrieve the total duration expressed in seconds as a floating-point number.

```typst
self.seconds(
) -> float

```

--------------------------------

### Set Heading Supplement in Typst

Source: https://typst.app/docs/reference/model/heading

Demonstrates how to set a supplement for headings, which is added before the referenced number. This example shows adding 'Chapter' as a supplement.

```typst
#set heading(numbering: "1.", supplement: [Chapter])

= Introduction <intro>
In @intro, we see how to turn
Sections into Chapters. And
in @intro[Part], it is done
manually.
```

--------------------------------

### Define and Use Custom Alert Function

Source: https://typst.app/docs/reference/foundations/function

Shows how to define a custom function `alert` with parameters and default values, and then use it with different arguments. The function creates a styled alert box.

```typst
#let alert(body, fill: red) = {
  set text(white)
  set align(center)
  rect(
    fill: fill,
    inset: 8pt,
    radius: 4pt,
    [*Warning:\ #body*],
  )
}

#alert[
  Danger is imminent!
]

#alert(fill: blue)[
  KEEP OFF TRACKS
]
```

--------------------------------

### Customizing Footnote Entry Appearance

Source: https://typst.app/docs/reference/model/footnote

Demonstrates how to customize the appearance of footnote entries in the listing using `show footnote.entry`. This example sets the text color to red.

```typst
#show footnote.entry: set text(red)

My footnote listing
#footnote[It's down here]
has red text!
```

--------------------------------

### Typst `mid` Function Example

Source: https://typst.app/docs/reference/math/lr

Demonstrates the `mid` function for vertically scaling delimiters within an `lr()` group, particularly useful for complex expressions.

```typst
$ { x mid(|) sum_(i=1)^n w_i|f_i (x)| < 1 } $
```

--------------------------------

### Demonstrate v Element Spacing Options

Source: https://typst.app/docs/reference/layout/v

Illustrates different ways to use the v element for vertical spacing within a grid layout, showcasing absolute, relative, and fractional spacing, as well as weak spacing.

```typst
#grid(
  rows: 3cm,
  columns: 6,
  gutter: 1fr,
  [A #parbreak() B],
  [A #v(0pt) B],
  [A #v(10pt) B],
  [A #v(0pt, weak: true) B],
  [A #v(40%, weak: true) B],
  [A #v(1fr) B],
)

```

--------------------------------

### Typst Dash Pattern Examples

Source: https://typst.app/docs/reference/visualize/stroke

Illustrates the use of different dash patterns for strokes, including predefined patterns and custom arrays with phase. Use this to create varied line appearances.

```typst
#set line(length: 100%, stroke: 2pt)
#stack(
  spacing: 1em,
  line(stroke: (dash: "dashed")),
  line(stroke: (dash: (10pt, 5pt, "dot", 5pt))),
  line(stroke: (dash: (array: (10pt, 5pt, "dot", 5pt), phase: 10pt))),
)

```

--------------------------------

### Typst Regex String Splitting and Show Rule Example

Source: https://typst.app/docs/reference/foundations/regex

Demonstrates how to use Typst's regex function for splitting strings based on a regular expression pattern and for applying conditional styling to text matching a pattern using show rules. It highlights the syntax for both use cases.

```typst
#"a,b;c".split(regex("[,;]"))

// Works with show rules.
#show regex("\\d+"): set text(red)

The numbers 1 to 10.
```

--------------------------------

### Get Today's Date

Source: https://typst.app/docs/reference/foundations/datetime

Retrieves the current date. This can be influenced by CLI arguments or environment variables for testing purposes.

```APIDOC
## Function datetime.today

Returns the current date.

### Parameters
#### Path Parameters
- **offset** (auto or int) - Optional - An offset to apply to the current UTC date. If set to `auto`, the offset will be the local offset. Default: `auto`.

### Request Example
```typ
Today's date is #datetime.today().display().
```

### Response
#### Success Response (200)
- **datetime** (datetime) - The current date.

#### Response Example
```json
{
  "datetime": "2023-10-27"
}
```
```

--------------------------------

### Get Type of Content

Source: https://typst.app/docs/reference/foundations/content

Use the `type` function to determine the type of a given content, such as text.

```typst
Type of *Hello!* is
#type([*Hello!*])
```

--------------------------------

### Create a Basic Rectangle

Source: https://typst.app/docs/reference/visualize/rect

Use the `rect` function to create a rectangle. It can be created without content, or with content that it will automatically size to fit.

```typst
#rect(width: 35%, height: 30pt)

// With content.
#rect[
  Automatically sized \ 
  to fit the content.
]
```

--------------------------------

### Create a pre Element

Source: https://typst.app/docs/reference/html/typed

Use html.pre to create a block of preformatted text. It takes content as its only parameter.

```typst
html.pre(content,)
```

--------------------------------

### Use Weak Spacing in a Theorem

Source: https://typst.app/docs/reference/layout/v

Shows how to use the weak: true parameter of the v element to manage spacing around mathematical content, ensuring it collapses appropriately at the start or end of a flow.

```typst
The following theorem is
foundational to the field:
#v(4pt, weak: true)
$ x^2 + y^2 = r^2 $
#v(4pt, weak: true)
The proof is simple:

```

--------------------------------

### Dictionary Constructor

Source: https://typst.app/docs/reference/foundations/dictionary

Explains how to use the `dictionary()` constructor to convert dictionary-like values into a dictionary.

```APIDOC
## Dictionary Constructor

### Description
Converts a value into a dictionary. Note that this function is only intended for conversion of a dictionary-like value to a dictionary, not for creation of a dictionary from individual pairs. Use the dictionary syntax `(key: value)` instead.

### Method
`dictionary(value)`

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
```ty
#dictionary(sys).at("version")
```

### Response
#### Success Response (200)
N/A

#### Response Example
N/A
```

--------------------------------

### Basic Outline Usage

Source: https://typst.app/docs/reference/model/outline

Demonstrates the basic usage of the `outline` function to generate a table of contents for headings.

```APIDOC
## Basic Outline

### Description
Generates a list of all occurrences of an element in the document, up to a given depth. By default, it creates a table of contents for headings.

### Method
`outline()`

### Parameters
- `depth` (int) - Optional - The maximum depth of elements to include in the outline.

### Request Example
```typ
#set heading(numbering: "1.")
#outline()

= Introduction
#lorem(5)

= Methods
== Setup
#lorem(10)
```

### Response Example
(This function generates content, not a direct response in the typical API sense. The output is the rendered outline within the document.)
```

--------------------------------

### Typst Module Import

Source: https://typst.app/docs/reference/scripting

Explains how to import definitions from other Typst modules. This example shows importing a specific item ('face') from the 'emoji' module.

```typst
#import emoji: face
#face.grin

```

--------------------------------

### Get Fractional Part of a Number

Source: https://typst.app/docs/reference/foundations/calc

Returns the fractional part of a number. Returns 0 if the number is an integer.

```typst
#calc.fract(-3.1)
#assert(calc.fract(3) == 0)
#assert(calc.fract(decimal("234.23949211")) == decimal("0.23949211"))
```

--------------------------------

### Set Theme for Syntax Highlighting

Source: https://typst.app/docs/reference/text/raw

Apply a theme for syntax highlighting in raw code blocks using the `theme` parameter. Supports 'none', 'auto', paths, or raw bytes.

```typ
#set raw(theme: "halcyon.tmTheme")
#show raw: it => block(
  fill: rgb("#1d2433"),
  inset: 8pt,
  radius: 5pt,
  text(fill: rgb("#a2aabc"), it)
)

= Chapter 1
#let hi = "Hello World"

```

--------------------------------

### Get Dictionary Keys

Source: https://typst.app/docs/reference/foundations/dictionary

Returns an array containing all the keys in the dictionary, preserving the insertion order. Useful for iterating over keys or checking their presence.

```typst
self.keys()
```

--------------------------------

### Typst Columns Element Example

Source: https://typst.app/docs/reference/layout/columns

Demonstrates the basic usage of the 'columns' element to divide content into two columns with a specified gutter. It also shows how to use '#colbreak()' to force content into the next column.

```typst
#columns(2, gutter: 8pt)[
  This text is in the
  first column.

  #colbreak()

  This text is in the
  second column.
]
```

--------------------------------

### Curve Move Definition

Source: https://typst.app/docs/reference/visualize/curve

Details the `curve.move` definition, used to start a new curve component without drawing.

```APIDOC
### `move` Element

Element functions can be customized with `set` and `show` rules. 
Starts a new curve component.
If no `curve.move` element is passed, the curve will start at `(0pt, 0pt)`.

curve.move(
array,relative: bool,
) -> content

#### `start`
array
Required Positional
Positional parameters are specified in order, without names. 
The starting point for the new component.

#### `relative`
bool
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
Whether the coordinates are relative to the previous point.
Default: `false`
```

--------------------------------

### Populating Grid with Spread Content

Source: https://typst.app/docs/reference/layout/grid

Shows how to spread an array of strings or content into a grid to populate its cells, using `range` and `map`.

```typst
#grid(
  columns: 5,
  gutter: 5pt,
  ..range(25).map(str)
)


```

--------------------------------

### Controlling Block-Level Behavior of Headings

Source: https://typst.app/docs/reference/layout/block

Shows how to use `show` rules to change an element's behavior. The first example removes block-level properties from headings, while the second forces headings to be block-level elements.

```typst
#show heading: it => it.body
= Blockless
More text.

#show heading: it => block(it.body)
= Blocky
More text.

```

--------------------------------

### Rotate Content by a Specific Angle

Source: https://typst.app/docs/reference/layout/rotate

Shows how to rotate a piece of content by a specified angle using the `rotate` function. This example uses radians for the angle.

```typst
#rotate(-1.571rad)[Space!]

```

--------------------------------

### Create a Video Element with Controls

Source: https://typst.app/docs/reference/html/typed

Use the `html.video` function to create a video element. Set attributes like `controls`, `width`, `height`, and `src`. The content within the brackets will be displayed if the browser does not support the video tag.

```typst
#html.video(
  controls: true,
  width: 1280,
  height: 720,
  src: "sunrise.mp4",
)[
  Your browser does not support the video tag.
]

```

--------------------------------

### Set Text Spacing (Word Spacing)

Source: https://typst.app/docs/reference/text/text

Controls the space between words, specified as a percentage of the space character's width. This example doubles the word spacing.

```typst
#set text(spacing: 200%)
Text with distant words.
```

--------------------------------

### Typst `op` Element Usage Example

Source: https://typst.app/docs/reference/math/op

Demonstrates how to use the `op` element in Typst for creating custom operators in equations, including setting limits. This is a core Typst math syntax.

```typst
$ tan x = (sin x)/(cos x) $
$ op("custom",
     limits: #true)_(n->oo) n $

```

--------------------------------

### Create Bytes from Integers and Strings

Source: https://typst.app/docs/reference/foundations/bytes

Demonstrates creating bytes from an array of integers and from a UTF-8 encoded string. The `bytes` constructor handles these conversions.

```typst
#bytes((123, 160, 22, 0))
#bytes("Hello 😃")
```

--------------------------------

### Typst Show Rules for Styling

Source: https://typst.app/docs/guides/for-latex-users

Demonstrates the use of 'show' rules in Typst to redefine the appearance of elements, analogous to LaTeX's '\renewcommand'. This example uses a 'show' rule to apply small caps to all subsequent text.

```typst
#show: smallcaps

Boisterous Accusations
```

--------------------------------

### Set Matrix Delimiter with `delim`

Source: https://typst.app/docs/reference/math/mat

Customizes the delimiters for the matrix using the `delim` parameter. This example sets the left delimiter to '[' and infers the right delimiter. Requires a `set` rule for global application.

```typst
#set math.mat(delim: "[")
$ mat(1, 2; 3, 4) $
```

--------------------------------

### Quote Element Usage and Customization

Source: https://typst.app/docs/reference/model/quote

Demonstrates how to use the `quote` element for inline and block quotes, with examples of setting attributions and customizing appearance using `set` and `show` rules.

```APIDOC
## `quote` Element

Element functions can be customized with `set` and `show` rules. Displays a quote alongside an optional attribution.

### Example
```typ
Plato is often misquoted as the author of #quote[I know that I know
nothing], however, this is a derivation form his original quote:

#set quote(block: true)

#quote(attribution: [Plato])[
  ... ἔοικα γοῦν τούτου γε σμικρῷ τινι αὐτῷ τούτῳ σοφώτερος εἶναι, ὅτι
  ἃ μὴ οἶδα οὐδὲ οἴομαι εἰδέναι.
]
#quote(attribution: [from the Henry Cary literal translation of 1897])[
  ... I seem, then, in just this little thing to be wiser than this man at
  any rate, that what I do not know I do not think I know either.
]
```

By default block quotes are padded left and right by `1em`, alignment and padding can be controlled with show rules:
```typ
#set quote(block: true)
#show quote: set align(center)
#show quote: set pad(x: 5em)

#quote[
  You cannot pass... I am a servant of the Secret Fire, wielder of the
  flame of Anor. You cannot pass. The dark fire will not avail you,
  flame of Udûn. Go back to the Shadow! You cannot pass.
]
```

### Parameters
quote(
block: bool,quotes: autobool,attribution: nonelabelcontent,content,
) -> content

#### `block`
- **Type**: bool
- **Settable**: Yes
- **Description**: Whether this is a block quote.
- **Default**: `false`

```typ
An inline citation would look like
this: #quote(
  attribution: [René Descartes]
)[
  cogito, ergo sum
], and a block equation like this:
#quote(
  block: true,
  attribution: [JFK]
)[
  Ich bin ein Berliner.
]
```

#### `quotes`
- **Type**: auto or bool
- **Settable**: Yes
- **Description**: Whether double quotes should be added around this quote. The double quotes used are inferred from the `quotes` property on smartquote, which is affected by the `lang` property on text.
  * `true`: Wrap this quote in double quotes.
  * `false`: Do not wrap this quote in double quotes.
  * `auto`: Infer whether to wrap this quote in double quotes based on the `block` property. If `block` is `false`, double quotes are automatically added.
- **Default**: `auto`

```typ
#set text(lang: "de")

Ein deutsch-sprechender Author
zitiert unter umständen JFK:
#quote[Ich bin ein Berliner.]

#set text(lang: "en")

And an english speaking one may
translate the quote:
#quote[I am a Berliner.]
```

#### `attribution`
- **Type**: none or label or content
- **Settable**: Yes
- **Description**: The attribution of this quote, usually the author or source. Can be a label pointing to a bibliography entry or any content. By default only displayed for block quotes, but can be changed using a `show` rule.
- **Default**: `none`

```typ
#quote(attribution: [René Descartes])[
  cogito, ergo sum
]

#show quote.where(block: false): it => {
  [ "] + h(0pt, weak: true) + it.body + h(0pt, weak: true) + [ "]
  if it.attribution != none [ (#it.attribution)]
}

#quote(
  attribution: link("https://typst.app/home") [typst.app]
)[
  Compose papers faster
]

#set quote(block: true)

#quote(attribution: <tolkien54>)[
  You cannot pass... I am a servant
  of the Secret Fire, wielder of the
  flame of Anor. You cannot pass. The
  dark fire will not avail you, flame
  of Udûn. Go back to the Shadow! You
  cannot pass.
]

#bibliography("works.bib", style: "apa")
```

#### `body`
- **Type**: content
- **Required Positional**: Yes
- **Description**: The quote.
```

--------------------------------

### Avoid Recursive State Updates

Source: https://typst.app/docs/reference/introspection/state

This example demonstrates a problematic recursive state update that can lead to non-convergence. Avoid updating state within context expressions if possible.

```typst
#let x = state("key", 1)
#context x.update(x.final() + 1)
#context x.get()
```

--------------------------------

### Checkerboard Fill for Grid Cells

Source: https://typst.app/docs/reference/layout/grid

Use a function with `fill` to create a checkerboard pattern based on cell coordinates. This example uses `calc.even` to determine cell color.

```typst
#grid(
  fill: (x, y) =>
    if calc.even(x + y) { luma(230) }
    else { white },
  align: center + horizon,
  columns: 4,
  inset: 2pt,
  [X], [O], [X], [O],
  [O], [X], [O], [X],
  [X], [O], [X], [O],
  [O], [X], [O], [X],
)
```

--------------------------------

### Display Image with Explicit Format and Dimensions

Source: https://typst.app/docs/reference/visualize/image

Embed an SVG image, specifying its format and desired width. This example demonstrates how to explicitly set the format when reading a file.

```typst
#image(
  read(
    "tetrahedron.svg",
    encoding: none,
  ),
  format: "svg",
  width: 2cm,
)

```

--------------------------------

### Customizing Path Fill Rules in Typst

Source: https://typst.app/docs/reference/visualize/path

Shows how to create a reusable path shape with pre-applied fill and closed properties, then apply different `fill-rule` values. This illustrates how to use `.with` for function customization.

```typst
// We use `.with` to get a new
// function that has the common
// arguments pre-applied.
#let star = path.with(
  fill: red,
  closed: true,
  (25pt, 0pt),
  (10pt, 50pt),
  (50pt, 20pt),
  (0pt, 20pt),
  (40pt, 50pt),
)

#star(fill-rule: "non-zero")
#star(fill-rule: "even-odd")

```

--------------------------------

### WebAssembly Protocol: Importing `wasm_minimal_protocol_write_args_to_buffer`

Source: https://typst.app/docs/reference/foundations/plugin

Imports the `wasm_minimal_protocol_write_args_to_buffer` function from the `typst_env` module. This function is used to write arguments into a plugin-allocated buffer.

```wat
(import "typst_env" "wasm_minimal_protocol_write_args_to_buffer" (func (param i32)))

```

--------------------------------

### Use LibraryExt for Library Replacement

Source: https://typst.app/docs/changelog/0.14.0

The `Default` impl for `Library` was removed. Use `LibraryExt` for a drop-in replacement.

```rust
use typst::LibraryExt;
```

--------------------------------

### Apply Typst Function to Entire Document using Show Rule

Source: https://typst.app/docs/tutorial/making-a-template

Shows how to use an 'everything' show rule (`#show:`) to apply a Typst function ('amazed') to the entire document content. The document's content is passed as the argument to the function.

```typst
#show: amazed
I choose to focus on the good
in my life and let go of any
negative thoughts or beliefs.
In fact, I am amazing!

```

--------------------------------

### Get sign of direction

Source: https://typst.app/docs/reference/layout/direction

Obtains the numerical sign associated with a direction, useful for calculations. This method returns an integer.

```typst
#ltr.sign()
#rtl.sign()
#ttb.sign()
#btt.sign()
```

--------------------------------

### Get axis of direction

Source: https://typst.app/docs/reference/layout/direction

Retrieves the axis ('horizontal' or 'vertical') to which a given direction belongs using the '.axis()' method.

```typst
#ltr.axis()
#ttb.axis()
```

--------------------------------

### Typst: Basic Place Function with Rectangle

Source: https://typst.app/docs/tutorial/advanced-styling

Demonstrates the basic usage of the `place` function in Typst to overlay a filled rectangle on top of text. It shows how `place` takes content out of the document flow by default.

```typst
#place(
  top + center,
  rect(fill: black),
)
#lorem(30)

```

--------------------------------

### Tiling Constructor API

Source: https://typst.app/docs/reference/visualize/tiling

This section details the constructor for creating tiling patterns in Typst, including its parameters and their types.

```APIDOC
## tiling Constructor

Construct a new tiling.

tiling(
  size: autoarray,
  spacing: array,
  relative: autostr,
  content,
) -> tiling

### Parameters

#### `size`
auto or array
The bounding box of each cell of the tiling.
Default: `auto`

#### `spacing`
array
The spacing between cells of the tiling.
Default: `(0pt, 0pt)`

#### `relative`
auto or str
The relative placement of the tiling.
For an element placed at the root/top level of the document, the parent is the page itself. For other elements, the parent is the innermost block, box, column, grid, or stack that contains the element.
Variant| Details  
---|---
`"self"`| Relative to itself (its own bounding box).
`"parent"`| Relative to its parent (the parent's bounding box).
Default: `auto`

#### `body`
content
Required Positional
Positional parameters are specified in order, without names. 
The content of each cell of the tiling.
```

--------------------------------

### Arabic Text Alignment with `end + horizon`

Source: https://typst.app/docs/reference/layout/align

Example of aligning Arabic text using `end + horizon` for right and vertical centering. It also sets the page height and language for the text.

```typst
#set page(height: 6cm)
#set text(lang: "ar")

مثال
#align(
  end + horizon,
  rect(inset: 12pt)[ركن]
)


```

--------------------------------

### Alignment Basics

Source: https://typst.app/docs/reference/layout/alignment

Explains the fundamental alignment values and how to apply them to content.

```APIDOC
## Alignment

Where to align something along an axis.

### Possible Values

* `start`: Aligns at the start of the text direction.
* `end`: Aligns at the end of the text direction.
* `left`: Align at the left.
* `center`: Aligns in the middle, horizontally.
* `right`: Aligns at the right.
* `top`: Aligns at the top.
* `horizon`: Aligns in the middle, vertically.
* `bottom`: Align at the bottom.

### Usage

These values are available globally and also in the alignment type's scope.

### Request Example

```typc
#align(center)[Hi]
#align(alignment.center)[Hi]
```
```

--------------------------------

### Version `at` Method

Source: https://typst.app/docs/reference/foundations/version

Explains how to retrieve a specific component of a version object.

```APIDOC
### `at` Method

Retrieves a component of a version. The returned integer is always non-negative. Returns `0` if the version isn't specified to the necessary length.

#### Usage
`self.at(index: int) -> int`

#### Parameters
##### `index`
- `int` - Required Positional
  The index at which to retrieve the component. If negative, indexes from the back of the explicitly given components.
```

--------------------------------

### Typst Absolute Path for Images

Source: https://typst.app/docs/reference/syntax

Illustrates the use of an absolute path for including an image in Typst. Absolute paths are resolved from the root of the project and start with a leading `/`.

```typst
#image("/assets/logo.png")

```

--------------------------------

### Filter Elements with `where` Selector

Source: https://typst.app/docs/reference/foundations/function

Demonstrates using the `where` definition with a function to create a selector that filters elements based on their fields. This example styles headings of level 2.

```typst
#show heading.where(level: 2): set text(blue)
= Section
== Subsection
=== Sub-subsection
```

--------------------------------

### Round Rectangle Corners and Set Strokes

Source: https://typst.app/docs/reference/visualize/rect

Apply rounded corners using the `radius` parameter and configure individual side strokes using a dictionary with the `stroke` parameter. This example demonstrates setting a default stroke and then overriding specific sides and corners.

```typst
#set rect(stroke: 4pt)
#rect(
  radius: (
    left: 5pt,
    top-right: 20pt,
    bottom-right: 10pt,
  ),
  stroke: (
    left: red,
    top: yellow,
    right: green,
    bottom: blue,
  ),
)
```

--------------------------------

### Create and Update States

Source: https://typst.app/docs/reference/introspection/state

Illustrates creating multiple states with the same key but different initial values, and then updating one of them. Note how updates affect all states sharing the same key.

```typst
#let banana = state("key", "🍌")
#let broccoli = state("key", "🥦")

#banana.update(it => it + "😋")

#context [
  - #state("key", "🍎").get()
  - #banana.get()
  - #broccoli.get()
]
```

--------------------------------

### Get Dictionary Values

Source: https://typst.app/docs/reference/foundations/dictionary

Returns an array containing all the values in the dictionary, preserving the insertion order. Useful for processing all values without needing their keys.

```typst
self.values()
```

--------------------------------

### Position Configuration

Source: https://typst.app/docs/reference/layout/grid

Defines the position of a line relative to its column. Options include 'start' or 'end', with 'left' and 'right' being discouraged for consistency. This is relevant when column gutter is enabled.

```APIDOC
## Position Configuration

### Description
Defines the position of a line relative to its column. Options include 'start' or 'end', with 'left' and 'right' being discouraged for consistency. This is relevant when column gutter is enabled.

### Parameters
#### Settable Parameters
- **position** (alignment) - Settable - The position at which the line is placed, given its column (`x`) - either `start` to draw before it or `end` to draw after it. The values `left` and `right` are also accepted, but discouraged as they cause your grid to be inconsistent between left-to-right and right-to-left documents. This setting is only relevant when column gutter is enabled (and shouldn't be used otherwise - prefer just increasing the `x` field by one instead), since then the position after a column becomes different from the position before the next column due to the spacing between both.

### Default Value
`start`
```

--------------------------------

### Simple Line Segment

Source: https://typst.app/docs/reference/visualize/curve

Draws a single straight line segment from the current point to a specified end point. This example uses `curve.line` to define the path.

```typst
#curve(
  stroke: blue,
  curve.line((50pt, 0pt)),
  curve.line((50pt, 50pt)),
  curve.line((100pt, 50pt)),
  curve.line((100pt, 0pt)),
  curve.line((150pt, 0pt)),
)

```

--------------------------------

### Box with Baseline Adjustment

Source: https://typst.app/docs/reference/layout/box

Shows how to adjust the baseline of a box element, useful for aligning images or other elements within text. The `tiger.jpg` image is used as an example.

```typst
Image: #box(baseline: 40%, image("tiger.jpg", width: 2cm)).

```

--------------------------------

### Clipped Box with Larger Content

Source: https://typst.app/docs/reference/layout/box

Illustrates how to use the `clip` parameter to hide content that exceeds the box's dimensions. This example shows a larger image being clipped within a smaller box.

```typst
#box(
  width: 50pt,
  height: 50pt,
  clip: true,
  image("tiger.jpg", width: 100pt, height: 100pt)
)

```

--------------------------------

### Attaching Annotations with `place` and `box`

Source: https://typst.app/docs/reference/layout/place

Illustrates how to attach an annotation to a word using a custom `annotate` function. This function wraps `place` in a `box` to avoid layout interference and uses a word joiner (`sym.wj`) and zero-width spacing (`h(0pt, weak: true)`) to ensure proper attachment.

```typst
#let annotate(..args) = {
  box(place(..args))
  sym.wj
  h(0pt, weak: true)
}

A placed #annotate(square(), dy: 2pt)
square in my text.


```

--------------------------------

### Get direction to alignment

Source: https://typst.app/docs/reference/layout/direction

Uses the 'direction.to()' function to obtain a direction based on an endpoint alignment. The alignment parameter is required.

```typst
#direction.to(left) \
#direction.to(right) \
#direction.to(top) \
#direction.to(bottom)
```

--------------------------------

### Equivalent Citation Syntaxes

Source: https://typst.app/docs/reference/model/cite

Illustrates that the `@` syntax, explicit `cite(<key>)`, and `cite(label("key"))` are equivalent ways to reference a work from the bibliography.

```typst
// All the same
@netwok 
#cite(<netwok>) 
#cite(label("netwok"))
```

--------------------------------

### Calculate Datetime Difference

Source: https://typst.app/docs/reference/foundations/datetime

Calculate the duration between two datetime objects by subtracting them. The result can be used to get the difference in hours.

```typst
#let first-of-march = datetime(day: 1, month: 3, year: 2024)
#let first-of-jan = datetime(day: 1, month: 1, year: 2024)
#let distance = first-of-march - first-of-jan
#distance.hours()
```

--------------------------------

### Basic Block with Background and Inset

Source: https://typst.app/docs/reference/layout/block

Demonstrates creating a block with a background color and padding. This is useful for visually separating content while allowing it to span multiple pages.

```typst
#set page(height: 100pt)
#block(
  fill: luma(230),
  inset: 8pt,
  radius: 4pt,
  lorem(30),
)

```

--------------------------------

### String Slice

Source: https://typst.app/docs/reference/foundations/str

Extracts a substring based on start and end byte indices, supporting negative indexing and an optional count for length.

```APIDOC
## Method: slice()

Extracts a substring of the string. Fails if indices are out of bounds.

### Parameters
#### `start`
int
Required Positional
The start byte index (inclusive). If negative, indexes from the back.

#### `end`
none or int
Positional
The end byte index (exclusive). If omitted, slices to the end. If negative, indexes from the back. Default: `none`.

#### `count`
int
The number of bytes to extract. Mutually exclusive with `end`.

### Method Signature
`self.slice(start: int, end: none or int, count: int) -> str`

### Example
```
#"hello world".slice(6, 11) // outputs "world"
#"hello world".slice(6)
#"hello world".slice(0, 5, count: 3) // outputs "hel"
#"hello world".slice(-5)
```
```

--------------------------------

### Typst CLI: Update Command Proxy Support

Source: https://typst.app/docs/changelog/0.10.0

Fetching release metadata with `typst update` now correctly respects proxy settings, ensuring smooth updates in environments with network restrictions.

```bash
typst update
```

--------------------------------

### Integer Constructor

Source: https://typst.app/docs/reference/foundations/int

Details how to convert various types to integers using the `int()` constructor.

```APIDOC
## Constructor
If a type has a constructor, you can call it like a function to create a new value of the type. Converts a value to an integer. Raises an error if there is an attempt to produce an integer larger than the maximum 64-bit signed integer or smaller than the minimum 64-bit signed integer.
  * Booleans are converted to `0` or `1`.
  * Floats and decimals are rounded to the next 64-bit integer towards zero.
  * Strings are parsed in base 10.

### Example
```
#int(false) 
#int(true) 
#int(2.7) 
#int(decimal("3.8")) 
#(int("27") + int("4"))
```

### Signature
`int(value: bool | int | float | str | decimal) -> int`

#### `value`
bool or int or float or str or decimal
Required Positional
Positional parameters are specified in order, without names. The value that should be converted to an integer.
```

--------------------------------

### Get direction from alignment

Source: https://typst.app/docs/reference/layout/direction

Uses the 'direction.from()' function to obtain a direction based on an alignment point. Ensure the alignment is correctly specified.

```typst
#direction.from(left) \
#direction.from(right) \
#direction.from(top) \
#direction.from(bottom)
```

--------------------------------

### Customizing Footnote Entry Numbering Format

Source: https://typst.app/docs/reference/model/footnote

Shows how to customize the numbering format of footnote entries by redefining the `footnote.entry` show rule. This example prefixes numbers with '1: '.

```typst
#show footnote.entry: it => {
  let loc = it.note.location()
  numbering(
    "1: ",
    ..counter(footnote).at(loc),
  )
  it.note.body
}

Customized #footnote[Hello]
listing #footnote[World! 🌏]
```

--------------------------------

### Get Today's Date

Source: https://typst.app/docs/reference/foundations/datetime

Retrieves the current date. An optional offset can be applied, defaulting to the local timezone if set to 'auto'.

```typst
Today's date is
#datetime.today().display().
```

--------------------------------

### Typst `repeat` Element Example

Source: https://typst.app/docs/reference/layout/repeat

Demonstrates the usage of the `repeat` element to fill horizontal space with a specified character. It shows how to set text properties and alignment for the repeated content.

```typst
Sign on the dotted line:
#box(width: 1fr, repeat[.])

#set text(10pt)
#v(8pt, weak: true)
#align(right)[
  Berlin, the 22nd of December, 2022
]
```

--------------------------------

### Create a document section

Source: https://typst.app/docs/reference/html/typed

Use `html.section` to create a generic section within a document or application. It takes content as a positional parameter.

```typst
html.section(content)
```

--------------------------------

### curve.cubic

Source: https://typst.app/docs/reference/visualize/curve

Adds a cubic Bézier curve segment using start and end control points. The `relative` parameter influences coordinate calculations.

```APIDOC
## `curve.cubic`

### Description
Adds a cubic Bézier curve segment from the last point to `end`, using `control-start` and `control-end` as the control points.

### Method
Not applicable (function within a library)

### Endpoint
Not applicable

### Parameters
#### Path Parameters
None

#### Query Parameters
None

#### Request Body
None

### Request Example
```typst
#let handle(start, end) = place(
  line(stroke: red, start: start, end: end)
)

#handle((0pt, 80pt), (10pt, 20pt))
#handle((90pt, 60pt), (100pt, 0pt))

#curve(
  stroke: blue,
  curve.move((0pt, 80pt)),
  curve.cubic((10pt, 20pt), (90pt, 60pt), (100pt, 0pt)),
)
```

### Response
#### Success Response (200)
Content object representing the cubic Bézier curve segment.

#### Response Example
None provided
```

--------------------------------

### Typst `move` Element Example

Source: https://typst.app/docs/reference/layout/move

Demonstrates how to use the `move` function in Typst to reposition content visually. This function takes `dx` and `dy` parameters for horizontal and vertical displacement, respectively, and a `body` parameter for the content to be moved. The layout remains unaffected by the visual movement.

```typst
#rect(inset: 0pt, fill: gray, move(
  dx: 4pt, dy: 6pt,
  rect(
    inset: 8pt,
    fill: white,
    stroke: black,
    [Abra cadabra]
  )
))

```

--------------------------------

### Get Dictionary Pairs

Source: https://typst.app/docs/reference/foundations/dictionary

Returns an array where each element is a two-element array representing a key-value pair. This preserves insertion order and is useful for iterating over both keys and values simultaneously.

```typst
self.pairs()
```

--------------------------------

### Typst: Correctly Markup Headings for Semantics

Source: https://typst.app/docs/guides/accessibility

Demonstrates the idiomatic Typst way to create headings, ensuring semantic meaning is preserved for assistive technologies, unlike manually styling text.

```typst
// ❌ Don't do this
#text(
  size: 16pt,
  weight: "bold",
)[Heading]


```

```typst
// ✅ Do this
#show heading: set text(size: 16pt)
= Heading

```

--------------------------------

### Adjust Text Baseline in Typst

Source: https://typst.app/docs/reference/text/text

Use the `baseline` parameter to shift the text baseline. This example shows how to lower the baseline for specific text.

```typst
A #text(baseline: 3pt)[lowered]
word.
```

--------------------------------

### Set Document Title and Render with Explicit Body

Source: https://typst.app/docs/reference/model/title

This example shows how to set the document title using `set document` and then explicitly provide content to the `#title` function. The `title` parameter in `set document` defaults to `auto` if not provided.

```typst
#set document(title: "Course ABC, Homework 1")
#title[Homework 1]

...
```

--------------------------------

### Programmatic Raw Element Creation

Source: https://typst.app/docs/reference/text/raw

Shows how to create a `raw` element programmatically from a string, specifying the language tag.

```typst
#raw("fn " + "main() {}", lang: "rust")

```

--------------------------------

### Demonstrate `curve.close` Modes

Source: https://typst.app/docs/reference/visualize/curve

This example defines a function to display a shape closed with both 'smooth' and 'straight' modes of `curve.close`. The 'smooth' mode creates a smooth segment, while 'straight' uses a direct line.

```typst
#let shape(mode: "smooth") = curve(
  fill: blue.lighten(80%),
  stroke: blue,
  curve.move((0pt, 50pt)),
  curve.line((100pt, 50pt)),
  curve.cubic(auto, (90pt, 0pt), (50pt, 0pt)),
  curve.close(mode: mode),
)

#shape(mode: "smooth")
#shape(mode: "straight")

```

--------------------------------

### Set Heading Hanging Indent in Typst

Source: https://typst.app/docs/reference/model/heading

Demonstrates setting the `hanging-indent` for headings. This example shows a long heading and how alignment affects indentation.

```typst
#set heading(numbering: "1.")
= A very, very, very, very, very, very long heading

#show heading: set align(center)
== A very long heading\with center alignment
```

--------------------------------

### Create a summary element

Source: https://typst.app/docs/reference/html/typed

Use `html.summary` to provide a caption for `details` elements. It accepts content as a positional parameter.

```typst
html.summary(content)
```

--------------------------------

### Custom Page Header Content

Source: https://typst.app/docs/reference/layout/page

Defines custom content for the page header. This example sets a small font size and includes text and a horizontal rule with varying styles.

```typst
#set par(justify: true)
#set page(
  margin: (top: 32pt, bottom: 20pt),
  header: [
    #set text(8pt)
    #smallcaps[Typst Academy]
    #h(1fr) _Exercise Sheet 3_
  ],
)

#lorem(19)

```

--------------------------------

### Collect Enum Children with For Loop

Source: https://typst.app/docs/reference/model/enum

Demonstrates how adjacent items in an enum are automatically collected, even within constructs like for loops. This example iterates through phases and adds them as enum items.

```typst
#for phase in (
   "Launch",
   "Orbit",
   "Descent",
) [+ #phase]


```

--------------------------------

### Create an unordered list

Source: https://typst.app/docs/reference/html/typed

Use the `ul` function to create an unordered list. It takes content as its parameter.

```typst
html.ul(content)
```

--------------------------------

### State Constructor

Source: https://typst.app/docs/reference/introspection/state

Creates a new state identified by a key. Multiple states with the same key share updates.

```APIDOC
## state(key: str, init: any)

### Description
Creates a new state identified by a key. If multiple states share the same key, they will all be updated together.

### Parameters
#### `key` (str) - Required
The unique string identifier for the state.

#### `init` (any) - Optional
The initial value of the state. Defaults to `none`.

### Request Example
```typst
#let my_state = state("counter", 0)
```

### Response Example
(Returns a state object)
```

--------------------------------

### Create a Basic Typst Table

Source: https://typst.app/docs/guides/tables

Demonstrates how to create a basic table in Typst by specifying the number of columns and providing content for each cell. The `table` function takes the column count and cell content as arguments. Each cell's content is a separate block, and Typst automatically arranges them into rows.

```typst
#table(
  columns: 2,
  [*Amount*], [*Ingredient*],
  [360g], [Baking flour],
  [250g], [Butter (room temp.)],
  [150g], [Brown sugar],
  [100g], [Cane sugar],
  [100g], [70% cocoa chocolate],
  [100g], [35-40% cocoa chocolate],
  [2], [Eggs],
  [Pinch], [Salt],
  [Drizzle], [Vanilla extract],
)

```

--------------------------------

### Typst Document Control: Set Rules

Source: https://typst.app/docs/guides/for-latex-users

Explains how to use 'set' rules in Typst to modify the appearance of subsequent content, similar to LaTeX declarations. This example demonstrates changing text size globally and within a specific block.

```typst
I am starting out with small text.

#set text(14pt)

This is a bit #text(18pt)[larger,]
don't you think?
```

--------------------------------

### Load TMTheme Files for Raw Block Highlighting Themes

Source: https://typst.app/docs/changelog/0.7.0

Typst can now load `.tmTheme` files to apply custom highlighting themes to raw code blocks. This enhances the visual presentation of code.

```typst
#raw(theme: "tmTheme")
```

--------------------------------

### Frac Element Syntax and Usage

Source: https://typst.app/docs/reference/math/frac

This section details the syntax for creating fractions using the `frac` element and its dedicated slash syntax. It also explains how to group expressions using parentheses.

```APIDOC
## `frac` Element
A mathematical fraction.

### Syntax
Use a slash `/` to turn neighbouring expressions into a fraction. Multiple atoms can be grouped into a single expression using round grouping parentheses. Such parentheses are removed from the output, but you can nest multiple to force them.

### Parameters
math.frac(
content, content, style: str,
) -> content

#### `num` (content) - Required Positional
The fraction's numerator.

#### `denom` (content) - Required Positional
The fraction's denominator.

#### `style` (str) - Settable
How the fraction should be laid out. Possible values are `"vertical"`, `"skewed"`, and `"horizontal"`.

**Variants:**
- `"vertical"`: Stacked numerator and denominator with a bar.
- `"skewed"`: Numerator and denominator separated by a slash.
- `"horizontal"`: Numerator and denominator placed inline and parentheses are not absorbed.

**Default:** `"vertical"`

### Examples
```typst
$ 1/2 < (x+1)/2 $
$ ((x+1)) / 2 = frac(a, b) $

$ frac(x, y, style: "vertical") $
$ frac(x, y, style: "skewed") $
$ frac(x, y, style: "horizontal") $

#set math.frac(style: "skewed")
$ a / b $

#set math.frac(style: "vertical")
$ (a + b) / b $

#set math.frac(style: "skewed")
$ (a + b) / b $

#set math.frac(style: "horizontal")
$ (a + b) / b $

#show math.equation.where(block: false): set math.frac(style: "horizontal")
This $(x-y)/z = 3$ is inline math, and this is block math:
$ (x-y)/z = 3 $
```
```

--------------------------------

### Get end alignment of direction

Source: https://typst.app/docs/reference/layout/direction

Retrieves the ending alignment point for a given direction via the '.end()' method. This is helpful for layout calculations.

```typst
#ltr.end()
#rtl.end()
#ttb.end()
#btt.end()
```

--------------------------------

### Get Current Page Number

Source: https://typst.app/docs/reference/introspection/location

Retrieves the physical page number of the current context using the `here()` function. Ensure you are within a context where `here()` is valid.

```typst
#context [
  I am located on
  page #here().page()
]

```

--------------------------------

### Contextual Function Evaluation

Source: https://typst.app/docs/reference/context

Demonstrates how functions defined within a `context` block become contextual, similar to built-in functions like `to-absolute`. The example checks if a custom contextual function `foo` behaves as expected.

```typst
#let foo() = 1em.to-absolute()
#context {
  foo() == text.size
}


```

--------------------------------

### Create an HTML table

Source: https://typst.app/docs/reference/html/typed

Use `html.table` to create an HTML table. It accepts content as a positional parameter, typically rows and cells.

```typst
html.table(content)
```

--------------------------------

### Typst Line and Block Comments

Source: https://typst.app/docs/reference/syntax

Demonstrates the usage of single-line comments starting with `//` and multi-line block comments enclosed by `/*` and `*/` in Typst. Comments are ignored by the Typst compiler.

```typst
// our data barely supports
// this claim

We show with $p < 0.05$
that the difference is
significant.

Our study design is as follows:
/* Somebody write this up:
   - 1000 participants.
   - 2x2 data design. */

```

--------------------------------

### Set Page Margins with Dictionary

Source: https://typst.app/docs/reference/layout/page

Configures page margins using a dictionary for precise control over top, bottom, left, and right spacing. This example sets horizontal margins to 8pt and vertical margins to 4pt.

```typst
#set page(
  width: 3cm,
  height: 4cm,
  margin: (x: 8pt, y: 4pt),
)

#rect(
  width: 100%,
  height: 100%,
  fill: aqua,
)

```

--------------------------------

### Alternative Outlines with Target

Source: https://typst.app/docs/reference/model/outline

Shows how to use the `target` parameter to create outlines for elements other than headings, such as figures.

```APIDOC
## Alternative Outlines

### Description
Generates an outline for specific types of elements by using the `target` parameter with a selector.

### Method
`outline(target: <selector>)`

### Parameters
- `title` (content) - Optional - The title for the outline.
- `target` (labelselectorlocationfunction) - Required - Specifies the type of element to outline. Can use selectors like `figure.where(kind: image)`.

### Request Example
```typ
#outline(
  title: [List of Figures],
  target: figure.where(kind: image),
)

#figure(
  image("tiger.jpg"),
  caption: [A nice figure!],
)
```

### Response Example
(The rendered outline listing the specified elements.)
```

--------------------------------

### Set Page Fill Color

Source: https://typst.app/docs/reference/layout/page

Sets the background fill color for the page and the text color. This example enables a dark mode theme by setting the page fill to a dark gray and text to light gray.

```typst
#set page(fill: rgb("444352"))
#set text(fill: rgb("fdfdfd"))
*Dark mode enabled.*

```

--------------------------------

### Overline Element in Typst

Source: https://typst.app/docs/reference/math/underover

Use the `overline` function to draw a horizontal line over content in Typst math mode. Requires no special setup.

```typst
$ overline(1 + 2 + ... + 5) $
```

--------------------------------

### Underline Element in Typst

Source: https://typst.app/docs/reference/math/underover

Use the `underline` function to draw a horizontal line under content in Typst math mode. Requires no special setup.

```typst
$ underline(1 + 2 + ... + 5) $
```

--------------------------------

### Query Elements Before Current Position with `here()` and `selector`

Source: https://typst.app/docs/reference/introspection/here

This example shows how to use `here()` in conjunction with `query` and `selector` to count the number of headings that appear before the current position in the document. It highlights the use of `before(here())` to define the query range.

```typst
= Introduction
= Background

There are
#context query(
  selector(heading).before(here())
).len()
headings before me.

= Conclusion

```

--------------------------------

### Customizing Text with Set Rule and Direct Call

Source: https://typst.app/docs/reference/text/text

Shows two ways to apply text styling: using a `set` rule for global application and directly calling the `text` function for specific instances, such as passing styled text as an argument to another function.

```typst
#set text(18pt)
With a set rule.

#emph(text(blue)[
  With a function call.
])
```

--------------------------------

### Mat Element Usage

Source: https://typst.app/docs/reference/math/mat

Demonstrates the basic usage of the `mat` element for creating matrices, including row and column separation syntax.

```APIDOC
## `mat` Element

A matrix element for Typst.

### Description

Elements of a row are separated by commas, and rows are separated by semicolons. The semicolon syntax merges preceding comma-separated arguments into an array. This function can also be used to define custom 2D data functions.

Content in cells can be aligned using the `align` parameter or the `&` symbol for cells within the same row.

### Example

```typst
$ mat(
  1, 2, ..., 10;
  2, 2, ..., 10;
  dots.v, dots.v, dots.down, dots.v;
  10, 10, ..., 10;
) $
```
```

--------------------------------

### Set Text Size and Relative Sizing

Source: https://typst.app/docs/reference/text/text

Demonstrates setting a base font size and using relative units (em) for subsequent text. `1em` is relative to the current font size.

```typst
#set text(size: 20pt)
very #text(1.5em)[big] text
```

--------------------------------

### Typst Custom Numbering Function Example

Source: https://typst.app/docs/reference/model/numbering

Illustrates how to define and use a custom numbering function with the `heading` element.

```typst
#let unary(.., last) = "|" * last
#set heading(numbering: unary)
= First heading
= Second heading
= Third heading
```

--------------------------------

### Typst: Selector Constructor

Source: https://typst.app/docs/reference/foundations/selector

Illustrates the basic usage of the `selector` constructor in Typst. It shows how to convert various types like strings, regular expressions, labels, locations, or element functions into a selector object.

```typst
selector("some string")
selector(/regex/)
selector(<label>)
selector(location)
selector(heading.where(level: 1))

```

--------------------------------

### Create a ruby Element

Source: https://typst.app/docs/reference/html/typed

Use html.ruby to create ruby annotations. It takes content as its only parameter.

```typst
html.ruby(content,)
```

--------------------------------

### Typst: Horizontal Layout with Fractions

Source: https://typst.app/docs/reference/layout/fraction

Demonstrates how to use horizontal spacing with fractional units in Typst. Elements are allocated space proportionally to their 'fr' value relative to the total 'fr' sum. This example shows three text elements with different horizontal fraction allocations.

```typst
Left #h(1fr) Left-ish #h(2fr) Right
```

--------------------------------

### Deferred Evaluation in Nested Contexts with Templates

Source: https://typst.app/docs/reference/context

Illustrates deferred evaluation in nested Typst contexts using a template function. The example shows how `text.lang` reacts to a style change within a template only when the surrounding context block's evaluation is deferred.

```typst
#let template(body) = {
  set text(lang: "fr")
  upper(body)
}

#set text(lang: "de")
#context [
  #show: template
  #text.lang \ 
  #context text.lang
]


```

--------------------------------

### Typst Loop Control with Break

Source: https://typst.app/docs/reference/scripting

Illustrates how to control loop execution in Typst using 'break' to exit a loop early. This example stops iterating through a string when a space is encountered.

```typst
#for letter in "abc nope" {
  if letter == " " {
    break
  }

  letter
}

```

--------------------------------

### Page Reference with Custom Supplement

Source: https://typst.app/docs/reference/model/ref

Configures Typst to use 'page' form for all `ref` elements by default and sets a custom supplement 'p.' for page references. This example shows referencing a figure.

```typst
#set page(
  numbering: "1",
  supplement: "p.",
)
#set ref(form: "page")

#figure(
  stack(
    dir: ltr,
    spacing: 1em,
    circle(),
    square(),
  ),
  caption: [Shapes],
) <shapes>

#pagebreak()

See @shapes for examples
of different shapes.

```

--------------------------------

### Basic Enum Usage

Source: https://typst.app/docs/reference/model/enum

Demonstrates automatically and manually numbered list items, including multi-line items and function calls.

```typst
+ Preparations
+ Analysis
+ Conclusions

2. What is the first step?
5. I am confused.
+  Moving on ...

+ This enum item has multiple
  lines because the next line
  is indented.

#enum[First][Second]
```

--------------------------------

### Typst Identifier Syntax and Conventions

Source: https://typst.app/docs/reference/syntax

Shows valid Typst identifiers, which can include letters, numbers, hyphens, and underscores, and must start with a letter or underscore. It also demonstrates the recommended Kebab case convention for multi-word identifiers.

```typst
#let kebab-case = [Using hyphen]
#let _schön = "😊"
#let 始料不及 = "😱"
#let π = calc.pi

#kebab-case
#if -π < 0 { _schön } else { 始料不及 }
// -π means -1 * π,
// so it's not a valid identifier

```

--------------------------------

### Basic Footnote Usage

Source: https://typst.app/docs/reference/model/footnote

Demonstrates the basic usage of the `footnote` element to insert a reference and its corresponding note at the bottom of the page.

```typst
Check the docs for more details.
#footnote[https://typst.app/docs]
```

--------------------------------

### Typst CLI: Query Metadata Field

Source: https://typst.app/docs/reference/introspection/query

Shows how to extract a specific field (`value`) from query results using the `--field` argument with the Typst CLI. This example retrieves only the value of the metadata.

```bash
typst query example.typ "<note>" --field value

```

--------------------------------

### Basic `overline` Element Usage

Source: https://typst.app/docs/reference/text/overline

Demonstrates the fundamental usage of the `overline` element to add a line over text.

```typst
#overline[A line over text.]

```

--------------------------------

### Enum Element Usage

Source: https://typst.app/docs/reference/model/enum

Demonstrates various ways to use the `enum` element, including automatic and manual numbering, multi-line items, and function calls.

```APIDOC
## Enum Element Usage

### Description
Functions can be customized with `set` and `show` rules. A numbered list displays a sequence of items vertically and numbers them consecutively.

### Examples

Automatically numbered:
```typ
+ Preparations
+ Analysis
+ Conclusions
```

Manually numbered:
```typ
2. What is the first step?
5. I am confused.
+ Moving on ...
```

Multiple lines:
```typ
+ This enum item has multiple
  lines because the next line
  is indented.
```

Function call:
```typ
#enum[First][Second]
```

Switching numbering style with a set rule:
```typ
#set enum(numbering: "a)")

+ Starting off ...
+ Don't forget step two
```

Programmatically customizing item numbers with `enum.item`:
```typ
#enum(
  enum.item(1)[First step],
  enum.item(5)[Fifth step],
  enum.item(10)[Tenth step]
)
```
```

--------------------------------

### Constructing a Decimal from a String

Source: https://typst.app/docs/reference/foundations/decimal

Demonstrates the recommended way to create a decimal number using a string literal to preserve precision. Ensure the string is enclosed in double quotes.

```typst
#decimal("1.222222222222222")

```

--------------------------------

### Typst CLI Project Root Configuration

Source: https://typst.app/docs/reference/syntax

Shows how to set a specific directory as the project root using the Typst CLI's `--root` flag. This affects how absolute paths are resolved.

```bash
typst compile --root .. file.typ

```

--------------------------------

### Disable OpenType Features with `features`

Source: https://typst.app/docs/reference/text/text

Disable specific OpenType features by mapping their names to `0` in the `features` dictionary. This example disables contextual alternates (`calt`).

```typst
#set text(font: "Cascadia Code")
=>
// Disable the contextual alternates (`calt`) feature.
#set text(features: (calt: 0))
=>
```

--------------------------------

### Basic Alignment Usage

Source: https://typst.app/docs/reference/layout/alignment

Demonstrates the basic usage of alignment values within the align function. Both global and scoped access are shown.

```typst
#align(center)[Hi]
#align(alignment.center)[Hi]
```

--------------------------------

### Command Line Query

Source: https://typst.app/docs/reference/introspection/query

Shows how to use the `typst query` command to execute queries directly from the command line and retrieve serialized element data.

```APIDOC
## GET /query (CLI)

### Description
Executes an arbitrary query on a Typst document from the command line and returns the resulting elements in a serialized format.

### Method
`typst query` (CLI command)

### Endpoint
`typst query <file> [query] [--field <field>] [--one]`

### Parameters
#### Path Parameters
- **file** (string) - Required - The path to the Typst document.
- **query** (string) - Optional - The query string to execute (e.g., a label selector like `"<note>"`).

#### Query Parameters
- **--field** (string) - Optional - Specifies a particular field to extract from the resulting elements.
- **--one** (boolean) - Optional - Extracts only a single element if multiple are found.
- **--target** (string) - Optional - Specifies the export target for the query (`paged` or `html`).

### Request Example
```bash
$ typst query example.typ "<note>"
```

### Response
#### Success Response (200)
- **array** - A JSON array of serialized elements matching the query.

#### Response Example
```json
[
  {
    "func": "metadata",
    "value": "This is a note",
    "label": "<note>"
  }
]
```
```

--------------------------------

### Customizing Footnote Clearance

Source: https://typst.app/docs/reference/model/footnote

Demonstrates adjusting the vertical space between the separator and the footnote entries using the `clearance` parameter of `footnote.entry`. This example sets the clearance to `3em`.

```typst
#set footnote.entry(clearance: 3em)

Footnotes also need ...
#footnote[
  ... some space to breathe.
]
```

--------------------------------

### Customizing Footnote Separator

Source: https://typst.app/docs/reference/model/footnote

Illustrates how to change the separator between the document body and the footnote listing using the `separator` parameter of `footnote.entry`. This example uses repeated dots.

```typst
#set footnote.entry(
  separator: repeat[.]
)

Testing a different separator.
#footnote[
  Unconventional, but maybe
  not that bad?
]
```

--------------------------------

### Align Table Columns with an Array in Typst

Source: https://typst.app/docs/guides/tables

Demonstrates how to set column-specific alignment for a Typst table by providing an array to the `align` argument. Typst cycles through the array for each column, allowing for different alignments per column. This example right-aligns the first column and left-aligns the rest.

```typst
#set text(font: "IBM Plex Sans")
#show table.cell.where(y: 0): set text(weight: "bold")

#table(
  columns: 4,
  align: (right, left, left, left),
  fill: (_, y) => if calc.odd(y) { green.lighten(90%) },
  stroke: none,

  table.header[Day][Location][Hotel or Apartment][Activities],
  [1], [Paris, France], [Hôtel de l'Europe], [Arrival, Evening River Cruise],
  [2], [Paris, France], [Hôtel de l'Europe], [Louvre Museum, Eiffel Tower],
  [3], [Lyon, France], [Lyon City Hotel], [City Tour, Local Cuisine Tasting],
  [4], [Geneva, Switzerland], [Lakeview Inn], [Lake Geneva, Red Cross Museum],
  [5], [Zermatt, Switzerland], [Alpine Lodge], [Visit Matterhorn, Skiing],
)

```

--------------------------------

### Use a Color Map for Gradient

Source: https://typst.app/docs/reference/visualize/color

Shows how to use a predefined color map, 'crest', to create a linear gradient fill for a circle.

```typst
#circle(fill: gradient.linear(..color.map.crest))


```

--------------------------------

### Create a Rectangle with Aqua Fill

Source: https://typst.app/docs/reference/visualize/color

Demonstrates how to create a rectangle and fill it with the predefined 'aqua' color.

```typst
#rect(fill: aqua)


```

--------------------------------

### Simple Strokes

Source: https://typst.app/docs/reference/visualize/stroke

Explains how to create simple strokes using colors and thicknesses.

```APIDOC
## Simple Strokes

You can create a simple solid stroke from a color, a thickness, or a combination of the two.

- **Thickness only**: A length specifying the stroke's thickness. The color is inherited, defaulting to black.
- **Color only**: A color to use for the stroke. The thickness is inherited, defaulting to `1pt`.
- **Combination**: Use the `+` operator as in `2pt + red`.
```

--------------------------------

### Basic Show-Set Rule for Headings

Source: https://typst.app/docs/reference/styling

Applies a style to a specific element type. This example changes the color of all 'heading' elements to navy, while other text remains black. It uses a simple selector (the element function 'heading') and a set rule.

```typst
#show heading: set text(navy)

= This is navy-blue
But this stays black.

```

--------------------------------

### Create a Template Element in Typst

Source: https://typst.app/docs/reference/html/typed

Use `html.template` to create a template element, supporting various shadow root configurations and content.

```typst
html.template(
shadowrootclonable: bool,
shadowrootcustomelementregistry: bool,
shadowrootdelegatesfocus: bool,
shadowrootmode: str,
shadowrootserializable: bool,
content,
)
```

--------------------------------

### Define and Use a Simple Typst Function

Source: https://typst.app/docs/tutorial/making-a-template

Demonstrates defining a Typst function 'amazed' that takes a term and wraps it in sparkles. It shows how to call this function with a required argument.

```typst
#let amazed(term) = box[✨ #term ✨]

You are #amazed[beautiful]!

```

--------------------------------

### Create a script Element

Source: https://typst.app/docs/reference/html/typed

Use html.script to embed executable code. Various attributes like 'src', 'async', 'defer', and 'type' are supported.

```typst
html.script(async: bool,blocking: strarray,crossorigin: str,defer: bool,fetchpriority: autostr,integrity: str,nomodule: bool,referrerpolicy: nonestr,src: str,type: str,content,)
```

--------------------------------

### WebAssembly Protocol: Importing `wasm_minimal_protocol_send_result_to_host`

Source: https://typst.app/docs/reference/foundations/plugin

Imports the `wasm_minimal_protocol_send_result_to_host` function from the `typst_env` module. This function is used to send the output or an error message back to the Typst host.

```wat
(import "typst_env" "wasm_minimal_protocol_send_result_to_host" (func (param i32 i32)))

```

--------------------------------

### Typst: Styling Elements with Show Rules

Source: https://typst.app/docs/guides/accessibility

Illustrates how to customize the appearance of Typst elements, such as 'strong' emphasis, using show rules while preserving their semantic meaning.

```typst
// Change how text inside of strong emphasis looks
#show strong: set text(tracking: 0.2em, fill: blue, weight: "black")

When setting up your tents, *never forget* to secure the pegs.

```

--------------------------------

### Customizing Footnote Entry Gap

Source: https://typst.app/docs/reference/model/footnote

Shows how to adjust the vertical space between individual footnote entries using the `gap` parameter of `footnote.entry`. This example sets the gap to `0.8em`.

```typst
#set footnote.entry(gap: 0.8em)

Footnotes:
#footnote[Spaced],
#footnote[Apart]
```

--------------------------------

### Applying Gradients to Text

Source: https://typst.app/docs/reference/visualize/gradient

Shows how to apply a linear gradient as a fill for text. For more complex text gradients (word-by-word or glyph-by-glyph), content needs to be wrapped in boxes.

```typst
#set text(fill: gradient.linear(red, blue))
#let rainbow(content) = {
  set text(fill: gradient.linear(..color.map.rainbow))
  box(content)
}

This is a gradient on text, but with a #rainbow[twist]!

```

--------------------------------

### Importing Typst Packages

Source: https://typst.app/docs/guides/for-latex-users

Demonstrates how to import external packages from the Typst ecosystem. This is useful for extending Typst's functionality with community-created tools and libraries. The `@preview` namespace is used for experimental packages.

```typst
#import "@preview/cetz:0.4.1"

```

--------------------------------

### Bytes Constructor Usage

Source: https://typst.app/docs/reference/foundations/bytes

Illustrates the `bytes` constructor for converting strings (UTF-8 encoded) and arrays of integers (0-255) into the efficient byte representation.

```typst
#bytes("Hello 😃") \
#bytes((123, 160, 22, 0))
```

--------------------------------

### Typst Fraction Styling with Grouping Parentheses

Source: https://typst.app/docs/reference/math/frac

Demonstrates how grouping parentheses affect fraction rendering based on the `style` parameter. Parentheses are typically removed but can be retained with specific styles.

```typst
// Grouping parentheses are removed.
#set math.frac(style: "vertical")
$ (a + b) / b $

// Grouping parentheses are removed.
#set math.frac(style: "skewed")
$ (a + b) / b $

// Grouping parentheses are retained.
#set math.frac(style: "horizontal")
$ (a + b) / b $

```

--------------------------------

### Smallcaps Headings

Source: https://typst.app/docs/reference/text/smallcaps

Shows how to apply smallcaps formatting to all headings using a show rule, along with centering and disabling bold font.

```APIDOC
## Smallcaps headings

You can use a show rule to apply smallcaps formatting to all your headings. In the example below, we also center-align our headings and disable the standard bold font.

```ty
#set par(justify: true)
#set heading(numbering: "I.")

#show heading: smallcaps
#show heading: set align(center)
#show heading: set text(
  weight: "regular"
)

= Introduction
#lorem(40)
```
```

--------------------------------

### Conditional Stroke for Grid Cells

Source: https://typst.app/docs/reference/layout/grid

Applies a conditional stroke to grid cells based on their column index. This example sets a right-side stroke for cells not in the first column.

```typst
#set page(width: 420pt)
#set text(number-type: "old-style")
#show grid.cell.where(y: 0): set text(size: 1.3em)

#grid(
  columns: (1fr, 2fr, 2fr),
  row-gutter: 1.5em,
  inset: (left: 0.5em),
  stroke: (x, y) => if x > 0 { (left: 0.5pt + gray) },
  align: horizon,

  [Winter \ 2007 \ Season],
  [Aaron Copland \ *The Tender Land* \ January 2007],
  [Eric Satie \ *Gymnopedie 1, 2* \ February 2007],

  [],
  [Jan 12 \ *Middlebury College \ Center for the Arts* \ 20:00],
  [Feb 2 \ *Johnson State College Dibden Center for the Arts* \ 19:30],

  [],
  [Skip a week \ #text(0.8em)[_Prepare your exams!_]],
  [Feb 9 \ *Castleton State College \ Fine Arts Center* \ 19:30],

  [],
  [Jan 26, 27 \ *Lyndon State College Alexander Twilight Theater* \ 20:00],
  [
    Feb 17 --- #smallcaps[Anniversary] \
    *Middlebury College \ Center for the Arts* \
    19:00 #text(0.7em)[(for a special guest)]
  ],
)
```

--------------------------------

### Grid Element with Track Sizing and Cell Spanning

Source: https://typst.app/docs/reference/layout/grid

Demonstrates different track sizing options (fixed, fractional, auto) and how to make a cell span multiple tracks using `grid.cell`.

```typst
#set rect(
  inset: 8pt,
  fill: rgb("e4e5ea"),
  width: 100%,
)

#grid(
  columns: (60pt, 1fr, 2fr),
  rows: (auto, 60pt),
  gutter: 3pt,
  rect[Fixed width, auto height],
  rect[1/3 of the remains],
  rect[2/3 of the remains],
  rect(height: 100%)[Fixed height],
  grid.cell(
    colspan: 2,
    image("tiger.jpg", width: 100%),
  ),
)


```

--------------------------------

### Typst: Set Title and Basic Formatting

Source: https://typst.app/docs/tutorial/advanced-styling

Demonstrates how to set a document title using the `#title` function and apply basic formatting. The title is automatically bolded and spaced.

```typst
#title[
  A Fluid Dynamic Model
  for Glacier Flow
]

```

--------------------------------

### Check Type of a Variable

Source: https://typst.app/docs/reference/foundations/type

Use the `type()` function to check the type of a variable and conditionally execute code based on its type. This example demonstrates checking if `x` is an integer.

```typst
#let x = 10
#if type(x) == int [
  #x is an integer!
] else [
  #x is another value...
]

An image is of type
#type(image("glacier.jpg")).

```

--------------------------------

### Get Type of Various Values

Source: https://typst.app/docs/reference/foundations/type

The `type()` constructor can be called to determine the type of various Typst values, including numbers, strings, content blocks, functions, and types themselves.

```typst
#type(12) \
#type(14.7) \
#type("hello") \
#type(<glacier>) \
#type([Hi]) \
#type(x => x + 1) \
#type(type)

```

--------------------------------

### Create Basic Tiling Pattern

Source: https://typst.app/docs/reference/visualize/tiling

Defines a basic tiling pattern with diagonal lines and applies it to a rectangle. This is useful for creating simple repeating backgrounds or fills.

```typst
#let pat = tiling(size: (30pt, 30pt))[
  #place(line(start: (0%, 0%), end: (100%, 100%)))
  #place(line(start: (0%, 100%), end: (100%, 0%)))
]

#rect(fill: pat, width: 100%, height: 60pt, stroke: 1pt)

```

--------------------------------

### Converting Values to Floats in Typst

Source: https://typst.app/docs/reference/foundations/float

Shows how to use the `float()` constructor to convert booleans, integers, ratios, strings, and decimals to floating-point numbers. Note that string parsing supports exponential notation.

```typst
#float(false) \
#float(true) \
#float(4) \
#float(40%) \
#float("2.7") \
#float("1e5")
```

--------------------------------

### Set Document Language in Typst

Source: https://typst.app/docs/reference/text/text

Use the `lang` parameter to specify the document's language, which affects hyphenation, smart quotes, and accessibility. This example sets the language to German.

```typst
#set text(lang: "de")
#outline()

= Einleitung
In diesem Dokument, ...
```

--------------------------------

### Draw a closed shape with relative lines

Source: https://typst.app/docs/reference/visualize/curve

Use `relative: true` to draw segments relative to the previous point. This example draws a square using four line segments.

```typst
#curve(
  stroke: blue,
  curve.line((50pt, 0pt), relative: true),
  curve.line((0pt, 50pt), relative: true),
  curve.line((50pt, 0pt), relative: true),
  curve.line((0pt, -50pt), relative: true),
  curve.line((50pt, 0pt), relative: true),
)

```

--------------------------------

### Configure Page, Paragraph, and Text Styling in Typst

Source: https://typst.app/docs/tutorial/advanced-styling

This snippet demonstrates how to configure page dimensions, header content, page numbering, paragraph justification, and text font and size using Typst's set rules. It's useful for establishing a document's overall look and feel according to specific guidelines.

```typst
#set page(
  paper: "us-letter",
  header: align(right)[
    A Fluid Dynamic Model for
    Glacier Flow
  ],
  numbering: "1",
)
#set par(justify: true)
#set text(
  font: "Libertinus Serif",
  size: 11pt,
)

#lorem(600)

```

--------------------------------

### Typst: Customize Title Appearance with Show-Set Rules

Source: https://typst.app/docs/tutorial/advanced-styling

Shows how to customize the appearance of the title using `show-set` rules to change text size and alignment. This allows for fine-grained control over element styling.

```typst
#show title: set text(size: 17pt)
#show title: set align(center)

#title[
  A Fluid Dynamic Model
  for Glacier Flow
]

```

--------------------------------

### Block with Percentage Width and Inset

Source: https://typst.app/docs/reference/layout/block

Illustrates setting a specific width for a block using a percentage and applying inset. This is useful for controlling the horizontal space occupied by content.

```typst
#set align(center)
#block(
  width: 60%,
  inset: 8pt,
  fill: silver,
  lorem(10),
)

```

--------------------------------

### Create a q Element

Source: https://typst.app/docs/reference/html/typed

Use html.q to create a quotation. The 'cite' attribute can be used to provide a link to the source of the quotation.

```typst
html.q(cite: str,content,)
```

--------------------------------

### Typst Text and Layout: Highlighting and Polygons

Source: https://typst.app/docs/changelog/0.8.0

Shows how to use the `highlight` function to add a background color to text and how to draw regular polygons using `polygon.regular` in Typst.

```typst
// Highlight text
#highlight(fill: red, "This text is highlighted")

// Draw a regular polygon
#polygon.regular(sides: 5, radius: 1cm, fill: blue)
```

--------------------------------

### Using State Variables with Subsequent Computations

Source: https://typst.app/docs/reference/introspection/state

Demonstrates how state managed by Typst can be used in subsequent computations, even when stored in intermediate variables, ensuring correct results due to layout order updates.

```typst
#let more = [
  #compute("⭐ * 2") \
  #compute("⭐ - 5")
]

#compute("10") \
#compute("⭐ + 3") \
#more
```

--------------------------------

### Access Byte at Index with Default

Source: https://typst.app/docs/reference/foundations/bytes

Shows how to retrieve a specific byte at a given index using the `at()` method. A default value can be provided to handle out-of-bounds indices gracefully.

```typst
self.at(int,default: any)
```

--------------------------------

### Get Location Position

Source: https://typst.app/docs/reference/introspection/location

Retrieves a dictionary containing the page number and x, y coordinates of a location. Coordinates are measured from the top-left of the page. Use `page()` if only the page number is needed.

```typst
self.position()

```

--------------------------------

### Set Document Title and Render

Source: https://typst.app/docs/reference/model/title

Use `set document(title: ...)` to define the document's title, then render it with `#title()`. This is for the main document title, not section headings.

```typst
#set document(
  title: [Interstellar Mail Delivery]
)

#title()

= Introduction
In recent years, ...
```

--------------------------------

### HTML Elements Documentation

Source: https://typst.app/docs/reference/html/typed

Documentation for various HTML elements including optgroup, option, output, p, picture, pre, progress, q, rp, rt, ruby, s, samp, script, and search.

```APIDOC
## optgroup

### Description
Group of options in a list box.

### Method
N/A (Component definition)

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
N/A

### Response
#### Success Response (200)
N/A

#### Response Example
N/A

## option

### Description
Option in a list box or combo box control.

### Method
N/A (Component definition)

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
N/A

### Response
#### Success Response (200)
N/A

#### Response Example
N/A

## output

### Description
Calculated output value.

### Method
N/A (Component definition)

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
N/A

### Response
#### Success Response (200)
N/A

#### Response Example
N/A

## p

### Description
Paragraph.

### Method
N/A (Component definition)

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
N/A

### Response
#### Success Response (200)
N/A

#### Response Example
N/A

## picture

### Description
Image.

### Method
N/A (Component definition)

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
N/A

### Response
#### Success Response (200)
N/A

#### Response Example
N/A

## pre

### Description
Block of preformatted text.

### Method
N/A (Component definition)

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
N/A

### Response
#### Success Response (200)
N/A

#### Response Example
N/A

## progress

### Description
Progress bar.

### Method
N/A (Component definition)

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
N/A

### Response
#### Success Response (200)
N/A

#### Response Example
N/A

## q

### Description
Quotation.

### Method
N/A (Component definition)

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
N/A

### Response
#### Success Response (200)
N/A

#### Response Example
N/A

## rp

### Description
Parenthesis for ruby annotation text.

### Method
N/A (Component definition)

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
N/A

### Response
#### Success Response (200)
N/A

#### Response Example
N/A

## rt

### Description
Ruby annotation text.

### Method
N/A (Component definition)

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
N/A

### Response
#### Success Response (200)
N/A

#### Response Example
N/A

## ruby

### Description
Ruby annotation(s).

### Method
N/A (Component definition)

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
N/A

### Response
#### Success Response (200)
N/A

#### Response Example
N/A

## s

### Description
Inaccurate text.

### Method
N/A (Component definition)

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
N/A

### Response
#### Success Response (200)
N/A

#### Response Example
N/A

## samp

### Description
Computer output.

### Method
N/A (Component definition)

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
N/A

### Response
#### Success Response (200)
N/A

#### Response Example
N/A

## script

### Description
Embedded script.

### Method
N/A (Component definition)

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
N/A

### Response
#### Success Response (200)
N/A

#### Response Example
N/A

## search

### Description
Container for search controls.

### Method
N/A (Component definition)

### Endpoint
N/A

### Parameters
#### Path Parameters
N/A

#### Query Parameters
N/A

#### Request Body
N/A

### Request Example
N/A

### Response
#### Success Response (200)
N/A

#### Response Example
N/A
```

--------------------------------

### Create an HTML definition list element

Source: https://typst.app/docs/reference/html/typed

Use this function to create an HTML `<dl>` element, representing an association list. It takes content as a positional parameter.

```typst
html.dl(
content
)
```

--------------------------------

### Set Matrix Alignment with `align`

Source: https://typst.app/docs/reference/math/mat

Sets the horizontal alignment for all cells in the matrix using the `align` parameter. This example aligns cells to the right. Requires a `set` rule for global application.

```typst
#set math.mat(align: right)
$ mat(-1, 1, 1; 1, -1, 1; 1, 1, -1) $
```

--------------------------------

### Configure Table Appearance and Headers

Source: https://typst.app/docs/reference/model/table

Applies global table styling and custom header/cell display rules. Use this to set default fills, alignment, and to emphasize specific header or first column cells.

```typst
#set page(height: 11.5em)
#set table(
  fill: (x, y) =>
    if x == 0 or y == 0 {
      gray.lighten(40%)
    },
  align: right,
)

#show table.cell.where(x: 0): strong
#show table.cell.where(y: 0): strong

#table(
  columns: 4,
  table.header(
    [], [Blue chip],
    [Fresh IPO], [Penny st'k],
  ),
  table.cell(
    rowspan: 6,
    align: horizon,
    rotate(-90deg, reflow: true)[
      *USD / day*
    ],
  ),
  [0.20], [104], [5],
  [3.17], [108], [4],
  [1.59], [84],  [1],
  [0.26], [98],  [15],
  [0.01], [195], [4],
  [7.34], [57],  [2],
)


```

--------------------------------

### Create a document body

Source: https://typst.app/docs/reference/html/typed

Use `html.body` to define the main content of an HTML document. It accepts content as a positional parameter.

```typst
html.body(content)
```

--------------------------------

### Typst Rotate with Angle

Source: https://typst.app/docs/reference/layout/angle

Demonstrates how to use the 'angle' type with the 'rotate' function in Typst. It shows a basic example of rotating text by a specified degree value.

```typst
#rotate(10deg)[Hello there!]

```

--------------------------------

### Set Text Script and Language

Source: https://typst.app/docs/reference/text/text

Demonstrates how to set the writing script and language for text. The `script` parameter influences glyph substitution and feature implementation based on Unicode script properties. When set to `auto`, Typst selects an appropriate script.

```typst
#set text(
  font: "Libertinus Serif",
  size: 20pt,
)

#let scedilla = [Ş]
#scedilla // S with a cedilla

#set text(lang: "ro", script: "latn")
#scedilla // S with a subscript comma

#set text(lang: "ro", script: "grek")
#scedilla // S with a cedilla

```

--------------------------------

### Styling Outline Entries

Source: https://typst.app/docs/reference/model/outline

Explains how to style the appearance of outline entries, including spacing and content.

```APIDOC
## Styling the Outline

### Description
Customizes the appearance of the outline and its entries using show rules and helper functions.

### Method
`show outline.entry` or `show outline`

### Parameters
(Styling is achieved through show rules targeting `outline` or `outline.entry`.)

### Request Example (Spacing)
```typ
#show outline.entry.where(
  level: 1
): set block(above: 1.2em)

#outline()

= About ACME Corp.
== History
=== Origins
= Products
== ACME Tools
```

### Request Example (Custom Entry Content)
```typ
#show outline.entry: it => link(
  it.element.location(),
  // Keep just the body, dropping
  // the fill and the page.
  it.indented(it.prefix(), it.body()),
)

#outline()

= About ACME Corp.
== History
```

### Response Example
(The rendered outline with customized styling.)
```

--------------------------------

### Get Page Numbering Pattern

Source: https://typst.app/docs/reference/introspection/location

Retrieves the page numbering pattern of the page at a given location. Returns `none` if the page numbering is set to `none`. Useful for custom indices or outlines.

```typst
self.page-numbering()

```

--------------------------------

### Create a p Element

Source: https://typst.app/docs/reference/html/typed

Use html.p to create a paragraph element. It takes content as its only parameter.

```typst
html.p(content,)
```

--------------------------------

### Basic Path Drawing with Typst

Source: https://typst.app/docs/reference/visualize/path

Demonstrates how to draw a closed path with fill and stroke using the `path` function. Note that `path` is deprecated and `curve` should be used instead.

```typst
#path(
  fill: blue.lighten(80%),
  stroke: blue,
  closed: true,
  (0pt, 50pt),
  (100%, 50pt),
  ((50%, 0pt), (40pt, 0pt)),
)

```

--------------------------------

### Typst Headings and Emphasis

Source: https://typst.app/docs/guides/for-latex-users

Demonstrates how to create section headings and apply emphasis (italic and bold) in Typst using simple prefix characters and punctuation.

```typst
= Introduction
== In this paper
_emphasis_
*strong emphasis*
```

--------------------------------

### Smallcaps Element Usage

Source: https://typst.app/docs/reference/text/smallcaps

Demonstrates the basic usage of the `smallcaps` element to render text in small capitals.

```APIDOC
## `smallcaps` Element

Displays text in small capitals.

### Example
```ty
Hello \
#smallcaps[Hello]
```
```

--------------------------------

### Customizing Heading Appearance

Source: https://typst.app/docs/reference/model/heading

Explains how to customize the appearance of headings using `set` and `show` rules, including handling custom looks and preventing orphans.

```APIDOC
## Customization

Element functions can be customized with `set` and `show` rules.

When writing a show rule that accesses the `body` field to create a completely custom look for headings, make sure to wrap the content in a `block`. This prevents headings from becoming "orphans" (remaining at the end of the page with the following content on the next page).

### Example with `show` rule
```typ
#show heading.where(level: 2): set text(red)

= Level 1
== Level 2
```
```

--------------------------------

### Image Element Usage

Source: https://typst.app/docs/reference/visualize/image

Demonstrates how to use the `image` element, including wrapping it in a `figure` for captions and numbering, and making it inline using `box`.

```APIDOC
## `image` Element
A raster or vector graphic.

You can wrap the image in a `figure` to give it a number and caption.
Like most elements, images are _block-level_ by default and thus do not integrate themselves into adjacent paragraphs. To force an image to become inline, put it into a `box`.

### Example
```ty
#figure(
  image("molecular.jpg", width: 80%),
  caption: [
    A step in the molecular testing
    pipeline of our lab.
  ],
)
```
```

--------------------------------

### Add Custom Syntaxes to Raw Code Blocks

Source: https://typst.app/docs/reference/text/raw

Load additional syntax definitions for raw code blocks using the `syntaxes` parameter. Accepts paths, raw bytes, or an array of these.

```typ
#set raw(syntaxes: "SExpressions.sublime-syntax")
(defun factorial (x)
  (if (zerop x)
    ; with a comment
    1
    (* x (factorial (- x 1)))))

```

--------------------------------

### Combine Alignments with `+` Operator

Source: https://typst.app/docs/reference/layout/align

Combine two alignment types, such as `right + bottom`, using the `+` operator. This example demonstrates applying combined alignment to a specific piece of content using the function form.

```typst
#set page(height: 120pt)
Though left in the beginning ...

#align(right + bottom)[
  ... they were right in the end, \ 
  and with addition had gotten, \ 
  the paragraph to the bottom!
]


```

--------------------------------

### Retrieve current counter value with `get()`

Source: https://typst.app/docs/reference/introspection/counter

A contextual function that retrieves the counter's value at the current location. It always returns an array of integers, equivalent to `counter.at(here())`.

```typst
self.get() -> intarray
```

--------------------------------

### Typst CLI: Open Flag on Windows

Source: https://typst.app/docs/changelog/0.10.0

Resolves an issue with the `--open` flag on Windows where it would fail if the file path contained spaces. The flag now functions correctly with spaced paths.

```bash
--open
```

--------------------------------

### Override Cell Style and Position in Typst Grid

Source: https://typst.app/docs/reference/layout/grid

Demonstrates overriding the position and stroke for a single cell within a Typst grid. Includes setup for text styles and regex-based show rules.

```typst
#set text(15pt, font: "Noto Sans Symbols 2")
#show regex("[♚-♟︎]"): set text(fill: rgb("21212A"))
#show regex("[♔-♙]"): set text(fill: rgb("111015"))

#grid(
  fill: (x, y) => rgb(
    if calc.odd(x + y) { "7F8396" }
    else { "EFF0F3" }
  ),
  columns: (1em,) * 8,
  rows: 1em,
  align: center + horizon,

  [♖], [♘], [♗], [♕], [♔], [♗], [♘], [♖],
  [♙], [♙], [♙], [♙], [],  [♙], [♙], [♙],
  grid.cell(
    x: 4, y: 3,
    stroke: blue.transparentize(60%)
  )[♙],

  ..(grid.cell(y: 6)[♟︎],) * 8,
  ..([♜], [♞], [♝], [♛], [♚], [♝], [♞], [♜])
    .map(grid.cell.with(y: 7)),
)


```

--------------------------------

### Call a Typst Function

Source: https://typst.app/docs/reference/foundations/function

Demonstrates basic function calls with positional arguments, named arguments, and trailing content blocks. Parentheses can be omitted if the argument list is empty.

```typst
#list([A], [B])

// Named arguments and trailing
// content blocks.
#enum(start: 2)[A][B]

// Version without parentheses.
#list[A][B]
```

--------------------------------

### Typst: Styling All Headings Centered and Small Caps

Source: https://typst.app/docs/tutorial/advanced-styling

Applies global styling to all headings in a Typst document. It uses `show` rules to center headings, set their size to 13pt with regular weight, and apply the `smallcaps` function.

```typst
#show heading: set align(center)
#show heading: set text(
  size: 13pt,
  weight: "regular",
)
#show heading: smallcaps

... 

= Introduction
...

== Motivation
...

```

--------------------------------

### Convert Bytes to Array in Typst

Source: https://typst.app/docs/reference/foundations/array

Shows how to convert a string (encoded as bytes) into an array using the `array()` constructor with `bytes()`.

```typst
#let hi = "Hello 😃"
#array(bytes(hi))
```

--------------------------------

### Create and Convert Duration

Source: https://typst.app/docs/reference/foundations/duration

Construct a duration using days and hours, then retrieve the total hours.

```typst
#duration(
  days: 3,
  hours: 12,
).hours()

```

--------------------------------

### Construct and Manipulate Arrays in Typst

Source: https://typst.app/docs/reference/foundations/array

Demonstrates array construction, element access, modification, and common array operations. Note the special syntax for single-element arrays `(1,)` and empty arrays `()`.

```typst
#let values = (1, 7, 4, -3, 2)

#values.at(0) \
#(values.at(0) = 3)
#values.at(-1) \
#values.find(calc.even) \
#values.filter(calc.odd) \
#values.map(calc.abs) \
#values.rev() \
#(1, (2, 3)).flatten() \
#(("A", "B", "C")
    .join(", ", last: " and "))
```

--------------------------------

### Label Constructor and Usage

Source: https://typst.app/docs/reference/foundations/label

This section details how to create and use labels in Typst, including the constructor and dedicated syntax.

```APIDOC
## Label Constructor and Usage

### Description
A label for an element. Inserting a label into content attaches it to the closest preceding element that is not a space. The preceding element must be in the same scope as the label.
A labelled element can be referenced, queried for, and styled through its label.

### Constructor
`label(name: str) -> label`

Creates a label from a string.

#### Parameters
- **name** (str) - Required - The name of the label. Unlike the dedicated syntax, this constructor accepts any non-empty string, including names with special characters.

### Syntax
Labels can also be created using dedicated syntax by enclosing the label name in angle brackets (`<...>`). This works in both markup and code. A label's name can contain letters, numbers, `_`, `-`, `:`, and `.`. A label cannot be empty.

**Note:** There is a syntactical difference when using the dedicated syntax. For example, `= Introduction <a>` attaches the label `a` to the heading itself, while `= Conclusion #label("b")` attaches the label `b` to the heading's text.

### Example
```typc
#show <a>: set text(blue)
#show label("b"): set text(red)

= Heading <a>
*Strong* #label("b")
```

### Limitations
Currently, labels can only be attached to elements in markup mode, not in code mode.
```

--------------------------------

### Typst Visualization: Patterns for Fills and Strokes

Source: https://typst.app/docs/changelog/0.10.0

Introduces support for using patterns as fills and strokes in Typst visualizations. This allows for more complex and textured graphical elements.

```typst
patterns
```

--------------------------------

### Block Element Parameters

Source: https://typst.app/docs/reference/layout/block

Detailed documentation of the parameters available for the `block` element, including their types, settability, and default values.

```APIDOC
## Parameters

Parameters are the inputs to a function. They are specified in parentheses after the function name.
block(
width: autorelative,height: autorelativefraction,breakable: bool,fill: nonecolorgradienttiling,stroke: nonelengthcolorgradientstroketilingdictionary,radius: relativedictionary,inset: relativedictionary,outset: relativedictionary,spacing: relativefraction,above: autorelativefraction,below: autorelativefraction,clip: bool,sticky: bool,nonecontent,
) -> content

### `width`
auto or relative
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
The block's width.
```typ
#set align(center)
#block(
  width: 60%,
  inset: 8pt,
  fill: silver,
  lorem(10),
)
```

Default: `auto`

### `height`
auto or relative or fraction
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
The block's height. When the height is larger than the remaining space on a page and `breakable` is `true`, the block will continue on the next page with the remaining height.
```typ
#set page(height: 80pt)
#set align(center)
#block(
  width: 80%,
  height: 150%,
  fill: aqua,
)
```

Default: `auto`

### `breakable`
bool
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
Whether the block can be broken and continue on the next page.
```typ
#set page(height: 80pt)
The following block will
jump to its own page.
#block(
  breakable: false,
  lorem(15),
)
```

Default: `true`

### `fill`
none or color or gradient or tiling
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
The block's background color. See the rectangle's documentation for more details.
Default: `none`

### `stroke`
none or length or color or gradient or stroke or tiling or dictionary
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
The block's border color. See the rectangle's documentation for more details.
Default: `(:)`

### `radius`
relative or dictionary
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
How much to round the block's corners. See the rectangle's documentation for more details.
Default: `(:)`

### `inset`
relative or dictionary
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
How much to pad the block's content. See the box's documentation for more details.
Default: `(:)`

### `outset`
relative or dictionary
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
How much to expand the block's size without affecting the layout. See the box's documentation for more details.
Default: `(:)`

### `spacing`
relative or fraction
The spacing around the block. When `auto`, inherits the paragraph `spacing`.
For two adjacent blocks, the larger of the first block's `above` and the second block's `below` spacing wins. Moreover, block spacing takes precedence over paragraph `spacing`.
Note that this is only a shorthand to set `above` and `below` to the same value. Since the values for `above` and `below` might differ, a context block only provides access to `block.above` and `block.below`, not to `block.spacing` directly.
This property can be used in combination with a show rule to adjust the spacing around arbitrary block-level elements.
```typ
#set align(center)
#show math.equation: set block(above: 8pt, below: 16pt)

This sum of $x$ and $y$:
$ x + y = z $
A second paragraph.
```

Default: `1.2em`

### `above`
auto or relative or fraction
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
The spacing between this block and its predecessor.
Default: `auto`

### `below`
auto or relative or fraction
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
The spacing between this block and its successor.
Default: `auto`
```

--------------------------------

### Import and Apply a Typst Template Function

Source: https://typst.app/docs/guides/for-latex-users

This snippet demonstrates how to import a custom template function (conf) from a local file and apply it to the entire document using a show rule. The .with method is used to pre-configure arguments like title, authors, and abstract before applying the styling.

```typst
#import "conf.typ": conf
#show: conf.with(
  title: [
    Towards Improved Modelling
  ],
  authors: (
    (
      name: "Theresa Tungsten",
      affiliation: "Artos Institute",
      email: "tung@artos.edu",
    ),
    (
      name: "Eugene Deklan",
      affiliation: "Honduras State",
      email: "e.deklan@hstate.hn",
    ),
  ),
  abstract: lorem(80),
)

Let's get started writing this
article by putting insightful
paragraphs right here!

```

--------------------------------

### Import and Use Typst Module Definitions

Source: https://typst.app/docs/reference/foundations/module

Demonstrates how to import definitions from Typst modules using the '#import' syntax. It shows direct import and aliased import, followed by calling functions from the imported module. This is useful for organizing and reusing code within Typst projects.

```typst
#import "utils.typ"
#utils.add(2, 5)

#import utils: sub
#sub(1, 4)
```

--------------------------------

### Compare Typst Versions

Source: https://typst.app/docs/reference/foundations/version

Shows how to compare the current Typst compiler version (`sys.version`) with specific versions using comparison operators.

```typst
Current version: #sys.version \
```

```typst
#(sys.version >= version(0, 14, 0)) \
```

```typst
#(version(3, 2, 0) > version(4, 1, 0))
```

--------------------------------

### Length Units and Operations

Source: https://typst.app/docs/reference/layout/length

Demonstrates the usage of different length units and arithmetic operations in Typst.

```APIDOC
## Length Units and Operations

A size or distance, possibly expressed with contextual units. Typst supports the following length units:
  * Points: `72pt`
  * Millimeters: `254mm`
  * Centimeters: `2.54cm`
  * Inches: `1in`
  * Relative to font size: `2.5em`

You can multiply lengths with and divide them by integers and floats.

### Request Example
```
#rect(width: 20pt)
#rect(width: 2em)
#rect(width: 1in)

#(3em + 5pt).em \
#(20pt).em \
#(40em + 2pt).abs \
#(5em).abs
```

### Fields
  * `abs`: A length with just the absolute component of the current length (that is, excluding the `em` component).
  * `em`: The amount of `em` units in this length, as a float.
```

--------------------------------

### Typst: Create Two-Column Author Layout with Grid

Source: https://typst.app/docs/tutorial/advanced-styling

Demonstrates using the `#grid` function to create a two-column layout for author information. Each column is centered and contains contact details.

```typst
#grid(
  columns: (1fr, 1fr),
  align(center)[
    Therese Tungsten \ 
    Artos Institute \ 
    #link("mailto:tung@artos.edu")
  ],
  align(center)[
    Dr. John Doe \ 
    Artos Institute \ 
    #link("mailto:doe@artos.edu")
  ]
)

```

--------------------------------

### math.op Element Documentation

Source: https://typst.app/docs/reference/math/op

Documentation for the math.op element, which allows for the creation of custom text operators within Typst equations. It details the function signature, parameter types, and usage examples.

```APIDOC
## `op` Element

Element functions can be customized with `set` and `show` rules. A text operator in an equation.

### Example
```typst
$ tan x = (sin x)/(cos x) $
$ op("custom",
     limits: #true)_(n->oo) n $
```

## Predefined Operators
Typst predefines the operators `arccos`, `arcsin`, `arctan`, `arg`, `cos`, `cosh`, `cot`, `coth`, `csc`, `csch`, `ctg`, `deg`, `det`, `dim`, `exp`, `gcd`, `lcm`, `hom`, `id`, `im`, `inf`, `ker`, `lg`, `lim`, `liminf`, `limsup`, `ln`, `log`, `max`, `min`, `mod`, `Pr`, `sec`, `sech`, `sin`, `sinc`, `sinh`, `sup`, `tan`, `tanh`, `tg` and `tr`.

## Parameters

Parameters are the inputs to a function. They are specified in parentheses after the function name.

### `math.op` Function Signature
```
math.op(
    content: auto,
    limits: bool
) -> content
```

### `content` Parameter
- **content** (auto) - Required Positional
  Positional parameters are specified in order, without names.
  The operator's text.

### `limits` Parameter
- **limits** (bool) - Settable
  Settable parameters can be customized for all following uses of the function with a `set` rule.
  Whether the operator should show attachments as limits in display mode.
  Default: `false`
```

--------------------------------

### Unpacking syntax for let bindings

Source: https://typst.app/docs/changelog/0.2.0

Use this syntax to destructure arrays directly into variables.

```typst
let (1, 2) = array
```

--------------------------------

### Set Line Number Margin

Source: https://typst.app/docs/reference/model/par

Defines the margin where line numbers appear. `start` places them at the beginning of the line, while `end` places them at the end. Note that in multi-column layouts, numbers in the last column always use the `end` margin.

```typst
#set par.line(
  numbering: "1",
  number-margin: right,
)

= Report
- Brightness: Dark, yet darker
- Readings: Negative

```

--------------------------------

### Set Heading Numbering Style in Typst

Source: https://typst.app/docs/tutorial/formatting

Applies numbering to headings in a Typst document. This example shows how to set the numbering format for headings using Arabic numerals with a dot separator.

```typst
#set heading(numbering: "1.")

= Introduction
#lorem(10)

== Background
#lorem(12)

== Methods
#lorem(15)
```

--------------------------------

### Basic `vec` Usage

Source: https://typst.app/docs/reference/math/vec

Demonstrates the basic usage of the `vec` function for typesetting vector components in a mathematical equation.

```typst
$ vec(a, b, c) dot vec(1, 2, 3) 
    = a + 2b + 3c $
```

--------------------------------

### Control Line Numbering Scope

Source: https://typst.app/docs/reference/model/par

Determines when line numbering resets. Set to `"page"` to reset at the start of each new page, or `"document"` to maintain a continuous count throughout. This setting should be defined before page content.

```typst
#set par.line(
  numbering: "1",
  numbering-scope: "page",
)

First line 
Second line
#pagebreak()
First line again 
Second line again

```

--------------------------------

### Typst Method Calls

Source: https://typst.app/docs/reference/scripting

Demonstrates Typst's method call syntax, which provides a concise way to call functions scoped to a value's type. It shows equivalent calls using dot notation and full function calls.

```typst
#let values = (1, 2, 3, 4)
#values.pop() 
#values.len() 

#("a, b, c"
    .split(", ")
    .join[ --- ])

#"abc".len() is the same as
#str.len("abc")

```

--------------------------------

### Color Space

Source: https://typst.app/docs/reference/visualize/color

Returns the constructor function for the color's space. Supported spaces include luma, oklab, oklch, linear-rgb, rgb, cmyk, hsl, and hsv.

```APIDOC
## `space`

### Description
Returns the constructor function for this color's space.

### Method
`self.space()`

### Endpoint
N/A (Method on color object)

### Parameters
None

### Request Example
```
#let color = cmyk(1%, 2%, 3%, 4%)
#(color.space() == cmyk)
```

### Response
#### Success Response (200)
- **space** (constructor function) - The constructor function of the color's space.

#### Response Example
`cmyk` (constructor function)
```

--------------------------------

### Adjust Text Layout Costs

Source: https://typst.app/docs/reference/text/text

Customizes the 'cost' of various layout choices to influence text flow. Higher costs make the engine less likely to choose a specific layout option. This example increases the hyphenation cost significantly.

```typst
#set text(hyphenate: true, size: 11.4pt)
#set par(justify: true)

#lorem(10)

// Set hyphenation to ten times the normal cost.
#set text(costs: (hyphenation: 1000%))

#lorem(10)

```

--------------------------------

### Set Heading Numbering with Letters in Typst

Source: https://typst.app/docs/tutorial/formatting

Configures headings in a Typst document to use letter-based numbering. This example demonstrates setting heading numbering to Arabic numerals followed by lowercase letters.

```typst
#set heading(numbering: "1.a")

= Introduction
#lorem(10)

== Background
#lorem(12)

== Methods
#lorem(15)
```

--------------------------------

### Typst Math Expression with Sub/Superscripts

Source: https://typst.app/docs/reference/math/attach

Demonstrates the basic syntax for creating mathematical expressions with subscripts and superscripts using Typst's attachment syntax.

```typst
$ sum_(i=0)^n a_i = 2^(1+i) $
```

--------------------------------

### Enum Element Syntax

Source: https://typst.app/docs/reference/model/enum

Explains the markup syntax for creating enumeration items.

```APIDOC
## Enum Element Syntax

### Description
This functions also has dedicated syntax:

* Starting a line with a plus sign creates an automatically numbered enumeration item.
* Starting a line with a number followed by a dot creates an explicitly numbered enumeration item.

Enumeration items can contain multiple paragraphs and other block-level content. All content that is indented more than an item's marker becomes part of that item.
```

--------------------------------

### Typst `parbreak` Usage in a For Loop

Source: https://typst.app/docs/reference/model/parbreak

Demonstrates how to use the `parbreak` element within a Typst `for` loop to create distinct paragraphs for each iteration. This example shows generating blind text and a paragraph break after each item.

```typst
#for i in range(3) {
  [Blind text #i: ]
  lorem(5)
  parbreak()
}

```

--------------------------------

### Basic Curve with Fill and Stroke

Source: https://typst.app/docs/reference/visualize/curve

Demonstrates creating a basic curve with a fill and stroke. The `curve.move`, `curve.line`, `curve.cubic`, and `curve.close` functions are used to define the shape's path.

```typst
#curve(
  fill: blue.lighten(80%),
  stroke: blue,
  curve.move((0pt, 50pt)),
  curve.line((100pt, 50pt)),
  curve.cubic(none, (90pt, 0pt), (50pt, 0pt)),
  curve.close(),
)

```

--------------------------------

### Set Text Direction

Source: https://typst.app/docs/reference/text/text

Configures the dominant direction for text layout. Use `rtl` for right-to-left scripts like Arabic or Hebrew to ensure correct placement of punctuation and inline objects. This setting also affects alignment values `start` and `end`.

```typst
#set text(dir: rtl)
هذا عربي.

```

--------------------------------

### Array `windows` Method

Source: https://typst.app/docs/reference/foundations/array

Returns sliding windows of a specified size over an array.

```APIDOC
## `windows`

### Description
Returns sliding windows of `window-size` elements over an array. If the array length is less than `window-size`, this will return an empty array.

### Method
`self.windows(window-size: int) -> array`

### Parameters
#### Path Parameters
- **window-size** (int) - Required - How many elements each window will contain.

### Request Example
```
#let array = (1, 2, 3, 4, 5, 6, 7, 8)
#array.windows(5)
```
```

--------------------------------

### Typst Set Rule: Scoping List Styles with Blocks

Source: https://typst.app/docs/reference/styling

Illustrates how to use a content block in Typst to scope the effect of a 'set rule'. In this example, the list marker style is applied only to the list within the block.

```typst
This list is affected: #[
  #set list(marker: [--])
  - Dash
]

This one is not:
- Bullet

```

--------------------------------

### Transition API for Side Effects

Source: https://typst.app/docs/reference/foundations/plugin

The `transition` API allows calling plugin functions that have side effects, returning a new module that reflects these changes. It's crucial for managing mutable state within plugins.

```APIDOC
## plugin.transition

### Description
Calls a plugin function that has side effects and returns a new module with plugin functions that are guaranteed to have observed the results of the mutable call. Note that calling an impure function through a normal function call (without use of the transition API) is forbidden and leads to unpredictable behaviour.

### Method
`plugin.transition`

### Parameters
#### Path Parameters
- **func** (function) - Required - The plugin function to call.
- **arguments** (bytes) - Required - Variadic - The byte buffers to call the function with.

### Request Example
```typescript
// Example usage:
let base = plugin("hello-mut.wasm");
assert.eq(base.get(), "[]");

let mutated = plugin.transition(base.add, "hello");
assert.eq(base.get(), "[]");
assert.eq(mutated.get(), "[hello]");
```

### Response
#### Success Response (module)
- **module** - A new module object reflecting the changes from the side-effectful function call.
```

--------------------------------

### Bytes Methods

Source: https://typst.app/docs/reference/foundations/bytes

Documentation for the methods available on bytes objects.

```APIDOC
## Bytes Methods

### `len`

#### Description
The length in bytes.

#### Syntax
```
self.len()
```

#### Returns
- int - The length in bytes.

### `at`

#### Description
Returns the byte at the specified index. Returns the default value if the index is out of bounds or fails with an error if no default value was specified.

#### Syntax
```
self.at(int, default: any)
```

#### Parameters
##### `index`
- **index** (int) - Required Positional - The index at which to retrieve the byte.
##### `default`
- **default** (any) - Optional - A default value to return if the index is out of bounds.

#### Returns
- any - The byte at the specified index or the default value.

### `slice`

#### Description
Extracts a subslice of the bytes. Fails with an error if the start or end index is out of bounds.

#### Syntax
```
self.slice(start: int, end: none or int, count: int)
```

#### Parameters
##### `start`
- **start** (int) - Required Positional - The start index (inclusive).
##### `end`
- **end** (none or int) - Optional Positional - The end index (exclusive). If omitted, the whole slice until the end is extracted. Default: `none`.
##### `count`
- **count** (int) - Optional - The number of items to extract. This is equivalent to passing `start + count` as the `end` position. Mutually exclusive with `end`.

#### Returns
- bytes - A subslice of the original bytes.
```

--------------------------------

### Two Closed Shapes with Different Fills

Source: https://typst.app/docs/reference/visualize/curve

Draws two distinct closed shapes using the `curve` element. The first shape is a filled square, and the second is a smaller filled square offset from the first. This example showcases `curve.move`, `curve.line`, `curve.close`, and `fill-rule`.

```typst
#curve(
  fill: blue.lighten(80%),
  fill-rule: "even-odd",
  stroke: blue,
  curve.line((50pt, 0pt)),
  curve.line((50pt, 50pt)),
  curve.line((0pt, 50pt)),
  curve.close(),
  curve.move((10pt, 10pt)),
  curve.line((40pt, 10pt)),
  curve.line((40pt, 40pt)),
  curve.line((10pt, 40pt)),
  curve.close(),
)

```

--------------------------------

### Force Display Style in Math

Source: https://typst.app/docs/reference/math/sizes

Use the `display` function for normal size in block equations. It takes content and an optional `cramped` boolean.

```typst
$sum_i x_i/2 = display(sum_i x_i/2)$
```

--------------------------------

### Typst: Striped Rows using Stroke Argument

Source: https://typst.app/docs/guides/tables

Achieves striped rows in a table by manipulating the `stroke` argument. This example applies a 1pt stroke to all horizontal borders and conditionally applies strokes to the left and right borders of even-numbered rows.

```typst
#set table(
  stroke: (x, y) => (
    y: 1pt,
    left: if x > 0 { 0pt } else if calc.even(y) { 1pt },
    right: if calc.even(y) { 1pt },
  ),
)

```

--------------------------------

### Nested Contexts with Counter Updates

Source: https://typst.app/docs/reference/context

Demonstrates how nested contexts affect counter state in Typst. The example shows that an inner context block sees counter updates made within it, while an outer context block might not, depending on evaluation order.

```typst
#let c = counter("mycounter")
#c.update(1)
#context [
  #c.update(2)
  #c.display() \ 
  #context c.display()
]


```

--------------------------------

### Customizing Table Cells

Source: https://typst.app/docs/reference/model/table

Demonstrates how to customize individual table cells using `table.cell` and `show` rules.

```APIDOC
## Customizing Table Cells with `table.cell` and `show` rules

### Description
Customize the appearance and position of individual cells using `table.cell`. This example also shows how to apply global styling to cells using `show table.cell` rules.

### Method
`table.cell()`

### Endpoint
N/A (Typst element)

### Parameters
`table.cell` accepts parameters similar to the `table` element for cell-specific styling, such as `fill` and `inset`.

### Request Example
```typst
#set table(stroke: none, gutter: 0.2em)

#show table.cell: it => {
  if it.x == 0 or it.y == 0 {
    set text(white)
    strong(it)
  } else if it.body == [] {
    pad(..it.inset)[_N/A_]
  } else {
    it
  }
}

#let a = table.cell(fill: green.lighten(60%))[A]
#let b = table.cell(fill: aqua.lighten(60%))[B]

#table(
  columns: 4,
  [], [Exam 1], [Exam 2], [Exam 3],
  [John], [], a, [],
  [Mary], [], a, a,
  [Robert], b, a, b,
)
```

### Response
#### Success Response (200)
- **content** (content) - The rendered table with customized cells.

#### Response Example
(No specific response example provided for the element itself, as it's rendered within the Typst document.)
```

--------------------------------

### Create a Datetime Object

Source: https://typst.app/docs/reference/foundations/datetime

Constructs a new datetime object with specified year, month, and day. The display format will default to a date.

```typst
#datetime(
  year: 2012,
  month: 8,
  day: 3,
).display()
```

--------------------------------

### Typst Emph Element Usage and Customization

Source: https://typst.app/docs/reference/model/emph

Demonstrates how to use the `emph` element in Typst for emphasizing text and how to customize its appearance using `show` rules. The `emph` element toggles italics, and the example shows how to change the emphasized text color to blue.

```typst
This is _emphasized._ 
This is #emph[too.]

#show emph: it => {
  text(blue, it.body)
}

This is _emphasized_ differently.

```

--------------------------------

### Typst colbreak Element Usage

Source: https://typst.app/docs/reference/layout/colbreak

Demonstrates how to use the `colbreak` element in Typst to force a break into the next column. This is particularly useful in multi-column layouts to control content flow. The example sets up a two-column page and then inserts a `colbreak` to separate content.

```typst
#set page(columns: 2)
Preliminary findings from our
ongoing research project have
revealed a hitherto unknown
phenomenon of extraordinary
significance.

#colbreak()
Through rigorous experimentation
and analysis, we have discovered
a hitherto uncharacterized process
that defies our current
understanding of the fundamental
laws of nature.

```

--------------------------------

### Styling Raw Elements with Show-Set Rules

Source: https://typst.app/docs/reference/text/raw

Customizes the font and size for raw text blocks and inline raw text using `show-set` rules.

```typst
#show raw: set text(font: "Cascadia Code")

// Reset raw blocks to the same size as normal text,
// but keep inline raw at the reduced size.
#show raw.where(block: true): set text(1em / 0.8)

Now using the `Cascadia Code` font for raw text.
Here's some Python code. It looks larger now:

```py
def python():
  return 5 + 5
```

```

--------------------------------

### Enable Simple Line Numbering

Source: https://typst.app/docs/reference/model/par

Enables line numbering for paragraphs using a simple format. The `numbering` option accepts a string pattern or a function.

```typst
#set par.line(numbering: "1")

Roses are red. \
Violets are blue.
Typst is there for you.

```

--------------------------------

### Create a shadow tree slot

Source: https://typst.app/docs/reference/html/typed

Use `html.slot` to define a shadow tree slot. It requires a `name` parameter and accepts content.

```typst
html.slot(name: str,content)
```

--------------------------------

### Typst: Define Data Cell in Table with pdf.data-cell

Source: https://typst.app/docs/reference/pdf/data-cell

Demonstrates how to use pdf.data-cell to mark a cell as a data cell within a Typst table. This example showcases styling different cells and using pdf.data-cell for the 'Status' column in a header row. Requires the 'a11y-extras' feature flag.

```typst
#show table.cell.where(x: 0): set text(weight: "bold")
#show table.cell.where(x: 1): set text(style: "italic")
#show table.cell.where(x: 1, y: 0): set text(style: "normal")

#table(
  columns: 3,
  align: (left, left, center),

  table.header[Objective][Key Result][Status],

  table.header(
    level: 2,
    table.cell(colspan: 2)[Improve Customer Satisfaction],
    // Status is data for this objective, not a header
    pdf.data-cell[✓ On Track],
  ),
  [], [Increase NPS to 50+], [45],
  [], [Reduce churn to <5%], [4.2%],

  table.header(
    level: 2,
    table.cell(colspan: 2)[Grow Revenue],
    pdf.data-cell[⚠ At Risk],
  ),
  [], [Achieve $2M ARR], [$1.8M],
  [], [Close 50 enterprise deals], [38],
)

```

--------------------------------

### Typst `root` Function Usage

Source: https://typst.app/docs/reference/math/roots

Illustrates the general root function in Typst, which takes an index and a radicand. The `index` parameter is optional and defaults to `none`.

```typst
$ root(3, x) $
```

--------------------------------

### Create Block Equation with Alt Text

Source: https://typst.app/docs/reference/math/equation

Renders a block-level equation and provides an alternative text description for accessibility. The `alt` parameter should describe the equation in natural language.

```typst
#math.equation(
  alt: "integral from 1 to infinity of a x squared plus b with respect to x",
  block: true,
  $ integral_1^oo a x^2 + b dif x $
)

```

--------------------------------

### Length Conversion Methods

Source: https://typst.app/docs/reference/layout/length

Explains how to convert lengths to different units and resolve them to absolute values.

```APIDOC
## Length Conversion Methods

Functions and types can have associated definitions. These are accessed by specifying the function or type, followed by a period, and then the definition's name.

### `pt`
Converts this length to points.
Fails with an error if this length has non-zero `em` units (such as `5em + 2pt` instead of just `2pt`). Use the `abs` field (such as in `(5em + 2pt).abs.pt()`) to ignore the `em` component of the length (thus converting only its absolute component).
```
self.pt(
) -> float
```

### `mm`
Converts this length to millimeters.
Fails with an error if this length has non-zero `em` units. See the `pt` method for more details.
```
self.mm(
) -> float
```

### `cm`
Converts this length to centimeters.
Fails with an error if this length has non-zero `em` units. See the `pt` method for more details.
```
self.cm(
) -> float
```

### `inches`
Converts this length to inches.
Fails with an error if this length has non-zero `em` units. See the `pt` method for more details.
```
self.inches(
) -> float
```

### `to-absolute`
Resolve this length to an absolute length.

#### Request Example
```
#set text(size: 12pt)
#context [
  #(6pt).to-absolute() \
  #(6pt + 10em).to-absolute() \
  #(10em).to-absolute()
]

#set text(size: 6pt)
#context [
  #(6pt).to-absolute() \
  #(6pt + 10em).to-absolute() \
  #(10em).to-absolute()
]
```

```
self.to-absolute(
) -> length
```
```

--------------------------------

### Typst: Horizontally Striped Table with Custom Styles

Source: https://typst.app/docs/guides/tables

Generates a table with horizontally striped rows using a light blue fill for odd-numbered rows. It also applies custom strokes and text styling for headers and specific columns. This example demonstrates combining fill and stroke for enhanced table appearance.

```typst
#set text(font: "IBM Plex Sans")

// Medium bold table header.
#show table.cell.where(y: 0): set text(weight: "medium")

// Bold titles.
#show table.cell.where(x: 1): set text(weight: "bold")

// See the strokes section for details on this!
#let frame(stroke) = (x, y) => (
  left: if x > 0 { 0pt } else { stroke },
  right: stroke,
  top: if y < 2 { stroke } else { 0pt },
  bottom: stroke,
)

#set table(
  fill: (rgb("EAF2F5"), none),
  stroke: frame(1pt + rgb("21222C")),
)

#table(
  columns: (0.4fr, 1fr, 1fr, 1fr),

  table.header[Month][Title][Author][Genre],
  [January], [The Great Gatsby], [F. Scott Fitzgerald], [Classic],
  [February], [To Kill a Mockingbird], [Harper Lee], [Drama],
  [March], [1984], [George Orwell], [Dystopian],
  [April], [The Catcher in the Rye], [J.D. Salinger], [Coming-of-Age],
)

```

--------------------------------

### Configure Character-Level Justification

Source: https://typst.app/docs/changelog/0.14.0

Opt-in support for character-level justification is available via the `par.justification-limits` property, offering improved microtypography for justified text.

```typst
par(justification-limits: (..))
```

--------------------------------

### Align Table Cells with a Function in Typst

Source: https://typst.app/docs/guides/tables

Shows how to use a function with the `align` argument in Typst tables to define complex alignment rules. This example aligns the first column to the right and all other columns to the left, while also aligning header cells to the bottom and other cells to the top. It utilizes the `+` operator to combine horizontal and vertical alignments.

```typst
#set text(font: "IBM Plex Sans")
#show table.cell.where(y: 0): set text(weight: "bold")

#table(
  columns: 4,
  align: (x, y) =>
    if x == 0 { right } else { left } +
    if y == 0 { bottom } else { top },
  fill: (_, y) => if calc.odd(y) { green.lighten(90%) },
  stroke: none,

  table.header[Day][Location][Hotel or Apartment][Activities],
  [1], [Paris, France], [Hôtel de l'Europe], [Arrival, Evening River Cruise],
  [2], [Paris, France], [Hôtel de l'Europe], [Louvre Museum, Eiffel Tower],
 // ... remaining days omitted
)

```

--------------------------------

### Typm Highlighting for Math in Raw Blocks

Source: https://typst.app/docs/changelog/0.12.0

A new `typm` highlighting mode has been added for mathematical content within raw code blocks.

```typst
typm
```

--------------------------------

### Draw a cubic Bézier curve with control points

Source: https://typst.app/docs/reference/visualize/curve

Illustrates drawing a cubic Bézier curve using `curve.cubic`. The `control-start` and `control-end` points shape the curve. The `handle` function visualizes these control points.

```typst
// Function to illustrate where the control points are.
#let handle(start, end) = place(
  line(stroke: red, start: start, end: end)
)

#handle((0pt, 80pt), (10pt, 20pt))
#handle((90pt, 60pt), (100pt, 0pt))

#curve(
  stroke: blue,
  curve.move((0pt, 80pt)),
  curve.cubic((10pt, 20pt), (90pt, 60pt), (100pt, 0pt)),
)

```

--------------------------------

### Enable Page Breaking for Tables within Figures in Typst

Source: https://typst.app/docs/guides/tables

This code snippet demonstrates how to make a Typst figure containing a table breakable across pages. It uses a show rule to reconfigure the figure's block to be `breakable: true`, allowing the table content to span multiple pages. The example also includes table headers and footers that repeat on each page.

```typst
#set page(width: 9cm, height: 6cm)
#show table.cell.where(y: 0): set text(weight: "bold")
#show figure: set block(breakable: true)

#figure(
  caption: [Training regimen for Marathon],
  table(
    columns: 3,
    fill: (_, y) => if y == 0 { gray.lighten(75%) },

    table.header[Week][Distance (km)][Time (hh:mm:ss)],
    [1], [5],  [00:30:00],
    [2], [7],  [00:45:00],
    [3], [10], [01:00:00],
    [4], [12], [01:10:00],
    [5], [15], [01:25:00],
    [6], [18], [01:40:00],
    [7], [20], [01:50:00],
    [8], [22], [02:00:00],
    [...], [...], [...],
    table.footer[_Goal_][_42.195_][_02:45:00_],
  )
)

```

--------------------------------

### Create a search Element

Source: https://typst.app/docs/reference/html/typed

Use html.search to create a container for search controls. It takes content as its only parameter.

```typst
html.search(content,)
```

--------------------------------

### Override Cell Fill Color in Typst Table

Source: https://typst.app/docs/guides/tables

This Typst code snippet demonstrates how to manually override a cell's fill color within a table. It defines helper functions for different political parties (CDU, SPD, FDP) to create cells with specific background colors and text colors, making the table more visually informative. The example also shows how to use the spread operator (`..`) to insert array elements as individual cells and customizes table appearance by removing strokes and setting a font.

```typst
#set text(font: "Roboto")

#let cdu(name) = ([CDU], table.cell(fill: black, text(fill: white, name)))
#let spd(name) = ([SPD], table.cell(fill: red, text(fill: white, name)))
#let fdp(name) = ([FDP], table.cell(fill: yellow, name))

#table(
  columns: (auto, auto, 1fr),
  stroke: (x: none),

  table.header[Tenure][Party][President],
  [1949-1959], ..fdp[Theodor Heuss],
  [1959-1969], ..cdu[Heinrich Lübke],
  [1969-1974], ..spd[Gustav Heinemann],
  [1974-1979], ..fdp[Walter Scheel],
  [1979-1984], ..cdu[Karl Carstens],
  [1984-1994], ..cdu[Richard von Weizsäcker],
  [1994-1999], ..cdu[Roman Herzog],
  [1999-2004], ..spd[Johannes Rau],
  [2004-2010], ..cdu[Horst Köhler],
  [2010-2012], ..cdu[Christian Wulff],
  [2012-2017], [n/a], [Joachim Gauck],
  [2017-],     ..spd[Frank-Walter-Steinmeier],
)

```

--------------------------------

### Set Table Inset with Different Values

Source: https://typst.app/docs/reference/model/table

Demonstrates setting table cell inset using a single value for all sides and a dictionary for specific sides (x and y).

```typst
#table(
  columns: 2,
  inset: 10pt,
  [Hello],
  [World],
)
```

```typst
#table(
  columns: 2,
  inset: (x: 20pt, y: 10pt),
  [Hello],
  [World],
)
```

--------------------------------

### Typst Math Mode Input

Source: https://typst.app/docs/guides/for-latex-users

Demonstrates how to enter and format mathematical equations in Typst using dollar signs for inline and display math. It covers basic formatting, variables, symbols, text, delimiters, fractions, sub/superscripts, function calls, and Unicode input.

```typst
$ sum_(k=1)^n k = (n(n+1))/2 $
```

```typst
$ delta "if" x <= 5 $
```

```typst
$ f(x) = (x + 1) / x $
```

```typst
$x^2$
```

```typst
$x_2$
```

```typst
$x_(a -> epsilon)$
```

```typst
$ f(x, y) := cases(
  1 "if" (x dot y)/2 <= 0,
  2 "if" x "is even",
  3 "if" x in NN,
  4 "else",
) $
```

```typst
$ (a + b)^2
  = a^2
  + text(fill: #maroon, 2 a b)
  + b^2 $
```

```typst
$ sum^10_(🤓=1)
  #rect(width: 4mm, height: 2mm)/🤓
  = 🧠 maltese $
```

```typst
$ mat(
  1, 2, ..., 10;
  2, 2, ..., 10;
  dots.v, dots.v, dots.down, dots.v;
  10, 10, ..., 10;
) $
```

--------------------------------

### Outline Function Signature and Parameters

Source: https://typst.app/docs/reference/model/outline

Details the parameters available for the `outline` function.

```APIDOC
## Outline Function Parameters

### Description
Provides a detailed overview of the parameters accepted by the `outline` function.

### Method
`outline(<parameters>)

### Parameters
#### Settable Parameters
- **`title`** (none | auto | content) - Optional - The title of the outline. Defaults to `auto`.
- **`target`** (label | selector | location | function) - Optional - The type of element to include in the outline. Defaults to `heading`.
- **`depth`** (none | int) - Optional - The maximum depth of elements to include. Defaults to `auto`.
- **`indent`** (auto | relative | function) - Optional - Controls the indentation of outline entries. Defaults to `auto`.

### Request Example
```typ
#outline(
  title: [Custom Title],
  depth: 2,
  target: figure
)
```

### Response Example
(The rendered outline based on the provided parameters.)
```

--------------------------------

### Create a title element

Source: https://typst.app/docs/reference/html/typed

Use the `title` function to set the document title. It accepts content as its parameter.

```typst
html.title(content)
```

--------------------------------

### Custom Enum Numbering Patterns

Source: https://typst.app/docs/reference/model/enum

Demonstrates using custom numbering patterns and functions for enum items, including nested lists.

```typst
#set enum(numbering: "1.a)")
+ Different
+ Numbering
  + Nested
  + Items
+ Style

#set enum(numbering: n => super[#n])
+ Superscript
+ Numbering!
```

--------------------------------

### Array Constructor

Source: https://typst.app/docs/reference/foundations/array

Explains the `array()` constructor for converting collection-like values into arrays.

```APIDOC
## Array Constructor

### Description
The `array()` constructor converts a collection-like value into an array. It is intended for conversion, not for creating arrays from individual items.

### Syntax
`array(value: bytes | array | version) -> array`

### Parameters
#### `value`
- **value** (bytes | array | version) - Required Positional - The value that should be converted to an array.

### Example
```
#let hi = "Hello 😃"
#array(bytes(hi))
```
```

--------------------------------

### Smallcaps Element Parameters

Source: https://typst.app/docs/reference/text/smallcaps

Details the parameters available for the `smallcaps` element, including `all` and `body`.

```APIDOC
## Parameters

Parameters are the inputs to a function. They are specified in parentheses after the function name.

smallcaps(
all: bool,
content,
) -> content

### `all`

- **Type**: bool
- **Settable**: true

Settable parameters can be customized for all following uses of the function with a `set` rule. 
Whether to turn uppercase letters into small capitals as well. Unless overridden by a show rule, this enables the `c2sc` OpenType feature.

#### Example
```ty
#smallcaps(all: true)[UNICEF] is an
agency of #smallcaps(all: true)[UN].
```

- **Default**: `false`

### `body`

- **Type**: content
- **Required**: true
- **Positional**: true

Positional parameters are specified in order, without names. 
The content to display in small capitals.
```

--------------------------------

### Array Reduce Method

Source: https://typst.app/docs/reference/foundations/array

Demonstrates how to use the `reduce` method on an array to find the maximum element.

```APIDOC
## `reduce`

### Description
Reduces the elements to a single one, by repeatedly applying a reducing operation. If the array is empty, returns `none`, otherwise, returns the result of the reduction. The reducing function is a closure with two arguments: an "accumulator", and an element. For arrays with at least one element, this is the same as `array.fold` with the first element of the array as the initial accumulator value, folding every subsequent element into it.

### Method
`reduce`

### Endpoint
`array.reduce(function)`

### Parameters
#### Path Parameters
None

#### Query Parameters
None

#### Request Body
None

### Request Example
```typst
#let array = (2, 1, 4, 3)
#array.reduce((acc, x) => calc.max(acc, x))
```

### Response
#### Success Response (200)
- **return value** (any) - The single reduced value from the array.

#### Response Example
```json
{
  "example": "4"
}
```

### Error Handling
- If the array is empty, returns `none`.

### `reducer` Parameter
- **function** (closure) - Required Positional. The reducing function. Must have two parameters: One for the accumulated value and one for an item.
```

--------------------------------

### Typst List Item Generation with For Loop

Source: https://typst.app/docs/reference/model/list

Demonstrates how to dynamically generate list items in Typst using a `for` loop. Adjacent items are automatically collected into lists, even within loops.

```typst
#for letter in "ABC" [
  - Letter #letter
]

```

--------------------------------

### Grid Function Parameters

Source: https://typst.app/docs/reference/layout/grid

Details the parameters available for the `grid` function, including their types, settability, and default values.

```APIDOC
## grid

### Parameters

`grid(
columns: autointrelativefractionarray,
rows: autointrelativefractionarray,
gutter: autointrelativefractionarray,
column-gutter: autointrelativefractionarray,
row-gutter: autointrelativefractionarray,
inset: relativearraydictionaryfunction,
align: autoarrayalignmentfunction,
fill: nonecolorgradientarraytilingfunction,
stroke: nonelengthcolorgradientarraystroketilingdictionaryfunction,
..content,
) -> content`

### `columns`

- **Type**: `auto` or `int` or `relative` or `fraction` or `array`
- **Settable**: Yes
- **Description**: The column sizes. Can be an integer for auto-sized columns or a track size array. A single track size creates a single column.
- **Default**: `()`

### `rows`

- **Type**: `auto` or `int` or `relative` or `fraction` or `array`
- **Settable**: Yes
- **Description**: The row sizes. If there are more cells than defined rows, the last row is repeated.
- **Default**: `()`

### `gutter`

- **Type**: `auto` or `int` or `relative` or `fraction` or `array`
- **Settable**: No
- **Description**: Gaps between rows and columns. Shorthand for `column-gutter` and `row-gutter`.
- **Default**: `()`

### `column-gutter`

- **Type**: `auto` or `int` or `relative` or `fraction` or `array`
- **Settable**: Yes
- **Description**: Gaps between columns.
- **Default**: `()`

### `row-gutter`

- **Type**: `auto` or `int` or `relative` or `fraction` or `array`
- **Settable**: Yes
- **Description**: Gaps between rows.
- **Default**: `()`

### `inset`

- **Type**: `relative` or `array` or `dictionary` or `function`
- **Settable**: Yes
- **Description**: Padding for cell content. Can be a single length, a dictionary of lengths, or an array of insets per column. See box documentation for details.
- **Default**: `(:)`

### `align`

- **Type**: `auto` or `array` or `alignment` or `function`
- **Settable**: Yes
- **Description**: Alignment of cell content. `auto` uses outer alignment. Can be a single alignment, an array per column, or a function mapping cell position to alignment.
- **Default**: `auto`

### `fill`

- **Type**: `none` or `color` or `gradient` or `array` or `tiling` or `function`
- **Settable**: No
- **Description**: Fill color or pattern for the cells.

### `stroke`

- **Type**: `none` or `length` or `color` or `gradient` or `array` or `stroketiling` or `dictionary` or `function`
- **Settable**: No
- **Description**: Stroke style for the cells.

```

--------------------------------

### Basic `eval` Usage

Source: https://typst.app/docs/reference/foundations/eval

Demonstrates basic arithmetic and string evaluation using `eval`. The `len()` method is shown on the result of evaluating a tuple.

```typst
#eval("1 + 1") \ 
#eval("(1, 2, 3, 4)").len() \ 
#eval("*Markup!*", mode: "markup") \ 

```

--------------------------------

### Typst Integer Arithmetic and Literals

Source: https://typst.app/docs/reference/foundations/int

Demonstrates basic integer arithmetic operations and different literal formats (decimal, hexadecimal, octal, binary) in Typst.

```typst
#(1 + 2) \
#(2 - 5) \
#(3 + 4 < 8)

#0xff \
#0o10 \
#0b1001
```

--------------------------------

### Typst Stack Element Parameters

Source: https://typst.app/docs/reference/layout/stack

Illustrates the parameters available for the Typst `stack` element, including direction (`dir`) and spacing (`spacing`), and their possible values.

```typst
stack(
dir: direction,
spacing: none or relative or fraction,
...content,
) -> content

```

--------------------------------

### Inline vs. Block Quote with Attribution

Source: https://typst.app/docs/reference/model/quote

Illustrates the difference between inline and block quotes, showing how to specify attributions for each.

```typst
An inline citation would look like
this: #quote(
  attribution: [René Descartes]
)[
  cogito, ergo sum
], and a block equation like this:
#quote(
  block: true,
  attribution: [JFK]
)[
  Ich bin ein Berliner.
]
```

--------------------------------

### Scripting: Bytes and Array Conversion Functions

Source: https://typst.app/docs/changelog/0.7.0

New functions `bytes` and `array` facilitate conversion between strings/integer arrays and byte buffers. `bytes` converts strings or integer arrays to bytes, while `array` converts bytes to an array of integers.

```typst
let byte_buffer = bytes("Hello")
let int_array = array(byte_buffer)
```

```typst
let byte_buffer = bytes([72, 101, 108, 108, 111])
let text_string = str(byte_buffer)
```

--------------------------------

### Show Rule for Ref Element

Source: https://typst.app/docs/reference/model/ref

Illustrates how to use a `show` rule to customize the appearance of `ref` elements, specifically targeting equation references.

```APIDOC
If you write a show rule for references, you can access the referenced element through the `element` field of the reference. The `element` may be `none` even if it exists if Typst hasn't discovered it yet, so you always need to handle that case in your code.
```
#set heading(numbering: "1.")
#set math.equation(numbering: "(1)")

#show ref: it => {
  let eq = math.equation
  let el = it.element
  // Skip all other references.
  if el == none or el.func() != eq { return it }
  // Override equation references.
  link(el.location(), numbering(
    el.numbering,
    ..counter(eq).at(el.location())
  ))
}

= Beginnings <beginning>
In @beginning we prove @pythagoras.
$ a^2 + b^2 = c^2 $ <pythagoras>

```
```

--------------------------------

### Create an HTML embed element

Source: https://typst.app/docs/reference/html/typed

Use this function to create an HTML `<embed>` element for embedding plugins. It requires `height`, `src`, `type`, and `width` parameters.

```typst
html.embed(
height: int,src: str,type: str,width: int,
)
```

--------------------------------

### Typst Math: Symbols and Shorthands

Source: https://typst.app/docs/reference/math

Illustrates the use of mathematical symbols in Typst and how shorthand sequences like '=>' can approximate symbols. It shows how to use modifiers for symbol variants and references the symbol documentation for available shorthands.

```typst
$ x < y => x gt.eq.not y $
```

--------------------------------

### Dictionary Methods

Source: https://typst.app/docs/reference/foundations/dictionary

Details the available methods for dictionaries, including `len`, `at`, `insert`, `remove`, `keys`, `values`, and `pairs`.

```APIDOC
## Dictionary Methods

### `len`

#### Description
The number of pairs in the dictionary.

#### Method
`self.len()`

#### Endpoint
N/A

#### Parameters
N/A

#### Request Example
```ty
#let dict = (a: 1, b: 2)
#dict.len()
```

#### Response
- **int** - The number of pairs.

#### Response Example
```ty
2
```

### `at`

#### Description
Returns the value associated with the specified key in the dictionary. May be used on the left-hand side of an assignment if the key is already present in the dictionary. Returns the default value if the key is not part of the dictionary or fails with an error if no default value was specified.

#### Method
`self.at(key, default: none)`

#### Endpoint
N/A

#### Parameters
- **key** (str) - Required - The key at which to retrieve the item.
- **default** (any) - Optional - A default value to return if the key is not part of the dictionary.

#### Request Example
```ty
#let dict = (a: 1)
#dict.at("a")
#dict.at("b", default: 0)
```

#### Response
- **any** - The value associated with the key, or the default value.

#### Response Example
```ty
1
0
```

### `insert`

#### Description
Inserts a new pair into the dictionary. If the dictionary already contains this key, the value is updated. To insert multiple pairs at once, you can just alternatively another dictionary with the `+=` operator.

#### Method
`self.insert(key, value)`

#### Endpoint
N/A

#### Parameters
- **key** (str) - Required - The key of the pair that should be inserted.
- **value** (any) - Required - The value of the pair that should be inserted.

#### Request Example
```ty
#let dict = (a: 1)
#dict.insert("b", 2)
#dict.insert("a", 10)
```

#### Response
N/A (Modifies the dictionary in place)

#### Response Example
N/A

### `remove`

#### Description
Removes a pair from the dictionary by key and returns the value.

#### Method
`self.remove(key, default: none)`

#### Endpoint
N/A

#### Parameters
- **key** (str) - Required - The key of the pair to remove.
- **default** (any) - Optional - A default value to return if the key does not exist.

#### Request Example
```ty
#let dict = (a: 1, b: 2)
#dict.remove("a")
#dict.remove("c", default: 0)
```

#### Response
- **any** - The value of the removed pair, or the default value.

#### Response Example
```ty
1
0
```

### `keys`

#### Description
Returns the keys of the dictionary as an array in insertion order.

#### Method
`self.keys()`

#### Endpoint
N/A

#### Parameters
N/A

#### Request Example
```ty
#let dict = (a: 1, b: 2)
#dict.keys()
```

#### Response
- **array** - An array of the dictionary's keys.

#### Response Example
```ty
[a, b]
```

### `values`

#### Description
Returns the values of the dictionary as an array in insertion order.

#### Method
`self.values()`

#### Endpoint
N/A

#### Parameters
N/A

#### Request Example
```ty
#let dict = (a: 1, b: 2)
#dict.values()
```

#### Response
- **array** - An array of the dictionary's values.

#### Response Example
```ty
[1, 2]
```

### `pairs`

#### Description
Returns the keys and values of the dictionary as an array of pairs. Each pair is represented as an array of length two.

#### Method
`self.pairs()`

#### Endpoint
N/A

#### Parameters
N/A

#### Request Example
```ty
#let dict = (a: 1, b: 2)
#dict.pairs()
```

#### Response
- **array** - An array of key-value pairs.

#### Response Example
```ty
[[a, 1], [b, 2]]
```
```

--------------------------------

### Create an HTML dialog element

Source: https://typst.app/docs/reference/html/typed

Use this function to create an HTML `<dialog>` element, representing a dialog box or window. It accepts an `open` boolean parameter and content.

```typst
html.dialog(
open: bool,content,
)
```

--------------------------------

### Create a Linear Gradient

Source: https://typst.app/docs/reference/visualize/gradient

Use gradient.linear to create a color transition along a straight line. It accepts color stops and optional parameters for color space, relative placement, direction, and angle.

```typst
#rect(
  width: 100%,
  height: 20pt,
  fill: gradient.linear(
    ..color.map.viridis,
  ),
)

```

--------------------------------

### Typst Scripting: Euclidean Division and Remainder

Source: https://typst.app/docs/changelog/0.10.0

Introduces `calc.div-euclid` and `calc.rem-euclid` functions for performing Euclidean division and calculating the remainder, ensuring consistent behavior with negative numbers.

```typst
calc.div-euclid
```

```typst
calc.rem-euclid
```

--------------------------------

### Scripting: Color Methods

Source: https://typst.app/docs/changelog/0.7.0

A suite of new methods for colors: `kind`, `hex`, `rgba`, `cmyk`, and `luma`. These allow for easy inspection and conversion between color spaces.

```typst
let c = rgb(255, 0, 0)
let hex_code = c.hex()
let rgba_values = c.rgba()
let cmyk_values = c.cmyk()
let luminance = c.luma()
```

--------------------------------

### Create Sharp Gradient

Source: https://typst.app/docs/reference/visualize/gradient

Creates a sharp version of a gradient with discrete jumps between colors. Use `sharp(steps)` to define the number of color stops and `smoothness` to control the smoothing.

```typst
#set rect(width: 100%, height: 20pt)
#let grad = gradient.linear(..color.map.rainbow)
#rect(fill: grad)
#rect(fill: grad.sharp(5))
#rect(fill: grad.sharp(5, smoothness: 20%))
```

--------------------------------

### Create an Oklab Color Square

Source: https://typst.app/docs/reference/visualize/color

Illustrates creating a square filled with a color defined in the Oklab color space, specifying lightness, a, b, and alpha components.

```typst
#square(
  fill: oklab(27%, 20%, -3%, 50%)
)


```

--------------------------------

### Grid Styling and Accessibility

Source: https://typst.app/docs/reference/layout/grid

Explains stroke precedence in grid cells and discusses accessibility limitations for grids, recommending tables for tabular data.

```APIDOC
## Stroke Styling Precedence

There are three ways to set the stroke of a grid cell: `grid.cell`'s `stroke` field, `grid.hline` and `grid.vline`, or the `grid`'s `stroke` field. The precedence order from highest to lowest is: `hline`/`vline` settings, `cell` settings, and `grid` settings. Strokes of a repeated grid header or footer take precedence over regular cell strokes.

## Accessibility

Grids do not carry special semantics for Assistive Technology (AT). AT cannot navigate grids two-dimensionally by cell. For tabular data, use the `table` element instead.

AT reads grid cells in their semantic order, typically the order they are passed to the grid. If `grid.cell`'s `x` and `y` arguments are used for manual positioning, cells are read row by row, left to right (in LTR documents). A cell is read when its position is first reached.
```

--------------------------------

### Typst Math: Variables and Literal Strings

Source: https://typst.app/docs/reference/math

Demonstrates how Typst handles single letters as is, multiple letters as variables/functions, and how to display literal strings or access variables within math mode using quotes and hash syntax.

```typst
$ A = pi r^2 $
$ "area" = pi dot "radius"^2 $
$ cal(A) := 
    { x in RR | x "is natural" } $
#let x = 5
$ #x < 17 $
```

--------------------------------

### Create New Empty Document in Typst

Source: https://typst.app/docs/guides/for-latex-users

Demonstrates how to create a new, empty Typst document. Typst requires no boilerplate, and paragraph breaks are indicated by blank lines, similar to LaTeX. The output is rendered on an A4 page.

```typst
Hey there!

Here are two paragraphs. The
output is shown to the right.


```

--------------------------------

### Page Break with 'to' Parameter

Source: https://typst.app/docs/reference/layout/pagebreak

Illustrates using the 'to' parameter to force the next page to be odd or even. An empty page may be inserted if necessary. The page height is set to 30pt for demonstration.

```typst
#set page(height: 30pt)

First.
#pagebreak(to: "odd")
Third.
```

--------------------------------

### Create a timed text track

Source: https://typst.app/docs/reference/html/typed

Use the `track` function to add a timed text track to media. It supports attributes like `default`, `kind`, `label`, `src`, and `srclang`.

```typst
html.track(default: bool, kind: str, label: str, src: str, srclang: str)
```

--------------------------------

### Create an HTML data element

Source: https://typst.app/docs/reference/html/typed

Use this function to create an HTML `<data>` element, which provides a machine-readable equivalent for its content. It requires a `value` string and content.

```typst
html.data(
value: str,content,
)
```

--------------------------------

### Array `join` Method

Source: https://typst.app/docs/reference/foundations/array

Combines all items in the array into one, with optional separators.

```APIDOC
## `join`

### Description
Combine all items in the array into one.

### Method
`self.join(separator: any, last: any, default: any) -> any`

### Parameters
#### Path Parameters
- **separator** (any or none) - Positional - A value to insert between each item of the array. Default: `none`
- **last** (any) - An alternative separator between the last two items.
- **default** (any or none) - What to return if the array is empty. Default: `none`
```

--------------------------------

### Text Script Configuration

Source: https://typst.app/docs/reference/text/text

Configure the writing script for text rendering. This, along with language, influences font feature implementation. 'auto' is the recommended default.

```APIDOC
## SET TEXT SCRIPT

### Description
Configures the OpenType writing script for text. This parameter, in conjunction with `lang`, determines how font features are applied. The default is `auto`, which selects an appropriate script based on Unicode properties.

### Method
SET

### Parameters
#### Settable Parameters
- **script** (auto or str) - Settable - The OpenType writing script. Can be an ISO 15924 identifier or `auto`.

### Request Example
```ty
#set text(script: "latn")
```

### Response
This is a settable parameter and does not have a direct response in the traditional sense. Changes are applied to subsequent text rendering.
```

--------------------------------

### Basic `terms` Element Usage

Source: https://typst.app/docs/reference/model/terms

Displays a sequence of terms and their descriptions vertically. Descriptions use hanging indent for visual hierarchy. Use this for simple term-description lists.

```typst
/ Ligature: A merged glyph.
/ Kerning: A spacing adjustment
  between two adjacent letters.

```

--------------------------------

### Stack with different directions

Source: https://typst.app/docs/reference/layout/direction

Demonstrates using the 'direction' type with the 'stack' function. Values can be specified globally or via the 'direction' scope.

```typst
#stack(dir: rtl)[A][B][C]
#stack(dir: direction.rtl)[A][B][C]
```

--------------------------------

### Show Rules with String and Regex Selectors

Source: https://typst.app/docs/reference/styling

Demonstrates using show rules with literal string and regular expression selectors. The first rule applies 'smallcaps' to the exact string 'Project', and the second replaces the string 'badly' with 'great'.

```typst
#show "Project": smallcaps
#show "badly": "great"

We started Project in 2019
and are still working on it.
Project is progressing badly.

```

--------------------------------

### Create an HTML datalist element

Source: https://typst.app/docs/reference/html/typed

Use this function to create an HTML `<datalist>` element, which contains options for a combo box control. It takes content as a positional parameter.

```typst
html.datalist(
content
)
```

--------------------------------

### Create an HTML code element

Source: https://typst.app/docs/reference/html/typed

Use this function to create an HTML `<code>` element. It takes content as a positional parameter.

```typst
html.code(
content
)
```

--------------------------------

### Customizing `underline` Stroke and Offset

Source: https://typst.app/docs/reference/text/underline

Shows how to customize the underline's stroke color, thickness, and offset from the baseline. Requires specifying stroke and offset parameters.

```typst
Take #underline(
  stroke: 1.5pt + red,
  offset: 2pt,
  [care],
)
```

--------------------------------

### Float Definitions: is-nan, is-infinite, signum

Source: https://typst.app/docs/reference/foundations/float

Explains and demonstrates the utility functions for checking float properties and calculating their sign.

```APIDOC
### `is-nan`
Checks if a float is not a number.
```
#float.is-nan(0) \
#float.is-nan(1) \
#float.is-nan(float.nan)

```

### `is-nan(self) -> bool`

### `is-infinite`
Checks if a float is infinite.
```
#float.is-infinite(0) \
#float.is-infinite(1) \
#float.is-infinite(float.inf)

```

### `is-infinite(self) -> bool`

### `signum`
Calculates the sign of a floating point number.
* If the number is positive (including `+0.0`), returns `1.0`.
* If the number is negative (including `-0.0`), returns `-1.0`.
* If the number is NaN, returns `float.nan`.

```
#(5.0).signum() \
#(-5.0).signum() \
#(0.0).signum() \
#float.nan.signum()

```

### `signum(self) -> float`
```

--------------------------------

### Create an Article Element

Source: https://typst.app/docs/reference/html/typed

Use the `html.article` function to create a self-contained composition element. It accepts content, which forms the body of the article.

```typst
html.article(
  content
) -> content
```

--------------------------------

### Integer Type Overview

Source: https://typst.app/docs/reference/foundations/int

Explains the properties of Typst integers, including their range, representation, and how they are parsed.

```APIDOC
## Integer Type
A whole number. The number can be negative, zero, or positive. As Typst uses 64 bits to store integers, integers cannot be smaller than `-9223372036854775808` or larger than `9223372036854775807`. Integer literals are always positive, so a negative integer such as `-1` is semantically the negation `-` of the positive literal `1`. A positive integer greater than the maximum value and a negative integer less than or equal to the minimum value cannot be represented as an integer literal, and are instead parsed as a `float`. The minimum integer value can still be obtained through integer arithmetic.

The number can also be specified as hexadecimal, octal, or binary by starting it with a zero followed by either `x`, `o`, or `b`.

You can convert a value to an integer with this type's constructor.

### Example
```
#(1 + 2) 
#(2 - 5) 
#(3 + 4 < 8)

#0xff 
#0o10 
#0b1001
```
```

--------------------------------

### Typst `strong` Element Syntax and Default Delta

Source: https://typst.app/docs/reference/model/strong

Illustrates the default behavior of the `strong` element when no delta is specified, showing that a delta of 0 has no effect.

```typst
#set strong(delta: 0)
No *effect!*
```

--------------------------------

### Basic Skew Element Usage

Source: https://typst.app/docs/reference/layout/skew

Demonstrates the basic application of the `skew` element to create a skewed text effect.

```typst
#skew(ax: -12deg)[
  This is some fake italic text.
]

```

--------------------------------

### Cite Element Syntax and Parameters

Source: https://typst.app/docs/reference/model/cite

Details the syntax and parameters of the `cite` function, including the required `label` and optional `supplement`, `form`, and `style` parameters.

```APIDOC
## Syntax

This function indirectly has dedicated syntax. References can be used to cite works from the bibliography. The label then corresponds to the citation key.

## Parameters

Parameters are the inputs to a function. They are specified in parentheses after the function name.

cite(
label,
supplement: nonecontent,
form: nonestr,
style: autostrbytes,
) -> content

### `key`
label
Required Positional
Positional parameters are specified in order, without names. 
The citation key that identifies the entry in the bibliography that shall be cited, as a label.
```
// All the same
@netwok 
#cite(<netwok>) 
#cite(label("netwok"))
```

### `supplement`
none or content
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
A supplement for the citation such as page or chapter number.
In reference syntax, the supplement can be added in square brackets:
```
This has been proven. @distress[p.~7]

#bibliography("works.bib")
```

Default: `none`

### `form`
none or str
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
The kind of citation to produce. Different forms are useful in different scenarios: A normal citation is useful as a source at the end of a sentence, while a "prose" citation is more suitable for inclusion in the flow of text.
If set to `none`, the cited work is included in the bibliography, but nothing will be displayed.
```
#cite(<netwok>, form: "prose")
show the outsized effects of
pirate life on the human psyche.
```

Variant| Details  
---|---
`"normal"`| Display in the standard way for the active style.  
`"prose"`| Produces a citation that is suitable for inclusion in a sentence.  
`"full"`| Mimics a bibliography entry, with full information about the cited work.  
`"author"`| Shows only the cited work's author(s).  
`"year"`| Shows only the cited work's year.  
Default: `"normal"`
```

--------------------------------

### Enable Fractions with `fractions: true`

Source: https://typst.app/docs/reference/text/text

Use `fractions: true` to enable the OpenType `frac` font feature for specific text. Avoid enabling this globally as it can affect numbers in URLs and other contexts.

```typst
1/2 
#text(fractions: true)[1/2]
```

--------------------------------

### Calculate floored quotient with `calc.quo`

Source: https://typst.app/docs/reference/foundations/calc

Calculates the quotient by flooring the result of the division. This function always returns an integer and may error if the result exceeds 64-bit integer limits.

```typst
$ "quo"(a, b) &= floor(a/b) 
  "quo"(14, 5) &= #calc.quo(14, 5) 
  "quo"(3.46, 0.5) &= #calc.quo(3.46, 0.5) $
```

--------------------------------

### Set Text Weight

Source: https://typst.app/docs/reference/text/text

Shows how to set the font weight using string names or integer values. Typst selects the closest available weight if the exact one is not found.

```typst
#set text(font: "IBM Plex Sans")

#text(weight: "light")[Light] \
#text(weight: "regular")[Regular] \
#text(weight: "medium")[Medium] \
#text(weight: 500)[Medium] \
#text(weight: "bold")[Bold]
```

--------------------------------

### Scripting: Bytes to String Conversion

Source: https://typst.app/docs/changelog/0.7.0

The `str` function can now convert byte buffers to strings, enabling easy handling of text data stored as bytes.

```typst
let byte_data = bytes([84, 121, 112, 115, 116])
let text = str(byte_data)
```

--------------------------------

### Create an HTML definition term element

Source: https://typst.app/docs/reference/html/typed

Use this function to create an HTML `<dfn>` element, representing the defining instance of a term. It takes content as a positional parameter.

```typst
html.dfn(
content
)
```

--------------------------------

### Create an HTML details element

Source: https://typst.app/docs/reference/html/typed

Use this function to create an HTML `<details>` element, which acts as a disclosure control for hiding details. It accepts optional `name` and `open` parameters, along with content.

```typst
html.details(
name: str,open: bool,content,
)
```

--------------------------------

### Fractional Spacing with h Element

Source: https://typst.app/docs/reference/layout/h

Illustrates using fractional spacing (fr units) with the h element for flexible alignment within a line. This method distributes available space proportionally among elements marked with fractions.

```typst
First #h(1fr) Second \
First #h(1fr) Second #h(1fr) Third \
First #h(2fr) Second #h(1fr) Third
```

--------------------------------

### Create an HTML definition description element

Source: https://typst.app/docs/reference/html/typed

Use this function to create an HTML `<dd>` element, which provides content for a corresponding `<dt>` element. It takes content as a positional parameter.

```typst
html.dd(
content
)
```

--------------------------------

### Importing and Using a Private Typst Package

Source: https://typst.app/docs/web-app/private-packages

Demonstrates how to import a private Typst package, either entirely or specific items, and then use its functions. This is crucial for code reuse across projects.

```typst
#import "@local/package-demo:0.1.0"
#package-demo.a-function()

// ... or only specific items.
#import "@local/package-demo:0.1.0": a-function
#a-function()
```

--------------------------------

### Typst: Label Syntax for Headings

Source: https://typst.app/docs/reference/foundations/label

Illustrates the dedicated syntax for creating labels within Typst markup, specifically showing how labels attach to different elements within headings based on their placement.

```typst
// Equivalent to `#heading[Introduction] <a>`.
= Introduction <a>

// Equivalent to `#heading[Conclusion #label("b")]`.
= Conclusion #label("b")

```

--------------------------------

### Draw a quadratic Bézier curve with control point

Source: https://typst.app/docs/reference/visualize/curve

Illustrates drawing a quadratic Bézier curve using `curve.quad`. The `control` point influences the curve's shape. The `mark` function is a helper to visualize the control point.

```typst
// Function to illustrate where the control point is.
#let mark((x, y)) = place(
  dx: x - 1pt, dy: y - 1pt,
  circle(fill: aqua, radius: 2pt),
)

#mark((20pt, 20pt))

#curve(
  stroke: blue,
  curve.move((0pt, 100pt)),
  curve.quad((20pt, 20pt), (100pt, 0pt)),
)

```

--------------------------------

### Load and Modify Image Bytes

Source: https://typst.app/docs/reference/visualize/image

Read an image file, modify its content (e.g., replace colors), and then display the original and modified versions using the `image` element with raw bytes.

```typst
#let original = read("diagram.svg")
#let changed = original.replace(
  "#2B80FF", // blue
  green.to-hex(),
)

#image(bytes(original))
#image(bytes(changed))

```

--------------------------------

### Typst Code and Verbatim Text

Source: https://typst.app/docs/guides/for-latex-users

Illustrates how to display code or verbatim text in Typst using the `raw` function or syntax, and how to use the `text` function for monospace formatted text.

```typst
`print(f"{x}")`
``#typst-code()``
#text(font: "DejaVu Sans Mono", size: 0.8em)[
  monospace *bold*
]
```

--------------------------------

### Skew with Reflow Enabled

Source: https://typst.app/docs/reference/layout/skew

Shows how to enable layout adjustments for skewed content by setting `reflow: true`. This makes the bounding box account for the skew transformation.

```typst
Hello #skew(ay: 30deg, reflow: true, "World")!

```

--------------------------------

### Scripting: Angle Unit Methods

Source: https://typst.app/docs/changelog/0.7.0

The `deg` and `rad` methods can now be applied to angles, simplifying conversions between degrees and radians.

```typst
let angle_in_degrees = 90.deg
```

```typst
let angle_in_radians = pi / 2.rad
```

--------------------------------

### Typst: Style Abstract with Ragged and Centered Text

Source: https://typst.app/docs/tutorial/advanced-styling

Shows how to format an abstract to be ragged and centered using the `#align` function. It also demonstrates scoping styling rules within a content block using `#set par(justify: false)`.

```typst
#align(center)[
  #set par(justify: false)
  *Abstract* \ 
  #lorem(80)
]

```

--------------------------------

### Page Element Parameters

Source: https://typst.app/docs/reference/layout/page

Detailed explanation of the parameters available for the `page` element, including their types, descriptions, and default values.

```APIDOC
## Parameters

Parameters are the inputs to a function. They are specified in parentheses after the function name.

page(
  paper: str,
  width: autolength,
  height: autolength,
  flipped: bool,
  margin: autorelativedictionary,
  binding: autoalignment,
  columns: int,
  fill: noneautocolorgradienttiling,
  numbering: nonestrfunction,
  supplement: noneautocontent,
  number-align: alignment,
  header: noneautocontent,
  header-ascent: relative,
  footer: noneautocontent,
  footer-descent: relative,
  background: nonecontent,
  foreground: nonecontent,
  body: content,
) -> content

### `paper`
str
A standard paper size to set width and height.
This is just a shorthand for setting `width` and `height` and, as such, cannot be retrieved in a context expression.
Default: `"a4"`
`"a0"` , `"a1"` , `"a2"` , `"a3"` , `"a4"` , `"a5"` , `"a6"` , `"a7"` , `"a8"` , `"a9"` , `"a10"` , `"a11"` , `"iso-b1"` , `"iso-b2"` , `"iso-b3"` , `"iso-b4"` , `"iso-b5"` , `"iso-b6"` , `"iso-b7"` , `"iso-b8"` , `"iso-c3"` , `"iso-c4"` , `"iso-c5"` , `"iso-c6"` , `"iso-c7"` , `"iso-c8"` , `"din-d3"` , `"din-d4"` , `"din-d5"` , `"din-d6"` , `"din-d7"` , `"din-d8"` , `"sis-g5"` , `"sis-e5"` , `"ansi-a"` , `"ansi-b"` , `"ansi-c"` , `"ansi-d"` , `"ansi-e"` , `"arch-a"` , `"arch-b"` , `"arch-c"` , `"arch-d"` , `"arch-e1"` , `"arch-e"` , `"jis-b0"` , `"jis-b1"` , `"jis-b2"` , `"jis-b3"` , `"jis-b4"` , `"jis-b5"` , `"jis-b6"` , `"jis-b7"` , `"jis-b8"` , `"jis-b9"` , `"jis-b10"` , `"jis-b11"` , `"sac-d0"` , `"sac-d1"` , `"sac-d2"` , `"sac-d3"` , `"sac-d4"` , `"sac-d5"` , `"sac-d6"` , `"iso-id-1"` , `"iso-id-2"` , `"iso-id-3"` , `"asia-f4"` , `"jp-shiroku-ban-4"` , `"jp-shiroku-ban-5"` , `"jp-shiroku-ban-6"` , `"jp-kiku-4"` , `"jp-kiku-5"` , `"jp-business-card"` , `"cn-business-card"` , `"eu-business-card"` , `"fr-tellière"` , `"fr-couronne-écriture"` , `"fr-couronne-édition"` , `"fr-raisin"` , `"fr-carré"` , `"fr-jésus"` , `"uk-brief"` , `"uk-draft"` , `"uk-foolscap"` , `"uk-quarto"` , `"uk-crown"` , `"uk-book-a"` , `"uk-book-b"` , `"us-letter"` , `"us-legal"` , `"us-tabloid"` , `"us-executive"` , `"us-foolscap-folio"` , `"us-statement"` , `"us-ledger"` , `"us-oficio"` , `"us-gov-letter"` , `"us-gov-legal"` , `"us-business-card"` , `"us-digest"` , `"us-trade"` , `"newspaper-compact"` , `"newspaper-berliner"` , `"newspaper-broadsheet"` , `"presentation-16-9"` , `"presentation-4-3"`

### `width`
auto or length
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
The width of the page.
```typ
#set page(
  width: 3cm,
  margin: (x: 0cm),
)

#for i in range(3) {
  box(square(width: 1cm))
}
```

Default: `595.28pt`

### `height`
auto or length
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
The height of the page.
If this is set to `auto`, page breaks can only be triggered manually by inserting a page break or by adding another non-empty page set rule. Most examples throughout this documentation use `auto` for the height of the page to dynamically grow and shrink to fit their content.
Default: `841.89pt`

### `flipped`
bool
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
Whether the page is flipped into landscape orientation.
```typ
#set page(
  "us-business-card",
  flipped: true,
  fill: rgb("f2e5dd"),
)

#set align(bottom + end)
#text(14pt)[*Sam H. Richards*] \ 
_Procurement Manager_

#set text(10pt)
17 Main Street \ 
New York, NY 10001 \ 
+1 555 555 5555
```

Default: `false`
```

--------------------------------

### Accent Element Usage

Source: https://typst.app/docs/reference/math/accent

Demonstrates basic usage of the accent element with different accents.

```APIDOC
## `accent` Element

Element functions can be customized with `set` and `show` rules. Attaches an accent to a base.

### Example
```typ
$grave(a) = accent(a, `)$
$arrow(a) = accent(a, arrow)$
$tilde(a) = accent(a, ~)$
```
```

--------------------------------

### Typst Heading Structure (Correct)

Source: https://typst.app/docs/guides/accessibility

Demonstrates the correct sequential usage of headings in Typst. Headings must follow a strict hierarchical order, never skipping levels when descending.

```typst
= First level heading
== Second level heading
=== Third level heading
```

--------------------------------

### Create an rt Element

Source: https://typst.app/docs/reference/html/typed

Use html.rt to define ruby annotation text. It takes content as its only parameter.

```typst
html.rt(content,)
```

--------------------------------

### Create a picture Element

Source: https://typst.app/docs/reference/html/typed

Use html.picture to embed an image. It takes content as its only parameter.

```typst
html.picture(content,)
```

--------------------------------

### Defining and Using Custom Counters

Source: https://typst.app/docs/reference/introspection/counter

Explains how to define a custom counter using `counter("key")` and then use its `.display()`, `.step()`, and `.update()` methods. Custom counters are identified by a string key.

```typst
#let mine = counter("mycounter")
#context mine.display() 
#mine.step()
#context mine.display() 
#mine.update(c => c * 3)
#context mine.display()
```

--------------------------------

### Cite Element Usage

Source: https://typst.app/docs/reference/model/cite

Demonstrates various ways to use the `cite` element for referencing bibliography entries, including direct syntax and explicit function calls.

```APIDOC
## `cite` Element

Element functions can be customized with `set` and `show` rules. Cite a work from the bibliography. Before you starting citing, you need to add a bibliography somewhere in your document.

### Example
```
This was already noted by
pirates long ago. @arrgh

Multiple sources say ...
@arrgh @netwok.

You can also call `cite`
explicitly. #cite(<arrgh>)

#bibliography("works.bib")
```

If your source name contains certain characters such as slashes, which are not recognized by the `<>` syntax, you can explicitly call `label` instead.
```
Computer Modern is an example of a modernist serif typeface.
#cite(label("DBLP:books/lib/Knuth86a")).
```
```

--------------------------------

### Apply Tiling to Text

Source: https://typst.app/docs/reference/visualize/tiling

Applies a colorful gradient tiling pattern to text. Ensure the `relative` parameter is set to 'parent' or 'auto' when tiling text for correct rendering.

```typst
#let pat = tiling(
  size: (30pt, 30pt),
  relative: "parent",
  square(
    size: 30pt,
    fill: gradient
      .conic(..color.map.rainbow),
  )
)

#set text(fill: pat)
#lorem(10)


```

--------------------------------

### Create a progress Element

Source: https://typst.app/docs/reference/html/typed

Use html.progress to create a progress bar. It accepts 'max' and 'value' attributes to define the range and current progress.

```typst
html.progress(max: float,value: float,content,)
```

--------------------------------

### Create sliding windows

Source: https://typst.app/docs/reference/foundations/array

Generates sliding windows of a specified size over an array. If the array length is less than the window size, an empty array is returned.

```typst
#let array = (1, 2, 3, 4, 5, 6, 7, 8)
#array.windows(5)
```

--------------------------------

### Generate Default Table of Contents

Source: https://typst.app/docs/reference/model/outline

Use `outline()` to generate a standard table of contents for headings. Ensure headings are numbered using `set heading(numbering: "1.")` for proper display.

```typst
#set heading(numbering: "1.")
#outline()

= Introduction
#lorem(5)

= Methods
== Setup
#lorem(10)
```

--------------------------------

### Define an image or media source

Source: https://typst.app/docs/reference/html/typed

Use `html.source` to specify the source for an `img`, `video`, or `audio` element. It supports attributes like `height`, `media`, `sizes`, `src`, `srcset`, `type`, and `width`.

```typst
html.source(height: int,media: str,sizes: array,src: str,srcset: array,type: str,width: int)
```

--------------------------------

### Typst Stroke Constructor Usage

Source: https://typst.app/docs/reference/visualize/stroke

Shows how to use the stroke constructor to convert values to strokes or define strokes with specific parameters. Useful for ensuring a value has all stroke fields.

```typst
#let my-func(x) = {
    x = stroke(x) // Convert to a stroke
    [Stroke has thickness #x.thickness.]
}
#my-func(3pt) \
#my-func(red) \
#my-func(stroke(cap: "round", thickness: 1pt))

```

--------------------------------

### Draw quadratic Bézier curves with relative coordinates

Source: https://typst.app/docs/reference/visualize/curve

Demonstrates drawing quadratic Bézier curves where `control` and `end` points are specified relative to the previous point using `relative: true`.

```typst
#curve(
  stroke: 2pt,
  curve.quad((20pt, 40pt), (40pt, 40pt), relative: true),
  curve.quad(auto, (40pt, -40pt), relative: true),
)

```

--------------------------------

### Importing a Template from Typst Universe

Source: https://typst.app/docs/guides/for-latex-users

This code illustrates how to import a template directly from Typst Universe, Typst's package repository. Templates from Typst Universe are automatically downloaded upon first use. The specific template function name (e.g., 'elsearticle') depends on the template's documentation.

```typst
#import "@preview/elsearticle:0.2.1": elsearticle
```

--------------------------------

### Ref Element Parameters

Source: https://typst.app/docs/reference/model/ref

Details the parameters available for the `ref` function, including `label`, `supplement`, and `form`, with explanations for each.

```APIDOC
## Parameters
Parameters are the inputs to a function. They are specified in parentheses after the function name.
ref(
label,supplement: noneautocontentfunction,form: str,
) -> content
### `target`
label
Required Positional
Positional parameters are specified in order, without names. 
The target label that should be referenced.
Can be a label that is defined in the document or, if the `form` is set to `"normal"`, an entry from the `bibliography`.
### `supplement`
none or auto or content or function
Settable
Settable parameters can be customized for all following uses of the function with a `set` rule. 
A supplement for the reference.
If the `form` is set to `"normal"`:
  * For references to headings or figures, this is added before the referenced number.
  * For citations, this can be used to add a page number.


If the `form` is set to `"page"`, then this is added before the page number of the label referenced.
If a function is specified, it is passed the referenced element and should return content.
```
#set heading(numbering: "1.")
#show ref.where(
  form: "normal"
): set ref(supplement: it => {
  if it.func() == heading {
    "Chapter"
  } else {
    "Thing"
  }
})

= Introduction <intro>
In @intro, we see how to turn
Sections into Chapters. And
in @intro[Part], it is done
manually.

```

Default: `auto`
```

--------------------------------

### Raw Element Parameters

Source: https://typst.app/docs/reference/text/raw

Configuration options for the `raw` element that can be set using `#set raw(...)`.

```APIDOC
## `lang`

none or str
Settable

The language to syntax-highlight in. Supports standard language tags, plus `"typ"`, `"typc"`, and `"typm"` for Typst markup, code, and math.

Default: `none`

## `align`

alignment
Settable

The horizontal alignment for each line within a raw block. Ignored if not a raw block (e.g., `block: false` or single backticks).

Default: `start`

## `syntaxes`

str or bytes or array
Settable

Additional syntax definitions to load in `sublime-syntax` format. Can be a path string, raw bytes, or an array of these.

Default: `()`

## `theme`

none or auto or str or bytes
Settable

The theme to use for syntax highlighting in `tmTheme` file format. Options include `none`, `auto`, a path string, or raw bytes.

Default: `auto`

## `tab-size`

int
Settable

The size for a tab stop in spaces. Tabs are replaced with spaces to align with the next multiple of this size.

Default: `2`
```

--------------------------------

### Scripting: Length Unit Methods

Source: https://typst.app/docs/changelog/0.7.0

New methods `pt`, `mm`, `cm`, and `inches` are available for lengths, providing convenient ways to convert between different units.

```typst
let length_in_mm = 100.mm
```

```typst
let length_in_inches = 5.inches
```

--------------------------------

### Typst Bindings: Variable Assignment and Function Definition

Source: https://typst.app/docs/reference/scripting

Demonstrates defining variables with 'let' bindings and creating custom named functions. Variables can be accessed within their containing block. Function arguments can also be destructured.

```typst
#let name = "Typst"
This is #name's documentation.
It explains #name.

#let my-add(x, y) = x + y
Sum is #my-add(2, 3).

```

--------------------------------

### Create a Regular Polygon

Source: https://typst.app/docs/reference/visualize/polygon

Use the `polygon.regular` function to create a regular polygon with equal sides and angles. Specify the size and the number of vertices. Custom fill and stroke can also be applied.

```typst
#polygon.regular(
  fill: blue.lighten(80%),
  stroke: blue,
  size: 30pt,
  vertices: 3,
)

```

--------------------------------

### Custom Quote Styles with Smartquote

Source: https://typst.app/docs/reference/text/smartquote

Demonstrates how to define custom quotation marks using the `quotes` parameter. You can specify custom double quotes as a string or an array, or configure both single and double quotes using a dictionary.

```typst
#set text(lang: "de")
'Das sind normale Anführungszeichen.'

#set smartquote(quotes: "()")
"Das sind eigene Anführungszeichen."

#set smartquote(quotes: (single: ("||", "||"),  double: auto))
```

--------------------------------

### Float Byte Conversions: from-bytes, to-bytes

Source: https://typst.app/docs/reference/foundations/float

Details the methods for converting between float values and their byte representations.

```APIDOC
### `from-bytes`
Interprets bytes as a float.
```
#float.from-bytes(bytes((0, 0, 0, 0, 0, 0, 240, 63))) \
#float.from-bytes(bytes((63, 240, 0, 0, 0, 0, 0, 0)), endian: "big")

```

### `from-bytes(bytes, endian: str) -> float`
#### `bytes`
bytes
Required Positional
The bytes that should be converted to a float.
Must have a length of either 4 or 8. The bytes are then interpreted in IEEE 754's binary32 (single-precision) or binary64 (double-precision) format depending on the length of the bytes.
#### `endian`
str
The endianness of the conversion.
Variant| Details  
---|---
`"big"`| Big-endian byte order: The highest-value byte is at the beginning of the bytes.  
`"little"`| Little-endian byte order: The lowest-value byte is at the beginning of the bytes.  
Default: `"little"`

### `to-bytes`
Converts a float to bytes.
```
#array(1.0.to-bytes(endian: "big")) \
#array(1.0.to-bytes())

```

### `to-bytes(endian: str, size: int) -> bytes`
#### `endian`
str
The endianness of the conversion.
Variant| Details  
---|---
`"big"`| Big-endian byte order: The highest-value byte is at the beginning of the bytes.  
`"little"`| Little-endian byte order: The lowest-value byte is at the beginning of the bytes.  
Default: `"little"`
#### `size`
int
The size of the resulting bytes.
This must be either 4 or 8. The call will return the representation of this float in either IEEE 754's binary32 (single-precision) or binary64 (double-precision) format depending on the provided size.
Default: `8`
```

--------------------------------

### Typst Markup Mode Syntax

Source: https://typst.app/docs/reference/syntax

Illustrates basic markup syntax in Typst. This includes elements for paragraph breaks, emphasis, links, labels, references, headings, lists, and more. These are shortcuts for corresponding functions.

```typst
parbreak
```

```typst
*strong*
```

```typst
_emphasis_
```

```typst
`print(1)`
```

```typst
https://typst.app/
```

```typst
<intro>
```

```typst
@intro
```

```typst
= Heading
```

```typst
- item
```

```typst
+ item
```

```typst
/ Term: description
```

```typst
`\`
```

```typst
'single' or "double"
```

```typst
~
```

```typst
---
```

```typst
#rect(width: 1cm)
```

```typst
Tweet at us \#ad
```

```typst
/* block */
```

```typst
// line
```

--------------------------------

### math.display

Source: https://typst.app/docs/reference/math/sizes

Forces display style in math, the normal size for block equations.

```APIDOC
## math.display

### Description
Forced display style in math. This is the normal size for block equations.

### Signature
`math.display(content, cramped: bool)`

### Parameters
#### `body`
- **content** (content) - Required Positional - The content to size.
#### `cramped`
- **cramped** (bool) - Optional - Whether to impose a height restriction for exponents, like regular sub- and superscripts do. Default: `false`

### Request Example
```json
{
  "content": "sum_i x_i/2",
  "cramped": false
}
```

### Response
#### Success Response (200)
- **content** (content) - The sized content.

#### Response Example
```json
{
  "content": "display(sum_i x_i/2)"
}
```
```

--------------------------------

### Evaluate with Different Modes

Source: https://typst.app/docs/reference/foundations/eval

Shows how to use `eval` with specific syntactical modes: 'markup' for evaluating as markup content and 'math' for evaluating mathematical expressions.

```typst
#eval("= Heading", mode: "markup")
#eval("1_2^3", mode: "math")

```

--------------------------------

### Typst Scripting: Arguments Constructor

Source: https://typst.app/docs/changelog/0.10.0

A constructor for the `arguments` type has been added, simplifying the creation and manipulation of argument collections within Typst scripts.

```typst
arguments
```

--------------------------------

### Create Tiling with Spacing

Source: https://typst.app/docs/reference/visualize/tiling

Creates a tiling pattern with specified horizontal and vertical spacing between cells. This allows for gaps or overlaps in the pattern, depending on the spacing value relative to the tile size.

```typst
#let pat = tiling(
  size: (30pt, 30pt),
  spacing: (10pt, 10pt),
  relative: "parent",
  square(
    size: 30pt,
    fill: gradient
     .conic(..color.map.rainbow),
  ),
)

#rect(
  width: 100%,
  height: 60pt,
  fill: pat,
)


```

--------------------------------

### Basic Highlight Usage

Source: https://typst.app/docs/reference/text/highlight

Highlights text with a default background color. This is the simplest way to use the highlight function.

```typst
This is #highlight[important].
```

--------------------------------

### Full Numbering Display

Source: https://typst.app/docs/reference/model/enum

Illustrates using the `full` parameter to display the complete numbering, including parent enumeration numbers.

```typst
#set enum(numbering: "1.a)", full: true)
+ Cook
  + Heat water
  + Add ingredients
+ Eat
```

--------------------------------

### Enum Tightness Control

Source: https://typst.app/docs/reference/model/enum

Explains how to control the spacing between enum items using the `tight` parameter, influenced by blank lines between items.

```typst
+ If an enum has a lot of text, and
  maybe other inline content, it
  should not be tight anymore.

+ To make an enum wide, simply
  insert a blank line between the
  items.
```

--------------------------------

### Array Product Method

Source: https://typst.app/docs/reference/foundations/array

Calculates the product of all elements in an array.

```APIDOC
## `product`

### Description
Calculates the product of all items (works for all types that can be multiplied).

### Method
`self.product(default: any) -> any`

### Parameters
#### Path Parameters
None

#### Query Parameters
None

#### Request Body
None

### Request Example
None

### Response
#### Success Response (200)
- **any** - The product of the array elements.

#### Response Example
None
```

--------------------------------

### Typst Math: Function Calls

Source: https://typst.app/docs/reference/math

Demonstrates various math function calls in Typst, including fractions, vectors, matrices, and limits. It highlights the special argument handling within math calls and the use of semicolons for 2D argument lists.

```typst
$ frac(a^2, 2) $
$ vec(1, 2, delim: "[") $
$ mat(1, 2; 3, 4) $
$ mat(..#range(1, 5).chunks(2)) $
$ lim_x =
    op("lim", limits: #true)_x $
```

--------------------------------

### Typst CLI: Multiple Font Paths

Source: https://typst.app/docs/changelog/0.10.0

The `TYPST_FONT_PATHS` environment variable now supports multiple paths for font discovery. Paths should be separated by `;` on Windows and `:` on other operating systems.

```bash
TYPST_FONT_PATHS
```

--------------------------------

### Typst: Create Table of Contents with `query`

Source: https://typst.app/docs/reference/introspection/query

Demonstrates how to manually create a table of contents by querying for level 1 headings with the `outlined` property set to true within a `context`. It retrieves element locations, page numbers, and formats the output.

```typst
#set page(numbering: "1")

#heading(outlined: false)[
  Table of Contents
]
#context {
  let chapters = query(
    heading.where(
      level: 1,
      outlined: true,
    )
  )
  for chapter in chapters {
    let loc = chapter.location()
    let nr = numbering(
      loc.page-numbering(),
      ..counter(page).at(loc),
    )
    [#chapter.body #h(1fr) #nr \ ]
  }
}

= Introduction
#lorem(10)
#pagebreak()

== Sub-Heading
#lorem(8)

= Discussion
#lorem(18)


```

--------------------------------

### Create a variable element

Source: https://typst.app/docs/reference/html/typed

Use the `var` function to create an element representing a variable. It accepts content.

```typst
html.var(content)
```

--------------------------------

### Import Typst Package with Alias

Source: https://typst.app/docs/reference/scripting

Demonstrates how to import a specific function from a Typst package using an alias. This allows for cleaner code by referencing imported functions directly. Requires the package to be available in the specified namespace and version.

```typst
#import "@preview/example:0.1.0": add
#add(2, 7)

```

--------------------------------

### Popover Element

Source: https://typst.app/docs/reference/html/typed

Configuration options for making an element a popover.

```APIDOC
## Popover Element

### Description
Makes the element a popover element.

### Method
Not applicable (this is a configuration option, not an API endpoint).

### Endpoint
Not applicable.

### Parameters
#### Query Parameters
- **popover** (auto or str) - Required - Specifies how the popover should behave.

### Variants
- **`"manual"`**: Indicates a manually controlled popover.

```

--------------------------------

### Constructor Usage

Source: https://typst.app/docs/reference/foundations/arguments

Types with constructors can be called like functions to create new values. This includes constructing spreadable arguments in place, similar to `let args(..sink) = sink`.

```APIDOC
## Constructor

If a type has a constructor, you can call it like a function to create a new value of the type. Construct spreadable arguments in place. This function behaves like `let args(..sink) = sink`.

```ty
#let args = arguments(stroke: red, inset: 1em, [Body])
#box(..args)
```

arguments(
..any
) -> arguments
```

--------------------------------

### Automatically Style First Cells in Typst Tables

Source: https://typst.app/docs/guides/tables

Illustrates how to automatically apply styling to the first cell of each row in Typst tables using a `show` rule. This is a workaround for a current Typst limitation and applies strong emphasis to the first cells, useful for multiple tables in a document.

```typst
#show table.cell.where(y: 0): strong

#table(
  columns: 2,
  table.header[Amount][Ingredient],
  [360g], [Baking flour],
 // ... the remaining cells
)

```

--------------------------------

### HTML Menu Element

Source: https://typst.app/docs/reference/html/typed

The menu element represents a list of commands.

```APIDOC
## HTML Menu Element

### Description
Menu of commands.

### Method
html.menu

### Parameters
#### Path Parameters
None

#### Query Parameters
None

#### Request Body
- **content** (content) - Positional parameter for the contents of the HTML element.

### Request Example
```json
{
  "content": "Menu items"
}
```

### Response
#### Success Response (200)
- **content** (content) - The content within the menu element.
```

--------------------------------

### Basic Quote Usage with Attribution

Source: https://typst.app/docs/reference/model/quote

Demonstrates how to use the `quote` element for both inline and block quotes, including attributions in different languages and translations.

```typst
Plato is often misquoted as the author of #quote[I know that I know
nothing], however, this is a derivation form his original quote:

#set quote(block: true)

#quote(attribution: [Plato])[
  ... ἔοικα γοῦν τούτου γε σμικρῷ τινι αὐτῷ τούτῳ σοφώτερος εἶναι, ὅτι
  ἃ μὴ οἶδα οὐδὲ οἴομαι εἰδέναι.
]
#quote(attribution: [from the Henry Cary literal translation of 1897])[
  ... I seem, then, in just this little thing to be wiser than this man at
  any rate, that what I do not know I do not think I know either.
]
```

--------------------------------

### Smart Quotes Algorithm

Source: https://typst.app/docs/changelog/0.12.0

A new smart quote algorithm has been implemented, addressing various bugs and improving the accuracy of smart quote insertion.

```typst
smart quote algorithm
```

--------------------------------

### Accessing Standard Symbols in Typst

Source: https://typst.app/docs/reference/foundations/symbol

Demonstrates accessing predefined symbols from the 'sym' and 'emoji' modules. Standard symbols can be accessed directly in math mode.

```typst
#sym.arrow.r \
#sym.gt.eq.not \
$gt.eq.not$ \
#emoji.face.halo
```

--------------------------------

### Create a subscript element

Source: https://typst.app/docs/reference/html/typed

Use `html.sub` to render subscript text. It accepts content as a positional parameter.

```typst
html.sub(content)
```

--------------------------------

### Perform Euclidean division with `calc.div-euclid`

Source: https://typst.app/docs/reference/foundations/calc

Performs Euclidean division, rounding the result to the nearest integer such that the dividend is greater than or equal to the product of the integer and the divisor. Can error with large results.

```typst
#calc.div-euclid(7, 3) 
#calc.div-euclid(7, -3) 
#calc.div-euclid(-7, 3) 
#calc.div-euclid(-7, -3) 
#calc.div-euclid(1.75, 0.5) 
#calc.div-euclid(decimal("1.75"), decimal("0.5"))
```

--------------------------------

### Footnote with Labels and Multiple References

Source: https://typst.app/docs/reference/model/footnote

Shows how to use labels to create multiple references to the same footnote. This is useful for citing the same source multiple times.

```typst
You can edit Typst documents online.
#footnote[https://typst.app/app] <fn>
Checkout Typst's website. @fn
And the online app. #footnote(<fn>)
```

--------------------------------

### Image Fitting Options in Typst

Source: https://typst.app/docs/reference/visualize/image

Demonstrates different 'fit' modes for images within a defined area. Use 'cover' to fill the area by cropping, 'contain' to fit the whole image without cropping, and 'stretch' to fill the area by distorting the image.

```typst
#set page(width: 300pt, height: 50pt, margin: 10pt)
#image("tiger.jpg", width: 100%, fit: "cover")
#image("tiger.jpg", width: 100%, fit: "contain")
#image("tiger.jpg", width: 100%, fit: "stretch")
```

--------------------------------

### Create a Typed HTML Division Element

Source: https://typst.app/docs/changelog/0.14.0

Demonstrates the typed HTML API for creating HTML elements with individually typed attributes. Use `html.div` to create a division element.

```typst
html.div(id: "main", class: "container", "Hello, world!")
```

--------------------------------

### Typst For and While Loops

Source: https://typst.app/docs/reference/scripting

Demonstrates the usage of 'for' and 'while' loops in Typst. 'for' loops iterate over collections, while 'while' loops execute as long as a condition is true. Both join results from iterations.

```typst
#for c in "ABC" [
  #c is a letter.
]

#let n = 2
#while n < 10 {
  n = (n * 2) - 1
  (n,)
}

```

--------------------------------

### Scaling with `origin` Parameter

Source: https://typst.app/docs/reference/layout/scale

Illustrates how to change the origin of the scaling transformation using the `origin` parameter. This affects how the content is scaled relative to a point.

```typst
A#box(scale(75%)[A])A 
B#box(scale(75%, origin: bottom + left)[B])B

```

--------------------------------

### Create a superscript element

Source: https://typst.app/docs/reference/html/typed

Use `html.sup` to render superscript text. It accepts content as a positional parameter.

```typst
html.sup(content)
```

--------------------------------

### Typst: Headings and Emphasis

Source: https://typst.app/docs/tutorial/writing-in-typst

Shows how to create headings and emphasize text using Typst's simple markup. A single equals sign '=' denotes a main heading, while underscores '_' are used for italicizing text.

```typst
= Introduction
In this report, we will explore the
various factors that influence _fluid
dynamics_ in glaciers and how they
contribute to the formation and
behaviour of these natural structures.

```

--------------------------------

### List Element Parameters

Source: https://typst.app/docs/reference/model/list

Details the settable and positional parameters for the list element, including their types, descriptions, and default values.

```APIDOC
## Parameters

Parameters are the inputs to a function. They are specified in parentheses after the function name. 
list(
tight: bool,marker: contentarrayfunction,indent: length,body-indent: length,spacing: autolength,..content,
) -> content

### `tight`
bool
Settable
Defines the default spacing of the list. If it is `false`, the items are spaced apart with paragraph spacing. If it is `true`, they use paragraph leading instead. This makes the list more compact, which can look better if the items are short.
In markup mode, the value of this parameter is determined based on whether items are separated with a blank line. If items directly follow each other, this is set to `true`; if items are separated by a blank line, this is set to `false`. The markup-defined tightness cannot be overridden with set rules.
```typ
- If a list has a lot of text, and
  maybe other inline content, it
  should not be tight anymore.

- To make a list wide, simply insert
  a blank line between the items.
```

Default: `true`

### `marker`
content or array or function
Settable
The marker which introduces each item.
Instead of plain content, you can also pass an array with multiple markers that should be used for nested lists. If the list nesting depth exceeds the number of markers, the markers are cycled. For total control, you may pass a function that maps the list's nesting depth (starting from `0`) to a desired marker.
```typ
#set list(marker: [--])
- A more classic list
- With en-dashes

#set list(marker: ([•], [--]))
- Top-level
  - Nested
  - Items
- Items
```

Default: `([•], [‣], [–])`

### `indent`
length
Settable
The indent of each item.
Default: `0pt`

### `body-indent`
length
Settable
The spacing between the marker and the body of each item.
Default: `0.5em`

### `spacing`
auto or length
Settable
The spacing between the items of the list.
If set to `auto`, uses paragraph `leading` for tight lists and paragraph `spacing` for wide (non-tight) lists.
Default: `auto`

### `children`
content
Required Positional
Variadic
The bullet list's children.
When using the list syntax, adjacent items are automatically collected into lists, even through constructs like for loops.
```typ
#for letter in "ABC" [
  - Letter #letter
]
```
```

--------------------------------

### Configuring `terms` Element Tightness

Source: https://typst.app/docs/reference/model/terms

Demonstrates how to control the spacing between term list items. Setting `tight: false` with a blank line between items increases spacing, while `tight: true` (default) uses tighter spacing. This is useful for adjusting visual density based on content length.

```typst
/ Fact: If a term list has a lot
  of text, and maybe other inline
  content, it should not be tight
  anymore.

/ Tip: To make it wide, simply
  insert a blank line between the
  items.

```

--------------------------------

### Plugin Function Definition

Source: https://typst.app/docs/reference/foundations/plugin

Defines how plugin functions are called and their parameters. The `plugin` function takes a string or bytes representing the WebAssembly file and returns a module.

```APIDOC
## plugin

### Description
Loads a WebAssembly plugin and returns a module object.

### Method
`plugin`

### Parameters
#### Path Parameters
- **source** (str or bytes) - Required - A path to a WebAssembly file or raw WebAssembly bytes.

### Response
#### Success Response (module)
- **module** - An object representing the loaded plugin module.
```

--------------------------------

### Typst Template Function with Set and Show Rules

Source: https://typst.app/docs/tutorial/making-a-template

Defines a Typst template function 'template' that applies text styling and a string-based show rule to its content. The function then includes the document content.

```typst
#let template(doc) = [
  #set text(font: "Inria Serif")
  #show "something cool": [Typst]
  #doc
]

#show: template
I am learning something cool today.
It's going great so far!

```

--------------------------------

### Accessibility and HTML Export

Source: https://typst.app/docs/reference/model/heading

Discusses the importance of headings for accessibility and how they are translated during HTML export.

```APIDOC
## Accessibility

Headings are crucial for accessibility, enabling screen readers to navigate documents. Avoid skipping heading levels (e.g., go from level 1 to 3) and maintain hierarchical order.

## HTML Export

In HTML export, a `title` element becomes `<h1>`, and headings become `<h2>` and lower (level 1 heading becomes `<h2>`, level 2 becomes `<h3>`, etc.). This differs from the HTML standard where only one `<h1>` is recommended per document.
```

--------------------------------

### Sample Gradient at Multiple Positions

Source: https://typst.app/docs/reference/visualize/gradient

Samples the gradient at multiple positions simultaneously and returns the results as an array. Use `ts` for variadic positions.

```typst
self.samples(..angleratio)
```

--------------------------------

### Style First Column Cells with Show Rule

Source: https://typst.app/docs/reference/model/table

Applies a strong style to all cells in the first column (x: 0) using a show rule. This is effective for highlighting specific columns.

```typst
#show table.cell.where(x: 0): strong

#table(
  columns: 3,
  gutter: 3pt,
  [Name], [Age], [Strength],
  [Hannes], [36], [Grace],
  [Irma], [50], [Resourcefulness],
  [Vikram], [49], [Perseverance],
)


```

--------------------------------

### Scaling with `reflow` Parameter

Source: https://typst.app/docs/reference/layout/scale

Shows how the `reflow` parameter in the `scale` element affects layout. When `reflow` is `true`, the layout adjusts to the scaled content.

```typst
Hello #scale(x: 20%, y: 40%, reflow: true)[World]!

```

--------------------------------

### Quote Formatting Based on Language

Source: https://typst.app/docs/reference/model/quote

Demonstrates how the `quotes` parameter and `text(lang:)` setting influence the display of double quotes around a quote.

```typst
#set text(lang: "de")

Ein deutsch-sprechender Author
zitiert unter umständen JFK:
#quote[Ich bin ein Berliner.]

#set text(lang: "en")

And an english speaking one may
translate the quote:
#quote[I am a Berliner.]
```

--------------------------------

### Content Representation and Fields

Source: https://typst.app/docs/reference/foundations/content

Details how content is represented as elements with fields, distinguishing between required and optional fields, and how to customize appearance with show rules.

```APIDOC
## Representation
Content consists of elements with fields. When constructing an element with its _element function,_ you provide these fields as arguments and when you have a content value, you can access its fields with field access syntax.

Some fields are required: These must be provided when constructing an element and as a consequence, they are always available through field access on content of that type. Required fields are marked as such in the documentation.

Most fields are optional: Like required fields, they can be passed to the element function to configure them for a single element. However, these can also be configured with set rules to apply them to all elements within a scope. Optional fields are only available with field access syntax when they were explicitly passed to the element function, not when they result from a set rule.

Each element has a default appearance. However, you can also completely customize its appearance with a show rule. The show rule is passed the element. It can access the element's field and produce arbitrary content from it.

In the web app, you can hover over a content variable to see exactly which elements the content is composed of and what fields they have. Alternatively, you can inspect the output of the `repr` function.
```

--------------------------------

### Typst `strong` Element Usage

Source: https://typst.app/docs/reference/model/strong

Demonstrates different ways to use the `strong` element for emphasis, including direct application and with custom show rules.

```typst
This is *strong.* \nThis is #strong[too.] \n
#show strong: set text(red)
And this is *evermore.*
```

--------------------------------

### Scripting: 2D Alignment Inverse Method

Source: https://typst.app/docs/changelog/0.7.0

The `inv` method is now available for 2D alignments, allowing for easy inversion of alignment transformations.

```typst
let align_2d = (x: 0.5, y: 0.5)
let inverted_align_2d = align_2d.inv()
```

--------------------------------

### Force Script Style in Math

Source: https://typst.app/docs/reference/math/sizes

Use the `script` function for the smaller size used in powers or sub/superscripts. It takes content and an optional `cramped` boolean, defaulting to true.

```typst
$sum_i x_i/2 = script(sum_i x_i/2)$
```

--------------------------------

### Construct Arguments in Place

Source: https://typst.app/docs/reference/foundations/arguments

The `arguments` constructor can be used to create an `arguments` type directly, useful for passing arguments to other functions like `box`.

```typst
#let args = arguments(stroke: red, inset: 1em, [Body])
#box(..args)

```

--------------------------------

### Enabling Multiple Stylistic Sets

Source: https://typst.app/docs/changelog/0.12.0

It is now possible to enable multiple stylistic sets simultaneously for advanced typographic control.

```typst
Multiple stylistic sets
```

--------------------------------

### Typst Data Types and Variables

Source: https://typst.app/docs/guides/for-latex-users

Illustrates various data types in Typst, including Content, String, Integer, Floating point number, Absolute length, and Relative length. It also shows how to declare and use variables with the 'let' keyword.

```typst
// Store the integer `5`.
#let five = 5

// Define a function that
// increments a value.
#let inc(i) = i + 1

// Reference the variables.
I have #five fingers.

If I had one more, I'd have
#inc(five) fingers. Whoa!
```

--------------------------------

### Programmatic Item Numbering

Source: https://typst.app/docs/reference/model/enum

Illustrates using `enum.item` to explicitly set the number for each item in an enumeration.

```typst
#enum(
  enum.item(1)[First step],
  enum.item(5)[Fifth step],
  enum.item(10)[Tenth step]
)
```

--------------------------------

### Vector and Matrix Alignment Parameters

Source: https://typst.app/docs/changelog/0.12.0

New parameters `vec.align` and `mat.align` have been added to control the alignment of elements within vectors and matrices.

```typst
vec.align
```

```typst
mat.align
```

--------------------------------

### Import Plugin Functions in Typst

Source: https://typst.app/docs/reference/foundations/plugin

Imports specific functions from a loaded WebAssembly plugin. This is a more direct way to access plugin functionality.

```typst
#import plugin("hello.wasm"): concatenate

```

--------------------------------

### h Element - Weak Spacing

Source: https://typst.app/docs/reference/layout/h

Explains the 'weak' parameter and its behavior, including collapsing spacing and interaction with markup spaces.

```APIDOC
## `h` Element - Weak Spacing

### Description
The `weak` parameter, when set to `true`, causes the spacing to collapse at the start or end of a paragraph. If multiple adjacent weak spacings are used, all but the largest one collapse. Weak spacing also removes adjacent markup spaces. To force a space next to weak spacing, use `" "` for a normal space or `~` for a non-breaking space.

### Method
Not applicable (element function)

### Endpoint
Not applicable (element function)

### Parameters
#### Positional Parameters
- **amount** (relativefraction) - Required - How much spacing to insert.

#### Settable Parameters
- **weak** (bool) - Optional - If `true`, enables weak spacing behavior. Default: `false`.

### Request Example
```typst
#h(1cm, weak: true)
We identified a group of _weak_
specimens that fail to manifest
in most cases. However, when
#h(8pt, weak: true) supported
#h(8pt, weak: true) on both sides,
they do show up.

Further #h(0pt, weak: true) more,
even the smallest of them swallow
adjacent markup spaces.
```

### Response
Not applicable (element function)
```

--------------------------------

### Create a time element

Source: https://typst.app/docs/reference/html/typed

Use the `time` function to create a machine-readable date or time element. It requires a datetime or duration and content.

```typst
html.time(datetime: datetimeduration, content)
```

--------------------------------

### Customizing Ref Element Form and Supplement

Source: https://typst.app/docs/reference/model/ref

Shows how to globally customize the `ref` element's form to 'page' and modify the page supplement using `set` rules.

```APIDOC
## Customization
When you only ever need to reference pages of a figure/table/heading/etc. in a document, the default `form` field value can be changed to `"page"` with a set rule. If you prefer a short "p." supplement over "page", the `page.supplement` field can be used for changing this:
```
#set page(
  numbering: "1",
  supplement: "p.",
)
#set ref(form: "page")

#figure(
  stack(
    dir: ltr,
    spacing: 1em,
    circle(),
    square(),
  ),
  caption: [Shapes],
) <shapes>

#pagebreak()

See @shapes for examples
of different shapes.

```
```

--------------------------------

### Create Oklch Color

Source: https://typst.app/docs/reference/visualize/color

Use this to create an Oklch color. Components include lightness, chroma, hue, and alpha. Ratios for chroma are relative to 0.4.

```typst
#square(
  fill: oklch(40%, 0.2, 160deg, 50%)
)
```

--------------------------------

### Align Table Cells Across Columns

Source: https://typst.app/docs/reference/model/table

Shows how to set column alignment using an array of alignment values for left, center, and right justification.

```typst
#table(
  columns: 3,
  align: (left, center, right),
  [Hello], [Hello], [Hello],
  [A], [B], [C],
)
```

--------------------------------

### Math Operators: Aliases for Aleph, Beth, Gimmel

Source: https://typst.app/docs/changelog/0.7.0

New aliases `aleph`, `beth`, and `gimmel` are available for `alef`, `bet`, and `gimel`, respectively. These provide alternative names for common mathematical symbols.

```typst
$aleph$
```

```typst
$alef$
```

```typst
$beth$
```

```typst
$bet$
```

```typst
$gimmel$
```

```typst
$gimel$
```

--------------------------------

### Typst Destructuring: Arrays and Dictionaries

Source: https://typst.app/docs/reference/scripting

Shows how 'let' bindings can destructure arrays and dictionaries. The '..' operator collects remaining items. Destructuring also works with dictionary key-value pairs.

```typst
#let (x, y) = (1, 2)
The coordinates are #x, #y.

#let (a, .., b) = (1, 2, 3, 4)
The first element is #a.
The last element is #b.

#let books = (
  Shakespeare: "Hamlet",
  Homer: "The Odyssey",
  Austen: "Persuasion",
)

#let (Austen,) = books
Austen wrote #Austen.

#let (Homer: h) = books
Homer wrote #h.

#let (Homer, ..other) = books
#for (author, title) in other [
  #author wrote #title.
]

```

--------------------------------

### Interpreting Bytes as a Typst Float

Source: https://typst.app/docs/reference/foundations/float

Shows how to use `float.from-bytes()` to convert byte sequences into floating-point numbers. Supports both little-endian and big-endian byte orders, and interprets bytes as either 32-bit or 64-bit floats based on length.

```typst
#float.from-bytes(bytes((0, 0, 0, 0, 0, 0, 240, 63))) \
#float.from-bytes(bytes((63, 240, 0, 0, 0, 0, 0, 0)), endian: "big")
```

--------------------------------

### Export Typst to PNG via Command Line

Source: https://typst.app/docs/reference/png

Export Typst documents to PNG format using the command line. This method allows specifying the output format, resolution (PPI), and which pages to include. For multi-page documents, a filename template with page numbering is required.

```bash
# Compile a single page document to PNG with default PPI
typst compile --format png document.typ

# Compile a single page document to PNG with custom PPI
typst compile --format png --ppi 300 document.typ

# Compile a multi-page document to PNG with custom PPI and filename template
typst compile --format png --ppi 600 --output "output/page-{p}.png" document.typ

# Compile specific pages to PNG
typst compile --format png --pages "1,3-5,7-" document.typ
```

--------------------------------

### par Element Configuration

Source: https://typst.app/docs/reference/model/par

This snippet demonstrates how to configure the par element using set rules to customize paragraph properties such as first-line indent, spacing, and justification.

```APIDOC
## `par` Element Configuration

### Description

This section details how to configure the `par` element to customize paragraph properties. You can use `set` rules to globally affect all following paragraphs or `show` rules for specific instances.

### Method

SET RULE / SHOW RULE

### Endpoint

N/A (Configuration within Typst script)

### Parameters

#### Settable Parameters for `par`:

- **`first-line-indent`** (length) - Settable - Controls the indentation of the first line of a paragraph.
- **`spacing`** (length) - Settable - Defines the spacing between paragraphs.
- **`justify`** (bool) - Settable - Enables or disables text justification for paragraphs.
- **`justification-limits`** (dictionary) - Settable - Specifies limits for justification.
- **`linebreaks`** (autostr) - Settable - Controls line breaking behavior.
- **`hanging-indent`** (length) - Settable - Controls hanging indentation.

### Request Example

```typst
#set par(
  first-line-indent: 1em,
  spacing: 0.65em,
  justify: true,
)
```

### Response

N/A (Configuration affects document layout)

### Response Example

N/A
```

--------------------------------

### Typst Function Arguments: Positional and Named

Source: https://typst.app/docs/guides/for-latex-users

Demonstrates how to define and use functions with both positional and named arguments in Typst. Named arguments enhance legibility by explicitly stating the parameter name. Content can be passed as trailing arguments.

```typst
#lower("SCREAM")

#rect(
  width: 2cm,
  height: 1cm,
  stroke: red,
)

#underline([Alternative A])

Typst is an #underline[alternative]
to LaTeX.

#rect(fill: aqua)[Get started here!]
```

--------------------------------

### Typst: Numbered Lists

Source: https://typst.app/docs/tutorial/writing-in-typst

Illustrates the creation of numbered lists in Typst. Each list item is initiated with a '+' character, and Typst automatically handles the numbering.

```typst
+ The climate
+ The topography
+ The geology

```

--------------------------------

### math.accent Function

Source: https://typst.app/docs/reference/math/accent

Detailed documentation for the math.accent function, including parameters and return types.

```APIDOC
## `math.accent` Function

### Description
Attaches an accent to a base character or string.

### Method
`math.accent`

### Parameters

#### Path Parameters
None

#### Query Parameters
None

#### Request Body
None

### Parameters

- **content** (content): Required Positional. The base to which the accent is applied. May consist of multiple letters.
  Example: `$arrow(A B C)$`

- **strcontent** (str or content): Required Positional. The accent to apply to the base. Supported accents include Grave, Acute, Circumflex, Tilde, Macron, Dash, Breve, Dot, Double dot, Triple dot, Quadruple dot, Circle, Double acute, Caron, Right arrow, Left arrow, Left/Right arrow, Right harpoon, Left harpoon.
  Example: `accent(a, ~)`

- **size** (relative): Settable. The size of the accent, relative to the width of the base.
  Default: `100% + 0pt`
  Example: `$dash(A, size: #150%)$`

- **dotless** (bool): Settable. Whether to remove the dot on top of lowercase i and j when adding a top accent. This enables the `dtls` OpenType feature.
  Default: `true`
  Example: `$hat(dotless: #false, i)$`

### Request Example
```json
{
  "content": "a",
  "accent": "grave",
  "size": "100%",
  "dotless": true
}
```

### Response
#### Success Response (200)
- **content** (content): The resulting content with the accent applied.

#### Response Example
```json
{
  "content": "\u00e0"
}
```
```

--------------------------------

### Mat Element Parameters

Source: https://typst.app/docs/reference/math/mat

Details the settable and non-settable parameters for the `mat` element, including their types, descriptions, and default values.

```APIDOC
## Parameters for `math.mat`

### `delim`

- **Type**: `none` or `str` or `array` or `symbol`
- **Settable**: Yes
- **Description**: The delimiter to use for the matrix. Can be a single character for both left and right delimiters, or an array containing distinct left and right delimiters.
- **Default**: `("(", ")")`

### `align`

- **Type**: `alignment`
- **Settable**: Yes
- **Description**: The horizontal alignment for each cell in the matrix.
- **Default**: `center`

### `augment`

- **Type**: `none` or `int` or `dictionary`
- **Settable**: Yes
- **Description**: Draws augmentation lines in a matrix. Can be `none` for no lines, a single number for a vertical line after a column, or a dictionary to specify multiple horizontal and vertical lines, including their `stroke` style.
- **Default**: `none`

### `gap`

- **Type**: `relative`
- **Description**: Shorthand for setting both `row-gap` and `column-gap` to the same value.
- **Default**: `0% + 0pt`

### `row-gap`

- **Type**: `relative`
- **Settable**: Yes
- **Description**: The gap between rows in the matrix.
- **Default**: `0% + 0.2em`

### `column-gap`

- **Type**: `relative`
- **Settable**: Yes
- **Description**: The gap between columns in the matrix.
- **Default**: `0% + 0.5em`

### `rows`

- **Type**: `array`
- **Required Positional**: Yes
- **Variadic**: Yes
- **Description**: An array of arrays representing the rows and their elements in the matrix.
```

--------------------------------

### Advanced Attribution and Styling

Source: https://typst.app/docs/reference/model/quote

Shows advanced usage of the `attribution` parameter, including linking to external resources and custom `show` rules for inline quotes.

```typst
#quote(attribution: [René Descartes])[
  cogito, ergo sum
]

#show quote.where(block: false): it => {
  [ "] + h(0pt, weak: true) + it.body + h(0pt, weak: true) + [ "]
  if it.attribution != none [ (#it.attribution)]
}

#quote(
  attribution: link("https://typst.app/home")[typst.app]
)[
  Compose papers faster
]

#set quote(block: true)

#quote(attribution: <tolkien54>)[
  You cannot pass... I am a servant
  of the Secret Fire, wielder of the
  flame of Anor. You cannot pass. The
  dark fire will not avail you, flame
  of Udûn. Go back to the Shadow! You
  cannot pass.
]

#bibliography("works.bib", style: "apa")
```

--------------------------------

### Typst: Apply Styles to Labelled Elements

Source: https://typst.app/docs/reference/foundations/label

Demonstrates how to use the '#show' directive to apply styles to elements based on their labels. It shows styling for a specific element ('a') and a labelled element ('label("b")').

```typst
#show <a>: set text(blue)
#show label("b"): set text(red)

= Heading <a>
*Strong* #label("b")

```

--------------------------------

### Create a Figure with an Image and Caption

Source: https://typst.app/docs/reference/visualize/image

Use the `figure` element to wrap an image, providing it with a caption and a number. This is useful for academic or technical documents.

```typst
#figure(
  image("molecular.jpg", width: 80%),
  caption: [
    A step in the molecular testing
    pipeline of our lab.
  ],
)

```

--------------------------------

### Math Operators: Aliases for Nothing and Curly Brackets

Source: https://typst.app/docs/changelog/0.7.0

The `emptyset` alias is now available for `nothing`. Additionally, `lt.curly` and `gt.curly` are aliases for `prec` and `succ`, respectively, providing convenient shorthand.

```typst
$emptyset$
```

```typst
$nothing$
```

```typst
$lt.curly(a, b)$
```

```typst
$prec(a, b)$
```

```typst
$gt.curly(a, b)$
```

```typst
$succ(a, b)$
```

--------------------------------

### Parent-Scoped Floating Placement

Source: https://typst.app/docs/reference/layout/place

Shows how to use `place` with `scope: "parent"` and `float: true` to position content relative to the parent container, allowing it to span across columns. This is useful for creating single-column titles in multi-column documents.

```typst
#set page(height: 150pt, columns: 2)
#place(
  top + center,
  scope: "parent",
  float: true,
  rect(width: 80%, fill: aqua),
)

#lorem(25)


```

--------------------------------

### Custom Figure Numbering and Counters

Source: https://typst.app/docs/reference/model/figure

Demonstrates how to manage figure numbering and counters, specifically for tables. It shows resetting a counter and then creating a figure that uses the updated number.

```typst
#figure(
  table(columns: 2, $n$, $12$),
  caption: [The first table.],
)

#counter(
  figure.where(kind: table)
).update(41)

#figure(
  table(columns: 2, $n$, $42$),
  caption: [The 42nd table],
)

#figure(
  rect[Image],
  caption: [Does not affect images],
)

```

--------------------------------

### Create a small text element

Source: https://typst.app/docs/reference/html/typed

Use `html.small` to render side comments or small text. It accepts content as a positional parameter.

```typst
html.small(content)
```

--------------------------------

### Typst Scripting: CBOR and Byte Encoding/Decoding

Source: https://typst.app/docs/changelog/0.8.0

Illustrates how to encode and decode data using CBOR and other formats to and from bytes in Typst. This is useful for inter-process communication or data serialization.

```typst
// CBOR encoding/decoding
#let data = (a: 1, b: "hello")
#let encoded = cbor.encode(data)
#let decoded = cbor.decode(encoded)

// JSON encoding/decoding
#let json_data = "{\"key\": \"value\"}"
#let parsed_json = json.decode(json_data)
#let encoded_json = json.encode(parsed_json)
```

--------------------------------

### Typst: Basic Text Input

Source: https://typst.app/docs/tutorial/writing-in-typst

Demonstrates how to input basic prose into the Typst editor. The text entered in the source panel is immediately rendered in the preview panel.

```typst
In this report, we will explore the
various factors that influence fluid
dynamics in glaciers and how they
contribute to the formation and
behaviour of these natural structures.

```

--------------------------------

### Generate Boxes with Varying Luma Values

Source: https://typst.app/docs/reference/visualize/color

Creates a series of boxes, each filled with a grayscale color generated by the `luma` function with different lightness values.

```typst
#for x in range(250, step: 50) {
  box(square(fill: luma(x)))
}


```

--------------------------------

### Create an HTML footer element

Source: https://typst.app/docs/reference/html/typed

Use this function to create an HTML `<footer>` element for a page or section footer. It takes content as a positional parameter.

```typst
html.footer(
content
)
```

--------------------------------

### Typst Limits Function to Force Limit Positioning

Source: https://typst.app/docs/reference/math/attach

Demonstrates the `limits` function to force attachments to be displayed as limits, similar to how they appear in summation or integral notations.

```typst
$ limits(A)_1^2 != A_1^2 $
```

--------------------------------

### Load Bibliography with Typst

Source: https://typst.app/docs/guides/for-latex-users

Demonstrates how to load a bibliography using the `bibliography` function in Typst. This function is compatible with BibTeX files and can also use Typst's native YAML format. It's the primary method for integrating external literature sources.

```typst
#bibliography("biblio.bib")
#bibliography("biblio.yaml")
```

--------------------------------

### Converting a Typst Float to Bytes

Source: https://typst.app/docs/reference/foundations/float

Demonstrates the `to-bytes()` method for floats, which converts a float into its byte representation. Allows specifying endianness (big or little) and the desired byte size (4 for single-precision, 8 for double-precision).

```typst
#array(1.0.to-bytes(endian: "big")) \
#array(1.0.to-bytes())
```

--------------------------------

### Set Tab Size for Raw Code Blocks

Source: https://typst.app/docs/reference/text/raw

Control the tab stop size in spaces for raw code blocks with the `tab-size` parameter. Tabs are replaced with spaces to align with the next multiple of the size.

```typ
#set raw(tab-size: 8)
Year	Month	Day
2000	2	3
2001	2	1
2002	3	10

```

--------------------------------

### Displaying Linear, Radial, and Conic Gradients

Source: https://typst.app/docs/reference/visualize/gradient

Demonstrates the usage of linear, radial, and conic gradients with the rainbow color map. These can be applied as fills to shapes.

```typst
#stack(
  dir: ltr,
  spacing: 1fr,
  square(fill: gradient.linear(..color.map.rainbow)),
  square(fill: gradient.radial(..color.map.rainbow)),
  square(fill: gradient.conic(..color.map.rainbow)),
)
```

--------------------------------

### String Splitting

Source: https://typst.app/docs/reference/foundations/str

Explains the `split` function, which divides a string into an array of substrings based on a specified pattern. Special handling for empty string separators is also described.

```APIDOC
## String Splitting (`split`)

### Description
Splits a string at matches of a specified pattern and returns an array of the resulting parts. When the empty string is used as a separator, it separates every character (i.e., Unicode code point) in the string, along with the beginning and end of the string.

### Method
`split(pattern)`

### Parameters
#### Positional Parameters
- **pattern** (none or str or regex) - Optional - The pattern to split at. Defaults to whitespace.

### Request Example
```json
{
  "example": "splitting a string by comma"
}
```

### Response
#### Success Response (200)
- **array** (array) - An array of substrings.

#### Response Example
```json
{
  "example": ["part1", "part2", "part3"]
}
```
```

--------------------------------

### Gradient Properties

Source: https://typst.app/docs/reference/visualize/gradient

Explains the properties that define a gradient's appearance and behavior.

```APIDOC
## Gradient Properties

### `space`

- **Type**: any
- **Description**: The color space in which to interpolate the gradient. Defaults to a perceptually uniform color space called Oklab.
- **Default**: `oklab`

### `relative`

- **Type**: auto or str
- **Description**: The relative placement of the gradient. For an element placed at the root/top level of the document, the parent is the page itself. For other elements, the parent is the innermost block, box, column, grid, or stack that contains the element.
  - **Variant**: `"self"` - Relative to itself (its own bounding box).
  - **Variant**: `"parent"` - Relative to its parent (the parent's bounding box).
- **Default**: `auto`

### `center`

- **Type**: array
- **Description**: The center of the circle of the gradient. A value of `(50%, 50%)` means that the circle is centered inside of its container.
- **Default**: `(50%, 50%)`
```

--------------------------------

### Typst: Define Metadata for Querying

Source: https://typst.app/docs/reference/introspection/query

Shows how to define metadata within a Typst document using the `#metadata` function, which can then be queried from the command line.

```typst
#metadata("This is a note") <note>


```

--------------------------------

### Basic `smallcaps` Usage

Source: https://typst.app/docs/reference/text/smallcaps

Displays text in small capitals. This is the most basic usage of the `smallcaps` element.

```typst
Hello \\n#smallcaps[Hello]
```

--------------------------------

### Demonstrate miter-limit in Typst Curves

Source: https://typst.app/docs/reference/visualize/stroke

Shows how different miter-limit values affect the rendering of sharp bends in curves. Applicable when join is set to 'miter'.

```typst
#let items = (
  curve.move((15pt, 0pt)),
  curve.line((0pt, 30pt)),
  curve.line((30pt, 30pt)),
  curve.line((10pt, 20pt)),
)

#set curve(stroke: 6pt + blue)
#stack(
  dir: ltr,
  spacing: 1cm,
  curve(stroke: (miter-limit: 1), ..items),
  curve(stroke: (miter-limit: 4), ..items),
  curve(stroke: (miter-limit: 5), ..items),
)


```

--------------------------------

### Generate HTML Video Element with Typed API

Source: https://typst.app/docs/changelog/0.14.0

Use the typed HTML API for generating elements like video. The `width` attribute now accepts an integer directly.

```typst
#html.video(width: 400, src: "sunrise.mp4")
```

--------------------------------

### HTML Address (address) Element

Source: https://typst.app/docs/reference/html/typed

Documentation for the `html.address` function, used to create address elements.

```APIDOC
## `address` - Contact Information

### Description
Creates an address element for contact information.

### Method
`html.address`

### Parameters
#### Path Parameters
None

#### Query Parameters
None

#### Request Body
None

#### Attributes
- **content** (content) - Required - The contents of the HTML element.

### Request Example
```typc
#html.address()[
  123 Main St, Anytown, USA
]
```

### Response
#### Success Response (200)
- **content** (content) - The rendered HTML address element.
```

--------------------------------

### Stroke Constructor API

Source: https://typst.app/docs/reference/visualize/stroke

API documentation for the stroke constructor, detailing its parameters and their types.

```APIDOC
## stroke()

Converts a value to a stroke or constructs a stroke with the given parameters.

### Parameters

#### `paint`
auto or color or gradient or tiling
Required Positional
The color or gradient to use for the stroke. Defaults to `black` if `auto`.

#### `thickness`
auto or length
Required Positional
The stroke's thickness. Defaults to `1pt` if `auto`.

#### `cap`
auto or str
Required Positional
How the ends of the stroke are rendered. Defaults to `"butt"` if `auto`.
Variants:
- `"butt"`: Square stroke cap.
- `"round"`: Circular stroke cap.
- `"square"`: Square stroke cap centered at the stroke's end point.

#### `join`
auto or str
Required Positional
How sharp turns are rendered. Defaults to `"miter"` if `auto`.
Variants:
- `"miter"`: Sharp edges, bevelled if exceeding miter limit.
- `"round"`: Circular corners.
- `"bevel"`: Straight edge connecting segments.

#### `dash`
none or auto or str or array or dictionary
Required Positional
The dash pattern to use. Defaults to `none` if `auto`.
Predefined patterns:
- `"solid"` or `none`
- `"dotted"
- `"densely-dotted"
- `"loosely-dotted"
- `"dashed"
- `"densely-dashed"
- `"loosely-dashed"
- `"dash-dotted"
- `"densely-dash-dotted"
- `"loosely-dash-dotted"

Custom patterns can be defined using an array of alternating dash and gap lengths, or a dictionary with `array` and `phase` keys.
```

--------------------------------

### Handling Empty Paragraphs

Source: https://typst.app/docs/changelog/0.12.0

Fixes bugs related to the creation of empty paragraphs, particularly those generated using the `#""` syntax.

```typst
#""
```

--------------------------------

### Basic Figure with Image

Source: https://typst.app/docs/reference/model/figure

Demonstrates a basic figure containing an image with a caption. Typst automatically detects the content type for numbering.

```typst
#figure(
  image("glacier.jpg", width: 80%),
  caption: [A curious figure.],
) <glacier>
```

--------------------------------

### 2D Alignments

Source: https://typst.app/docs/reference/layout/alignment

Demonstrates how to combine alignments for both horizontal and vertical axes.

```APIDOC
## 2D Alignments

To align along both axes at the same time, add the two alignments using the `+` operator. For example, `top + right` aligns the content to the top right corner.

### Request Example

```typc
#set page(height: 3cm)
#align(center + bottom)[Hi]
```
```

--------------------------------

### Regex Constructor

Source: https://typst.app/docs/reference/foundations/regex

Constructs a regular expression from a string. Handles escaping for both Typst strings and regex syntax.

```APIDOC
## POST /regex

### Description
Creates a regular expression object from a string. This function is essential for using regular expressions with Typst's string methods and show rules.

### Method
POST

### Endpoint
/regex

### Parameters
#### Request Body
- **str** (string) - Required - The regular expression pattern as a string. Special care must be taken with backslashes due to Typst's string escaping rules. For example, to represent the regex `\\d`, you must use `"\\\\d"`.

### Request Example
```json
{
  "str": "\\d+" 
}
```

### Response
#### Success Response (200)
- **regex_object** (regex) - The created regular expression object.

#### Response Example
```json
{
  "regex_object": "regex(\"\\\d+\")"
}
```
```

--------------------------------

### Customizing Equation References with Show Rule

Source: https://typst.app/docs/reference/model/ref

Uses a `show` rule to customize how equation references are displayed, linking to the equation's number. It specifically targets math equations and handles cases where the element might not be found.

```typst
#set heading(numbering: "1.")
#set math.equation(numbering: "(1)")

#show ref: it => {
  let eq = math.equation
  let el = it.element
  // Skip all other references.
  if el == none or el.func() != eq { return it }
  // Override equation references.
  link(el.location(), numbering(
    el.numbering,
    ..counter(eq).at(el.location())
  ))
}

= Beginnings <beginning>
In @beginning we prove @pythagoras.
$ a^2 + b^2 = c^2 $ <pythagoras>

```

--------------------------------

### Image Scaling Options in Typst

Source: https://typst.app/docs/reference/visualize/image

Illustrates how to control image scaling behavior. 'smooth' uses interpolation for a softer look, while 'pixelated' preserves sharp edges. 'auto' leaves the decision to the viewer or Typst's export defaults.

```typst
#image("logo.png", scaling: "smooth")
#image("logo.png", scaling: "pixelated")
#image("logo.png", scaling: "auto")
```

--------------------------------

### Datetime Constructor

Source: https://typst.app/docs/reference/foundations/datetime

Creates a new datetime value. Depending on the provided components, it can store a date, a time, or a full datetime.

```APIDOC
## Constructor datetime

Creates a new datetime. You can specify the datetime using a year, month, day, hour, minute, and second.

* If you specify year, month and day, Typst will store just a date.
* If you specify hour, minute and second, Typst will store just a time.
* If you specify all of year, month, day, hour, minute and second, Typst will store a full datetime.

### Parameters
#### Path Parameters
- **year** (int) - Optional - The year of the datetime.
- **month** (int) - Optional - The month of the datetime.
- **day** (int) - Optional - The day of the datetime.
- **hour** (int) - Optional - The hour of the datetime.
- **minute** (int) - Optional - The minute of the datetime.
- **second** (int) - Optional - The second of the datetime.

### Request Example
```typ
#datetime(
  year: 2012,
  month: 8,
  day: 3,
).display()
```

### Response
#### Success Response (200)
- **datetime** (datetime) - The created datetime object.

#### Response Example
```json
{
  "datetime": "2012-08-03"
}
```
```

--------------------------------

### Typst Function Calls and Code Mode

Source: https://typst.app/docs/guides/for-latex-users

Explains and demonstrates Typst's code mode, initiated with a hash (#), for calling functions, performing calculations, and embedding markup within code blocks.

```typst
#rect()
#underline([_underlined_ text])
#calc.max(3, 2 * 4)
#for x in range(3) [
  Hi #x.
]
```

--------------------------------

### Create an HTML emphasis element

Source: https://typst.app/docs/reference/html/typed

Use this function to create an HTML `<em>` element for stress emphasis. It takes content as a positional parameter.

```typst
html.em(
content
)
```

--------------------------------

### Text Styling API

Source: https://typst.app/docs/reference/text/text

This section details the settable parameters for text styling in Typst, including style, weight, stretch, size, fill, stroke, tracking, spacing, and CJK-Latin spacing.

```APIDOC
## Text Styling Parameters

### `style`

**Type:** string
**Settable:** Yes

**Description:** Controls the font style (e.g., italic, oblique). If the exact style is not available, Typst will attempt to use a similar one. For emphasis, use the `emph` function instead.

**Variants:**
- `"normal"`: The default, upright style.
- `"italic"`: A cursive style with custom letterforms.
- `"oblique"`: A slanted version of the normal style.

**Default:** `"normal"`

### `weight`

**Type:** int or string
**Settable:** Yes

**Description:** Defines the thickness of the font's glyphs. Accepts an integer between 100 and 900 or predefined weight names. Typst selects the closest available weight if the exact one is not found. For strong emphasis, use the `strong` function.

**Variants:**
- `"thin"` (100)
- `"extralight"` (200)
- `"light"` (300)
- `"regular"` (400)
- `"medium"` (500)
- `"semibold"` (600)
- `"bold"` (700)
- `"extrabold"` (800)
- `"black"` (900)

**Default:** `"regular"`

### `stretch`

**Type:** ratio
**Settable:** Yes

**Description:** Adjusts the width of the glyphs, accepting values between 50% and 200%. Typst selects the closest available stretch if the exact one is not found. This only affects text if condensed or expanded font versions are available. For spacing between characters, use `tracking`.

**Default:** `100%`

### `size`

**Type:** length
**Settable:** Yes

**Description:** Sets the size of the glyphs. This value is the basis for the `em` unit. Font size can also be specified in `em` units, making it relative to the previous font size.

**Default:** `11pt`

### `fill`

**Type:** color or gradient or tiling
**Settable:** Yes

**Description:** Specifies the paint used to fill the glyphs.

**Default:** `luma(0%)`

### `stroke`

**Type:** none or length or color or gradient or stroke or tiling or dictionary
**Settable:** Yes

**Description:** Defines how the text should be stroked.

**Default:** `none`

### `tracking`

**Type:** length
**Settable:** Yes

**Description:** Controls the amount of space added between characters.

**Default:** `0pt`

### `spacing`

**Type:** relative
**Settable:** Yes

**Description:** Adjusts the space between words. Can be an absolute length or relative to the width of the space character. For character spacing, use `tracking`.

**Default:** `100% + 0pt`

### `cjk-latin-spacing`

**Type:** none or auto
**Settable:** Yes

**Description:** Determines whether to automatically insert spacing between CJK and Latin characters.

**Default:** `auto`

### Examples

```typ
#text(font: "Libertinus Serif", style: "italic")[Italic]
#text(font: "DejaVu Sans", style: "oblique")[Oblique]

#set text(font: "IBM Plex Sans")
#text(weight: "light")[Light] \ #text(weight: "regular")[Regular] \ #text(weight: "medium")[Medium] \ #text(weight: 500)[Medium] \ #text(weight: "bold")[Bold]

#text(stretch: 75%)[Condensed] \ #text(stretch: 100%)[Normal]

#set text(size: 20pt)
very #text(1.5em)[big] text

#set text(fill: red)
This text is red.

#text(stroke: 0.5pt + red)[Stroked]

#set text(tracking: 1.5pt)
Distant text.

#set text(spacing: 200%)
Text with distant words.

#set text(cjk-latin-spacing: auto)
第4章介绍了基本的API。

#set text(cjk-latin-spacing: none)
第4章介绍了基本的API.
```
```

--------------------------------

### Typst Code Mode Syntax

Source: https://typst.app/docs/reference/syntax

Demonstrates how to enter and use code mode in Typst. Code mode is prefixed with '#', allowing for the use of Typst's scripting features and functions within the document.

```typst
Number: #(1 + 2)
```

--------------------------------

### Page Footer Configuration

Source: https://typst.app/docs/reference/layout/page

Defines the content and behavior of the page footer.

```APIDOC
## `footer`

### Description
The page's footer. Fills the bottom margin of each page.
  * Content: Shows the content as the footer.
  * `auto`: Shows the page number if a `numbering` is set and `number-align` is `bottom`.
  * `none`: Suppresses the footer.

### Method
`set`

### Parameters
#### Query Parameters
- **footer** (string or content) - Optional - The content of the footer, or `auto` to show page number, or `none` to suppress.

### Request Example
```json
{
  "footer": "auto"
}
```

### Response
#### Success Response (200)
- **footer** (string or content) - The current footer setting.

#### Response Example
```json
{
  "footer": "auto"
}
```
```

--------------------------------

### Citation with Supplement

Source: https://typst.app/docs/reference/model/cite

Demonstrates how to add a supplement, such as a page number, to a citation using square brackets `[...]` after the reference. This requires the bibliography to be defined.

```typst
This has been proven. @distress[p.~7]

#bibliography("works.bib")
```

--------------------------------

### Image Element Parameters

Source: https://typst.app/docs/reference/visualize/image

Details the parameters available for the `image` function, including source, format, dimensions, and accessibility.

```APIDOC
## Parameters
image(
strbytes,format: autostrdictionary,width: autorelative,height: autorelativefraction,alt: nonestr,page: int,fit: str,scaling: autostr,icc: autostrbytes,
) -> content

### `source`
str or bytes
Required Positional
A path to an image file or raw bytes making up an image in one of the supported formats.

### `format`
auto or str or dictionary
Settable
The image's format. By default, the format is detected automatically.

Supported formats are `"png"`, `"jpg"`, `"gif"`, `"svg"`, `"pdf"`, `"webp"` as well as raw pixel data.

When providing raw pixel data as the `source`, you must specify a dictionary with the following keys as the `format`:
  * `encoding` (str): The encoding of the pixel data. One of: `"rgb8"`, `"rgba8"`, `"luma8"`, `"lumaa8"
  * `width` (int): The pixel width of the image.
  * `height` (int): The pixel height of the image.

### `width`
auto or relative
Settable
The width of the image.

### `height`
auto or relative or fraction
Settable
The height of the image.

### `alt`
none or str
Settable
An alternative description of the image. This text is used by Assistive Technology (AT) like screen readers to describe the image to users with visual impairments.
```

--------------------------------

### Typst Bibliography: LaTeX Command Support

Source: https://typst.app/docs/changelog/0.10.0

More LaTeX commands, such as those for quotes, are now respected within `.bib` files, ensuring better compatibility with LaTeX-formatted bibliographic data.

```bibtex
More LaTeX commands (e.g. for quotes) are now respected in `.bib` files
```

--------------------------------

### Text Justification Configuration

Source: https://typst.app/docs/reference/model/par

Configuration for text justification within Typst. This parameter controls whether text is justified in its line and influences hyphenation and alignment.

```APIDOC
## `justify`

### Description
Whether to justify text in its line. Hyphenation will be enabled for justified paragraphs if the text function's `hyphenate` property is set to `auto` and the current language is known. Note that the current alignment still has an effect on the placement of the last line except if it ends with a justified line break. By default, Typst only changes the spacing between words to achieve justification. However, you can also allow it to adjust the spacing between individual characters using the `justification-limits` property.

### Type
bool

### Settable
Yes

### Default
`false`
```

--------------------------------

### Using `offset` for `overline` Positioning

Source: https://typst.app/docs/reference/text/overline

Illustrates adjusting the `offset` parameter to control the vertical position of the overline relative to the text baseline.

```typst
#overline(offset: -1.2em)[
  The Tale Of A Faraway Line II
]

```

--------------------------------

### Principled Counter Implementation

Source: https://typst.app/docs/changelog/0.12.0

A more principled implementation of counters has been introduced, fixing various bugs related to counter behavior in complex layout scenarios.

```typst
counter behavior in complex layout situations
```

--------------------------------

### Add Floating Figures with Placement Argument

Source: https://typst.app/docs/changelog/0.7.0

Use the `placement` argument in the `figure` function to enable floating figures. This allows for more flexible layout control.

```typst
figure(placement: "top")
```

--------------------------------

### Measure Content Size within a Context

Source: https://typst.app/docs/reference/layout/measure

Illustrates measuring content size using the `measure` function within a custom context function. This requires the `context` keyword to be available.

```typst
#let thing(body) = context {
  let size = measure(body)
  [Width of "#body" is #size.width]
}

#thing[Hey] \
#thing[Welcome]

```

--------------------------------

### Create a generic phrasing container

Source: https://typst.app/docs/reference/html/typed

Use `html.span` to create a generic container for phrasing content. It accepts content as a positional parameter.

```typst
html.span(content)
```

--------------------------------

### Typst List Tightness Control

Source: https://typst.app/docs/reference/model/list

Illustrates how to control the tightness of a Typst list. A tight list uses paragraph leading for spacing, while a non-tight list uses paragraph spacing. Inserting a blank line between items makes the list non-tight.

```typst
- If a list has a lot of text, and
  maybe other inline content, it
  should not be tight anymore.

- To make a list wide, simply insert
  a blank line between the items.

```

--------------------------------

### Typst Expressions: Function Calls, Field Access, Method Calls

Source: https://typst.app/docs/reference/scripting

Demonstrates basic Typst expressions including function calls like emph(), field access like emoji.face, and method calls like "hello".len(). Expressions are introduced with a hash (#).

```typst
#emph[Hello] \
#emoji.face \
#"hello".len()
```

--------------------------------

### Basic `cancel` Element Usage

Source: https://typst.app/docs/reference/math/cancel

Demonstrates the basic usage of the `cancel` element to show the elimination of a term in a mathematical expression.

```typst
$ (a dot b dot cancel(x)) /
    cancel(x) $
```

--------------------------------

### String Replacement and Trimming

Source: https://typst.app/docs/reference/foundations/str

Methods for replacing substrings and trimming strings.

```APIDOC
## `replace`

### Description
Replaces occurrences of a pattern with a replacement string or function.

### Method
`self.replace`

### Parameters
#### Positional Parameters
- **pattern** (str or regex) - Required - The pattern to search for.
- **replacement** (str or function) - Required - The string or function to replace matches with.
#### Named Parameters
- **count** (int) - Optional - The maximum number of occurrences to replace. If not provided, all occurrences are replaced.

### Response
- Returns the string with replacements made.
```

```APIDOC
## `trim`

### Description
Removes matches of a pattern from one or both sides of the string, once or repeatedly.

### Method
`self.trim`

### Parameters
#### Positional Parameters
- **pattern** (str or regex) - Optional - The pattern to trim. If `none`, whitespace is trimmed.
#### Named Parameters
- **at** (alignment) - Optional - Specifies which side to trim ('left', 'right', or 'both').
- **repeat** (bool) - Optional - If `true`, repeatedly trims the pattern. Defaults to `false`.

### Response
- Returns the trimmed string.
```

--------------------------------

### Alignment Definitions

Source: https://typst.app/docs/reference/layout/alignment

Explains how to access definitions associated with alignment functions and types.

```APIDOC
## Definitions

Functions and types can have associated definitions. These are accessed by specifying the function or type, followed by a period, and then the definition's name.

### `axis` Definition

The axis this alignment belongs to.

* `"horizontal"` for `start`, `left`, `center`, `right`, and `end`
* `"vertical"` for `top`, `horizon`, and `bottom`
* `none` for 2-dimensional alignments

#### Request Example

```typc
#left.axis()
#bottom.axis()
```

### `inv` Definition

The inverse alignment.

#### Request Example

```typc
#top.inv()
#left.inv()
#center.inv()
#(left + bottom).inv()
```
```

--------------------------------

### Basic Strike Element Usage

Source: https://typst.app/docs/reference/text/strike

Demonstrates the basic usage of the strike element to cross out text. This is the simplest way to apply a strike-through effect.

```typst
This is #strike[not] relevant.

```

--------------------------------

### Typst Float Literals and Expressions

Source: https://typst.app/docs/reference/foundations/float

Demonstrates various ways to represent floating-point numbers in Typst, including decimal notation, scientific notation, and arithmetic expressions.

```typst
#3.14 \
#1e4 \
#(10 / 4)
```

--------------------------------

### Typst Attach Function for Detailed Control

Source: https://typst.app/docs/reference/math/attach

Illustrates the `attach` function for precise control over top, bottom, top-left, top-right, bottom-left, and bottom-right attachments to a base element.

```typst
$ attach(
  Pi, t: alpha, b: beta,
  tl: 1, tr: 2+3, bl: 4+5, br: 6,
) $
```

--------------------------------

### h Element - Fractional Spacing

Source: https://typst.app/docs/reference/layout/h

Illustrates how fractional spacing can be used to align elements within a line without forcing a paragraph break.

```APIDOC
## `h` Element - Fractional Spacing

### Description
With fractional spacing, you can align things within a line without forcing a paragraph break (like `align` would). Each fractionally sized element gets space based on the ratio of its fraction to the sum of all fractions.

### Method
Not applicable (element function)

### Endpoint
Not applicable (element function)

### Parameters
#### Positional Parameters
- **amount** (relativefraction) - Required - How much spacing to insert, specified as a fraction (e.g., `1fr`).

### Request Example
```typst
First #h(1fr) Second
First #h(1fr) Second #h(1fr) Third
First #h(2fr) Second #h(1fr) Third
```

### Response
Not applicable (element function)
```

--------------------------------

### Customizing hline Stroke in Typst Table

Source: https://typst.app/docs/reference/model/table

Demonstrates how to set a custom stroke for all horizontal lines in a table using `#set table.hline` and then drawing specific lines with `table.hline`.

```typst
#set table.hline(stroke: .6pt)

#table(
  stroke: none,
  columns: (auto, 1fr),
  [09:00], [Badge pick up],
  [09:45], [Opening Keynote],
  [10:30], [Talk: Typst's Future],
  [11:15], [Session: Good PRs],
  table.hline(start: 1),
  [Noon], [_Lunch break_],
  table.hline(start: 1),
  [14:00], [Talk: Tracked Layout],
  [15:00], [Talk: Automations],
  [16:00], [Workshop: Tables],
  table.hline(),
  [19:00], [Day 1 Attendee Mixer],
)

```

--------------------------------

### Typst Page-Level Columns with Breakout

Source: https://typst.app/docs/reference/layout/columns

Illustrates how to set up page-level columns using the 'page' function and temporarily break out of this columnar layout for elements like titles using the 'place' function with 'scope: "parent"'.

```typst
#set page(columns: 2, height: 150pt)

#place(
  top + center,
  scope: "parent",
  float: true,
  text(1.4em, weight: "bold")[
    My document
  ],
)

#lorem(40)
```

--------------------------------

### Drawing a Cross with `cancel`

Source: https://typst.app/docs/reference/math/cancel

Demonstrates how to use the `cross` parameter to draw two opposing cancel lines, forming a cross over the element. This overrides the `inverted` setting.

```typst
$ cancel(Pi, cross: #true) $
```

--------------------------------

### Customizing Smallcaps Fonts

Source: https://typst.app/docs/reference/text/smallcaps

Explains how to customize the font used for smallcaps when the default OpenType features are not supported or when using dedicated smallcaps fonts.

```APIDOC
## Smallcaps fonts

By default, this uses the `smcp` and `c2sc` OpenType features on the font. Not all fonts support these features. Sometimes, smallcaps are part of a dedicated font. This is, for example, the case for the _Latin Modern_ family of fonts. In those cases, you can use a show-set rule to customize the appearance of the text in smallcaps:

```ty
#show smallcaps: set text(font: "Latin Modern Roman Caps")
```

In the future, this function will support synthesizing smallcaps from normal letters, but this is not yet implemented.
```

--------------------------------

### Create a table row

Source: https://typst.app/docs/reference/html/typed

Use the `tr` function to create a table row. It takes content as its parameter.

```typst
html.tr(content)
```

--------------------------------

### Create a Table Footer in Typst

Source: https://typst.app/docs/reference/html/typed

Use `html.tfoot` to group footer rows within a table. It accepts content as its parameter.

```typst
html.tfoot(
content
)
```

--------------------------------

### Customizing `overline` Stroke and Offset

Source: https://typst.app/docs/reference/text/overline

Shows how to customize the `stroke` color and `offset` of the overline. The `stroke` can be a color, and `offset` adjusts its vertical position relative to the baseline.

```typst
#set text(fill: olive)
#overline(
  stroke: green.darken(20%),
  offset: -12pt,
  [The Forest Theme],
)

```

--------------------------------

### Create a Table Body in Typst

Source: https://typst.app/docs/reference/html/typed

Use `html.tbody` to group rows within a table. It accepts content as its parameter.

```typst
html.tbody(
content
)
```

--------------------------------

### Match Pattern in String

Source: https://typst.app/docs/reference/foundations/str

The `match` method searches for the first occurrence of a pattern (string or regex) and returns a dictionary containing match details like start/end offsets, matched text, and capturing groups. Returns `none` if no match is found.

```typst
#let pat = regex("not (a|an) (apple|cat)")
#"I'm a doctor, not an apple.".match(pat)
#"I am not a cat!".match(pat)
```

```typst
#assert.eq("Is there a".match("for this?"), none)
#"The time of my life.".match(regex("[mit]+e"))
```

--------------------------------

### Integer Definitions

Source: https://typst.app/docs/reference/foundations/int

Explains the available definitions for the integer type, including `signum`, `bit-not`, `bit-and`, `bit-or`, `bit-xor`, `bit-lshift`, and `bit-rshift`.

```APIDOC
## Definitions
Functions and types can have associated definitions. These are accessed by specifying the function or type, followed by a period, and then the definition's name.

### `signum`
Calculates the sign of an integer.
  * If the number is positive, returns `1`.
  * If the number is negative, returns `-1`.
  * If the number is zero, returns `0`.

#### Example
```
#(5).signum() 
#(-5).signum() 
#(0).signum()
```

### Signature
`self.signum() -> int`

### `bit-not`
Calculates the bitwise NOT of an integer.
For the purposes of this function, the operand is treated as a signed integer of 64 bits.

#### Example
```
#4.bit-not() 
#(-1).bit-not()
```

### Signature
`self.bit-not() -> int`

### `bit-and`
Calculates the bitwise AND between two integers.
For the purposes of this function, the operands are treated as signed integers of 64 bits.

#### Example
```
#128.bit-and(192)
```

### Signature
`self.bit-and(rhs: int) -> int`

#### `rhs`
int
Required Positional
Positional parameters are specified in order, without names. The right-hand operand of the bitwise AND.

### `bit-or`
Calculates the bitwise OR between two integers.
For the purposes of this function, the operands are treated as signed integers of 64 bits.

#### Example
```
#64.bit-or(32)
```

### Signature
`self.bit-or(rhs: int) -> int`

#### `rhs`
int
Required Positional
Positional parameters are specified in order, without names. The right-hand operand of the bitwise OR.

### `bit-xor`
Calculates the bitwise XOR between two integers.
For the purposes of this function, the operands are treated as signed integers of 64 bits.

#### Example
```
#64.bit-xor(96)
```

### Signature
`self.bit-xor(rhs: int) -> int`

#### `rhs`
int
Required Positional
Positional parameters are specified in order, without names. The right-hand operand of the bitwise XOR.

### `bit-lshift`
Shifts the operand's bits to the left by the specified amount.
For the purposes of this function, the operand is treated as a signed integer of 64 bits. An error will occur if the result is too large to fit in a 64-bit integer.

#### Example
```
#33.bit-lshift(2) 
#(-1).bit-lshift(3)
```

### Signature
`self.bit-lshift(shift: int) -> int`

#### `shift`
int
Required Positional
Positional parameters are specified in order, without names. The amount of bits to shift. Must not be negative.

### `bit-rshift`
Shifts the operand's bits to the right by the specified amount. Performs an arithmetic shift by default (extends the sign bit to the left, such that negative numbers stay negative), but that can be changed by the `logical` parameter.
For the purposes of this function, the operand is treated as a signed integer of 64 bits.

#### Example
```
#64.bit-rshift(2) 
#(-8).bit-rshift(2) 
#(-8).bit-rshift(2, logical: true)
```

### Signature
`self.bit-rshift(shift: int, logical: bool = false) -> int`

#### `shift`
int
Required Positional
Positional parameters are specified in order, without names. The amount of bits to shift. Must not be negative.
Shifts larger than 63 are allowed and will cause the return value to saturate. For non-negative numbers, the return value saturates at `0`, while, for negative numbers, it saturates at `-1` if `logical` is set to `false`, or `0` if it is `true`. This behavior is consistent with just applying this operation multiple times. Therefore, the shift will always succeed.
```

--------------------------------

### Create an Abbreviation

Source: https://typst.app/docs/reference/html/typed

Use the `html.abbr` function to create an abbreviation element. It accepts content as its only parameter, which represents the text of the abbreviation.

```typst
html.abbr(
  content
) -> content
```

--------------------------------

### Create a Textarea Element in Typst

Source: https://typst.app/docs/reference/html/typed

Use `html.textarea` for multiline text input. It accepts numerous attributes for form control and content.

```typst
html.textarea(
autocomplete: strarray,
cols: int,
dirname: str,
disabled: bool,
form: str,
maxlength: int,
minlength: int,
name: str,
placeholder: str,
readonly: bool,
required: bool,
rows: int,
wrap: str,
content,
)
```

--------------------------------

### HTML Kbd Element

Source: https://typst.app/docs/reference/html/typed

Represents user input, typically keyboard input.

```APIDOC
## HTML Kbd Element

### Description
Represents user input.

### Method
html.kbd

### Parameters
#### Path Parameters
None

#### Query Parameters
None

#### Request Body
- **body** (content) - Required - Positional - The contents of the HTML element.

### Request Example
```json
{
  "content": "Ctrl + C"
}
```

### Response
#### Success Response (200)
- **content** (content) - The rendered HTML content.

#### Response Example
```json
{
  "example": "<kbd>Ctrl + C</kbd>"
}
```
```

--------------------------------

### HTML Article (article) Element

Source: https://typst.app/docs/reference/html/typed

Documentation for the `html.article` function, used to create article elements.

```APIDOC
## `article` - Self-contained Composition

### Description
Creates an article element for self-contained, syndicatable, or reusable compositions.

### Method
`html.article`

### Parameters
#### Path Parameters
None

#### Query Parameters
None

#### Request Body
None

#### Attributes
- **content** (content) - Required - The contents of the HTML element.

### Request Example
```typc
#html.article()[
  <h1>Article Title</h1>
  <p>Article content...</p>
]
```

### Response
#### Success Response (200)
- **content** (content) - The rendered HTML article element.
```

--------------------------------

### Typst: Combining Selectors with 'and'

Source: https://typst.app/docs/reference/foundations/selector

Demonstrates the use of the `and` method to combine selectors in Typst. This method selects elements that match all of the specified selectors.

```typst
selector1.and(selector2, selector3)

```

--------------------------------

### h Element - Basic Usage

Source: https://typst.app/docs/reference/layout/h

Demonstrates inserting horizontal spacing into a paragraph using absolute, relative, or fractional amounts.

```APIDOC
## `h` Element - Basic Usage

### Description
Inserts horizontal spacing into a paragraph. The spacing can be absolute, relative, or fractional. In the last case, the remaining space on the line is distributed among all fractional spacings according to their relative fractions.

### Method
Not applicable (element function)

### Endpoint
Not applicable (element function)

### Parameters
#### Positional Parameters
- **amount** (relativefraction) - Required - How much spacing to insert. Can be absolute (e.g., `1cm`), relative (e.g., `30%`), or fractional (e.g., `1fr`).

#### Settable Parameters
- **weak** (bool) - Optional - If `true`, the spacing collapses at the start or end of a paragraph. Moreover, from multiple adjacent weak spacings all but the largest one collapse. Default: `false`.

### Request Example
```typst
First #h(1cm) Second
First #h(30%) Second
```

### Response
Not applicable (element function)
```

--------------------------------

### Customizing Block Quote Appearance

Source: https://typst.app/docs/reference/model/quote

Shows how to customize the appearance of block quotes using `set` and `show` rules to control alignment and padding.

```typst
#set quote(block: true)
#show quote: set align(center)
#show quote: set pad(x: 5em)

#quote[
  You cannot pass... I am a servant of the Secret Fire, wielder of the
  flame of Anor. You cannot pass. The dark fire will not avail you,
  flame of Udûn. Go back to the Shadow! You cannot pass.
]
```

--------------------------------

### Create a Table Cell in Typst

Source: https://typst.app/docs/reference/html/typed

Use `html.td` to create a table data cell. It supports `colspan`, `headers`, `rowspan`, and content.

```typst
html.td(
colspan: int,
headers: strarray,
rowspan: int,
content,
)
```

--------------------------------

### Custom Page Footer with Page Numbering

Source: https://typst.app/docs/reference/layout/page

Creates a custom footer that displays the current page number in a specific format ('1 of I'). It also sets text size and right-aligns the content.

```typst
#set par(justify: true)
#set page(
  height: 100pt,
  margin: 20pt,
  footer: context [
    #set align(right)
    #set text(8pt)
    #counter(page).display(
      "1 of I",
      both: true,
    )
  ]
)

#lorem(48)

```

--------------------------------

### Selecting Symbol Variants in Typst

Source: https://typst.app/docs/reference/foundations/symbol

Shows how to select different variants of a symbol by appending modifiers with dot notation. The order of modifiers does not matter.

```typst
$arrow.l$ \
$arrow.r$ \
$arrow.t.quad$
```

--------------------------------

### HTML Anchor (a) Element

Source: https://typst.app/docs/reference/html/typed

Documentation for the `html.a` function, used to create hyperlinks.

```APIDOC
## `a` - Hyperlink

### Description
Creates a hyperlink element.

### Method
`html.a`

### Parameters
#### Path Parameters
None

#### Query Parameters
None

#### Request Body
None

#### Attributes
- **download** (str) - Optional - Whether to download the resource instead of navigating to it, and its filename if so.
- **href** (str) - Required - Address of the hyperlink.
- **hreflang** (str) - Optional - Language of the linked resource.
- **ping** (str or array) - Optional - URLs to ping.
- **referrerpolicy** (none or str) - Optional - Referrer policy for fetches initiated by the element. Possible values: `"no-referrer"`, `"no-referrer-when-downgrade"`, `"same-origin"`, `"origin"`, `"strict-origin"`, `"origin-when-cross-origin"`, `"strict-origin-when-cross-origin"`, `"unsafe-url"`.
- **rel** (str or array) - Optional - Relationship between the location in the document containing the hyperlink and the destination resource. Possible values: `"alternate"`, `"canonical"`, `"author"`, `"bookmark"`, `"dns-prefetch"`, `"expect"`, `"external"`, `"help"`, `"icon"`, `"manifest"`, `"modulepreload"`, `"license"`, `"next"`, `"nofollow"`, `"noopener"`, `"noreferrer"`, `"opener"`, `"pingback"`, `"preconnect"`, `"prefetch"`, `"preload"`, `"prev"`, `"privacy-policy"`, `"search"`, `"stylesheet"`, `"tag"`, `"terms-of-service"`.
- **target** (str) - Optional - Navigable for hyperlink navigation. Possible values: `"_blank"`, `"_self"`, `"_parent"`, `"_top"`.
- **type** (str) - Optional - Hint for the type of the referenced resource.
- **content** (content) - Required - The contents of the HTML element.

### Request Example
```typc
#html.a(href: "https://example.com")[
  Visit Example
]
```

### Response
#### Success Response (200)
- **content** (content) - The rendered HTML anchor element.
```

--------------------------------

### Typst: Combining Selectors with 'or'

Source: https://typst.app/docs/reference/foundations/selector

Shows how to use the `or` method to combine multiple selectors in Typst. This allows selecting elements that match any of the provided selectors.

```typst
selector1.or(selector2, selector3)

```

--------------------------------

### Gradient Information Methods

Source: https://typst.app/docs/reference/visualize/gradient

Methods to retrieve information about the gradient's properties.

```APIDOC
## Gradient Information Methods

### `kind`

- **Description**: Returns the kind of this gradient.
- **Method**: `kind`
- **Returns**: `function`

### `stops`

- **Description**: Returns the stops of this gradient.
- **Method**: `stops`
- **Returns**: `array`

### `space`

- **Description**: Returns the mixing space of this gradient.
- **Method**: `space`
- **Returns**: `any`

### `relative`

- **Description**: Returns the relative placement of this gradient.
- **Method**: `relative`
- **Returns**: `autostr`

### `angle`

- **Description**: Returns the angle of this gradient. Returns `none` if the gradient is neither linear nor conic.
- **Method**: `angle`
- **Returns**: `noneangle`

### `center`

- **Description**: Returns the center of this gradient. Returns `none` if the gradient is neither radial nor conic.
- **Method**: `center`
- **Returns**: `nonearray`

### `radius`

- **Description**: Returns the radius of this gradient. Returns `none` if the gradient is not radial.
- **Method**: `radius`
- **Returns**: `noneratio`

### `focal-center`

- **Description**: Returns the focal-center of this gradient. Returns `none` if the gradient is not radial.
- **Method**: `focal-center`
- **Returns**: `nonearray`

### `focal-radius`

- **Description**: Returns the focal-radius of this gradient. Returns `none` if the gradient is not radial.
- **Method**: `focal-radius`
- **Returns**: `noneratio`
```

--------------------------------

### Language-Aware Quotes with Smartquote

Source: https://typst.app/docs/reference/text/smartquote

Demonstrates how the smartquote element automatically adapts to different language settings for quotation marks. Ensure the text language is set correctly for the desired output.

```typst
"This is in quotes."

#set text(lang: "de")
"Das ist in Anführungszeichen."

#set text(lang: "fr")
"C'est entre guillemets."
```

--------------------------------

### Styling `overline` with `background` Parameter

Source: https://typst.app/docs/reference/text/overline

Shows how the `background` parameter affects the `overline`'s placement relative to the content. `background: true` places the line behind the text, while `false` (default) places it in front.

```typst
#set overline(stroke: (thickness: 1em, paint: maroon, cap: "round"))
#overline(background: true)[This is stylized.] 
#overline(background: false)[This is partially hidden.]

```

--------------------------------

### Adjusting Text Layout Parameters

Source: https://typst.app/docs/changelog/0.12.0

The `text.costs` parameter allows for fine-tuning various settings that influence the layout engine's decisions during text layout.

```typst
text.costs
```

--------------------------------

### Enum Element Parameters

Source: https://typst.app/docs/reference/model/enum

Details the settable parameters for the `enum` element.

```APIDOC
## Enum Element Parameters

### Description
Parameters are the inputs to a function. They are specified in parentheses after the function name.

```typ
enum(
  tight: bool,
  numbering: str | function,
  start: auto | int,
  full: bool,
  reversed: bool,
  indent: length,
  body-indent: length,
  spacing: auto | length,
  number-align: alignment,
  ..content: array,
) -> content
```

### `tight`
- **Type**: bool
- **Settable**: Yes
- **Description**: Defines the default spacing of the enumeration. If it is `false`, the items are spaced apart with paragraph spacing. If it is `true`, they use paragraph leading instead. This makes the list more compact, which can look better if the items are short. In markup mode, the value of this parameter is determined based on whether items are separated with a blank line. If items directly follow each other, this is set to `true`; if items are separated by a blank line, this is set to `false`. The markup-defined tightness cannot be overridden with set rules.

### `numbering`
- **Type**: str or function
- **Settable**: Yes
- **Description**: How to number the enumeration. Accepts a numbering pattern or function. If the numbering pattern contains multiple counting symbols, they apply to nested enums. If given a function, the function receives one argument if `full` is `false` and multiple arguments if `full` is `true`.

### `start`
- **Type**: auto or int
- **Settable**: Yes
- **Description**: Which number to start the enumeration with.

### `full`
- **Type**: bool
- **Settable**: Yes
- **Description**: Whether to display the full numbering, including the numbers of all parent enumerations.

### `reversed`
- **Type**: bool
- **Settable**: Yes
- **Description**: Whether to reverse the numbering for this enumeration.

### `indent`
- **Type**: length
- **Settable**: Yes
- **Description**: The indentation of each item.

### `body-indent`
- **Type**: length
- **Settable**: Yes
- **Description**: The space between the numbering and the body of each item.

### `spacing`
- **Type**: auto or length
- **Settable**: Yes
- **Description**: The spacing between the items of the enumeration. If set to `auto`, uses paragraph `leading` for tight enumerations and paragraph `spacing` for wide (non-tight) enumerations.

### `number-align`
- **Type**: alignment
- **Settable**: Yes
- **Description**: The alignment of the numbering relative to the item body.
```

--------------------------------

### Reversed Enum Order

Source: https://typst.app/docs/reference/model/enum

Demonstrates reversing the order of enumeration items using the `reversed` parameter.

```typst
#set enum(reversed: true)
+ Coffee
+ Tea
+ Milk
```

--------------------------------

### Add Bibliography to Typst Document

Source: https://typst.app/docs/tutorial/writing-in-typst

Demonstrates how to include a bibliography in a Typst document using the `bibliography` function. It specifies the path to the bibliography file (e.g., 'works.bib') and shows how to cite sources using the '@' syntax. This is useful for backing up claims in reports and academic papers.

```typst
= Methods
We follow the glacier melting models
established in @glacier-melt.

#bibliography("works.bib")


```

--------------------------------

### Create an HTML form element

Source: https://typst.app/docs/reference/html/typed

Use this function to create an HTML `<form>` element for user-submittable data. It accepts numerous attributes including `accept-charset`, `action`, `autocomplete`, `enctype`, `method`, `name`, `novalidate`, `rel`, `target`, and content.

```typst
html.form(
accept-charset: str,action: str,autocomplete: bool,enctype: str,method: str,name: str,novalidate: bool,rel: strarray,target: str,content,
)
```

--------------------------------

### Typst Relative Path for Images

Source: https://typst.app/docs/reference/syntax

Demonstrates how to specify a relative path for including an image in Typst. Relative paths are resolved from the location of the Typst file where the feature is invoked.

```typst
#image("images/logo.png")

```

--------------------------------

### Typst LaTeX Look Configuration

Source: https://typst.app/docs/guides/for-latex-users

Configures Typst document settings to emulate the appearance of LaTeX documents. This includes setting page margins, paragraph spacing, line leading, justification, first-line indent, and font.

```typst
#set page(margin: 1.75in)
#set par(leading: 0.55em, spacing: 0.55em, first-line-indent: 1.8em, justify: true)
#set text(font: "New Computer Modern")
#show raw: set text(font: "New Computer Modern Mono")
#show heading: set block(above: 1.4em, below: 1em)
```

--------------------------------

### Controlling `cancel` Line Angle

Source: https://typst.app/docs/reference/math/cancel

Explains and shows how to control the rotation of the cancel line using the `angle` parameter, including fixed angles and dynamic adjustments based on a function.

```typst
$ cancel(Pi)
  cancel(Pi, angle: #0deg)
  cancel(Pi, angle: #45deg)
  cancel(Pi, angle: #90deg)
  cancel(1/(1+x), angle: #(a => a + 45deg))
  cancel(1/(1+x), angle: #(a => a + 90deg)) $
```

--------------------------------

### Predefined Colors

Source: https://typst.app/docs/reference/visualize/color

Typst includes a set of predefined colors that can be used directly.

```APIDOC
## Predefined Colors
Typst defines the following built-in colors:
Color| Definition  
---|---
`black`| `luma(0)`  
`gray`| `luma(170)`  
`silver`| `luma(221)`  
`white`| `luma(255)`  
`navy`| `rgb("#001f3f")`  
`blue`| `rgb("#0074d9")`  
`aqua`| `rgb("#7fdbff")`  
`teal`| `rgb("#39cccc")`  
`eastern`| `rgb("#239dad")`  
`purple`| `rgb("#b10dc9")`  
`fuchsia`| `rgb("#f012be")`  
`maroon`| `rgb("#85144b")`  
`red`| `rgb("#ff4136")`  
`orange`| `rgb("#ff851b")`  
`yellow`| `rgb("#ffdc00")`  
`olive`| `rgb("#3d9970")`  
`green`| `rgb("#2ecc40")`  
`lime`| `rgb("#01ff70")`  

The predefined colors and the most important color constructors are available globally and also in the color type's scope, so you can write either `color.red` or just `red`.
```

--------------------------------

### Create Custom HTML Element with Typed API

Source: https://typst.app/docs/changelog/0.14.0

The `html.elem` function now supports custom HTML element names, allowing for greater flexibility in generating HTML.

```typst
#html.elem("custom-element", attrs: (some-attr: "value"))
```

--------------------------------

### Add Alt Text to Images in Typst

Source: https://typst.app/docs/guides/accessibility

This snippet demonstrates how to use the `alt` argument within Typst's `image` function to provide a textual description for an image. This is crucial for accessibility, allowing screen readers to convey the image's content to users who cannot see it. The alt text should be concise and relevant to the image's context.

```typst
#image("heron.jpg", alt: "Heron in flight with feet and wings spread")
```

--------------------------------

### Global Attribute: accesskey

Source: https://typst.app/docs/reference/html/typed

The `accesskey` global attribute assigns a keyboard shortcut to an element, allowing for quick activation or focus. It expects a single-codepoint string or an array of such strings.

```typst
accesskey: str or array
```

--------------------------------

### Typst Math: Augmented Matrices

Source: https://typst.app/docs/changelog/0.8.0

Demonstrates the support for augmented matrices in Typst's math typesetting, which are commonly used in linear algebra.

```typst
// Augmented matrix example
#matrix(columns: 2, "a", "b", "c", "d") \
#vrule()
#matrix(columns: 1, "e", "f")
```

--------------------------------

### Parameter Details: leading

Source: https://typst.app/docs/reference/model/par

Detailed explanation of the `leading` parameter for the `par` element, which controls the spacing between lines.

```APIDOC
### `leading` Parameter

#### Description

The `leading` parameter specifies the spacing between lines within a paragraph. It defines the distance between the bottom edge of one line and the top edge of the following line. This can be used to achieve consistent baseline-to-baseline distances.

#### Type

`length`

#### Settable

Yes

#### Default Value

`0.65em`

#### Example Usage

```typst
#set par(leading: 1em)
```
```

--------------------------------

### Create an HTML div element

Source: https://typst.app/docs/reference/html/typed

Use this function to create a generic flow container `<div>` or a container for name-value groups in `dl` elements. It takes content as a positional parameter.

```typst
html.div(
content
)
```

--------------------------------

### Styling Inline and Block Raw Elements

Source: https://typst.app/docs/reference/text/raw

Applies distinct styling to inline and block raw elements using `box.with` to customize their appearance.

```typst
#show raw.where(block: false): box.with(
  fill: luma(240),
  inset: (x: 3pt, y: 0pt),
  outset: (y: 3pt),
  radius: 2pt,
)

// Display block code in a larger block
// with more padding.
#show raw.where(block: true): block.with(
  fill: luma(240),
  inset: 10pt,
  radius: 4pt,
)

With `rg`, you can search through your files quickly.
This example searches the current directory recursively
for the text `Hello World`:

```bash
rg "Hello World"
```

```

--------------------------------

### Typst Conference Paper Template with Show Rule

Source: https://typst.app/docs/tutorial/making-a-template

Presents a Typst conference template function 'conf' that configures page layout, paragraph justification, and text styles. It uses an 'everything' show rule with a closure to pass a title and the document body to the template function.

```typst
#let conf(title, doc) = {
  set page(
    paper: "us-letter",
    header: align(
      right + horizon,
      title
    ),
    columns: 2,
    ...
  )
  set par(justify: true)
  set text(
    font: "Libertinus Serif",
    size: 11pt,
  )

  // Heading show rules.
  ...

  doc
}

#show: doc => conf(
  [Paper title],
  doc,
)

= Introduction
...

```

--------------------------------

### Typst Code Mode Syntax Elements

Source: https://typst.app/docs/reference/syntax

This snippet outlines the various syntactic elements available in Typst's code mode, including basic data types, operators, control flow statements, and module inclusion/importing. It serves as a reference for writing Typst code.

```typst
none
auto
false, true
10, 0xff
3.14, 1e5
2pt, 3mm, 1em
90deg, 1rad
2fr
50%
"hello"
<intro>
$x^2$
`print(1)`
x
{ let x = 1; x + 2 }
[*Hello*]
(1 + 2)
(1, 2, 3)
(a: "hi", b: 2)
-x
x + y
x = 1
x.y
x.flatten()
min(x, y)
min(..nums)
(x, y) => x + y
let x = 1
let f(x) = 2 * x
set text(14pt)
set text(..) if .. 
show heading: set block(..)
show raw: it => {..}
show: template
context text.lang
if x == 1 {..} else {..}
for x in (1, 2, 3) {..}
while x < 10 {..}
break, continue
return x
include "bar.typ"
import "bar.typ"
import "bar.typ": a, b, c
/* block */, // line
```

--------------------------------

### Additional Parentheses and Shell Shapes

Source: https://typst.app/docs/changelog/0.12.0

The functions `underparen`, `overparen`, `undershell`, and `overshell` have been added for creating various enclosing shapes around mathematical expressions.

```typst
underparen
```

```typst
overparen
```

```typst
undershell
```

```typst
overshell
```

--------------------------------

### Array Definitions (Methods)

Source: https://typst.app/docs/reference/foundations/array

Details the various methods available for array manipulation, including length, first/last element access, indexing, insertion, removal, and slicing.

```APIDOC
## Array Definitions (Methods)

### `len()`
Returns the number of values in the array.

**Syntax:** `self.len() -> int`

### `first()`
Returns the first item in the array. Can be used for assignment. Returns a default value if the array is empty or an error if no default is specified.

**Syntax:** `self.first(default: any) -> any`

#### `default`
- **default** (any) - A default value to return if the array is empty.

### `last()`
Returns the last item in the array. Can be used for assignment. Returns a default value if the array is empty or an error if no default is specified.

**Syntax:** `self.last(default: any) -> any`

#### `default`
- **default** (any) - A default value to return if the array is empty.

### `at()`
Returns the item at the specified index. Can be used for assignment. Returns a default value if the index is out of bounds or an error if no default is specified.

**Syntax:** `self.at(index: int, default: any) -> any`

#### `index`
- **index** (int) - Required Positional - The index at which to retrieve the item. Negative indexes count from the back.
#### `default`
- **default** (any) - A default value to return if the index is out of bounds.

### `push()`
Adds a value to the end of the array.

**Syntax:** `self.push(value: any)`

#### `value`
- **value** (any) - Required Positional - The value to insert at the end of the array.

### `pop()`
Removes the last item from the array and returns it. Fails if the array is empty.

**Syntax:** `self.pop() -> any`

### `insert()`
Inserts a value into the array at the specified index, shifting subsequent elements. Fails if the index is out of bounds.

**Syntax:** `self.insert(index: int, value: any)`

#### `index`
- **index** (int) - Required Positional - The index at which to insert the item. Negative indexes count from the back.
#### `value`
- **value** (any) - Required Positional - The value to insert into the array.

### `remove()`
Removes the value at the specified index from the array and returns it.

**Syntax:** `self.remove(index: int, default: any) -> any`

#### `index`
- **index** (int) - Required Positional - The index at which to remove the item. Negative indexes count from the back.
#### `default`
- **default** (any) - A default value to return if the index is out of bounds.

### `slice()`
Extracts a subslice of the array. Fails if start or end indices are out of bounds.

**Syntax:** `self.slice(start: int, end: none | int, count: int) -> array`

#### `start`
- **start** (int) - Required Positional - The start index (inclusive). Negative indexes count from the back.
#### `end`
- **end** (none | int) - Positional - The end index (exclusive). If omitted, slices to the end. Negative indexes count from the back. Default: `none`.
#### `count`
- **count** (int) - The number of items to extract. Equivalent to `start + count` as `end`. Mutually exclusive with `end`.
```

--------------------------------

### Typst Circle Element Creation

Source: https://typst.app/docs/reference/visualize/circle

Demonstrates creating a basic circle and a circle with content in Typst. The `circle` element can be customized with parameters such as `radius`, `width`, `height`, `fill`, and `stroke`. Content within the circle can be styled using alignment rules.

```typst
#circle(radius: 25pt)

// With content.
#circle[
  #set align(center + horizon)
  Automatically \ 
  sized to fit.
]
```

--------------------------------

### Use Bounds Option for Tight Bounding Boxes in Text

Source: https://typst.app/docs/changelog/0.7.0

The `bounds` option for `top-edge` and `bottom-edge` arguments in the `text` function creates tight bounding boxes. This is useful for precise layout control.

```typst
text(top-edge: "bounds")
```

--------------------------------

### Path Fill Rule Variants

Source: https://typst.app/docs/reference/visualize/path

Illustrates the usage of the `fill-rule` parameter with different values for the `path` function. The `path` function is deprecated; use `curve` instead.

```APIDOC
## `path` with `fill-rule`

### Description
Demonstrates how to use the `fill-rule` parameter with the `path` function to control how the path is filled. The `path` function is deprecated; use `curve` instead.

### Method
Function Call

### Endpoint
N/A (Client-side function)

### Parameters
- **fill-rule** (str) - Optional - The drawing rule used to fill the path. Variants: `"non-zero"`, `"even-odd"`. Default: `"non-zero"`.

### Request Example
```typc
// We use `.with` to get a new
// function that has the common
// arguments pre-applied.
#let star = path.with(
  fill: red,
  closed: true,
  (25pt, 0pt),
  (10pt, 50pt),
  (50pt, 20pt),
  (0pt, 20pt),
  (40pt, 50pt),
)

#star(fill-rule: "non-zero")
#star(fill-rule: "even-odd")
```

### Response
#### Success Response (Content)
- **content** (content) - The rendered path element with the specified fill rule.

#### Response Example
(No direct response example, as it's a rendering function)

### Deprecation Note
The `path` function is deprecated. Use the `curve` function instead.
```

--------------------------------

### Typst: Add Alternative Description to Math Equation

Source: https://typst.app/docs/guides/accessibility

This code snippet demonstrates how to add an alternative description to a math equation in Typst using the `math.equation` function. This is crucial for accessibility, especially for PDF/UA-1 export, ensuring screen readers can interpret the equation.  The `alt` argument provides the description, and `block: true` formats the equation as a block.

```Typst
#math.equation(
  alt: "a squared plus b squared equals c squared",
  block: true,
  $ a^2 + b^2 = c^2 $,
)
```

--------------------------------

### math.inline

Source: https://typst.app/docs/reference/math/sizes

Forces inline (text) style in math, the normal size for inline equations.

```APIDOC
## math.inline

### Description
Forced inline (text) style in math. This is the normal size for inline equations.

### Signature
`math.inline(content, cramped: bool)`

### Parameters
#### `body`
- **content** (content) - Required Positional - The content to size.
#### `cramped`
- **cramped** (bool) - Optional - Whether to impose a height restriction for exponents, like regular sub- and superscripts do. Default: `false`

### Request Example
```json
{
  "content": "sum_i x_i/2",
  "cramped": false
}
```

### Response
#### Success Response (200)
- **content** (content) - The sized content.

#### Response Example
```json
{
  "content": "inline(sum_i x_i/2)"
}
```
```

--------------------------------

### Typst: Using 'before' for Temporal Selection

Source: https://typst.app/docs/reference/foundations/selector

Illustrates the `before` method for creating a selector that matches elements occurring before a specified end point. The `inclusive` parameter controls whether the end element itself is included in the match.

```typst
self.before(end: <label>, inclusive: true)

```

--------------------------------

### oklab Function

Source: https://typst.app/docs/reference/visualize/color

Creates an Oklab color. This color space is suitable for color manipulation and creating smooth gradients.

```APIDOC
## `oklab`
Create an Oklab color.
This color space is well suited for the following use cases:
  * Color manipulation such as saturating while keeping perceived hue
  * Creating grayscale images with uniform perceived lightness
  * Creating smooth and uniform color transition and gradients


A linear Oklab color is represented internally by an array of four components:
  * lightness (`ratio`)
  * a (`float` or `ratio`. Ratios are relative to `0.4`; meaning `50%` is equal to `0.2`)
  * b (`float` or `ratio`. Ratios are relative to `0.4`; meaning `50%` is equal to `0.2`)
  * alpha (`ratio`)


These components are also available using the `components` method.
```
#square(
  fill: oklab(27%, 20%, -3%, 50%)
)
```

color.oklab(
ratio,floatratio,floatratio,ratio,color,
) -> color
#### `lightness`
ratio
Required Positional
Positional parameters are specified in order, without names. 
The lightness component.
#### `a`
float or ratio
Required Positional
Positional parameters are specified in order, without names. 
The a ("green/red") component.
#### `b`
float or ratio
Required Positional
Positional parameters are specified in order, without names. 
The b ("blue/yellow") component.
```

--------------------------------

### Typst Math: Line Breaks and Alignment Points

Source: https://typst.app/docs/reference/math

Shows how to create line breaks within mathematical formulas in Typst using '\' and how to define alignment points using '&' for aligning equations across multiple lines.

```typst
$ sum_(k=0)^n k
    &= 1 + ... + n \
    &= (n(n+1)) / 2 $
```

--------------------------------

### Create an HTML figure element

Source: https://typst.app/docs/reference/html/typed

Use this function to create an HTML `<figure>` element, which can contain a figure with an optional caption. It takes content as a positional parameter.

```typst
html.figure(
content
)
```

--------------------------------

### Typst: Two-Column Layout with Floating Abstract

Source: https://typst.app/docs/tutorial/advanced-styling

Configures a Typst document for a two-column layout using the `page` set rule. It includes an abstract that floats above the main content, using the `place` function with `float: true` and `scope: "parent"`.

```typst
#set page(
  paper: "us-letter",
  header: align(
    right + horizon,
    context document.title,
  ),
  numbering: "1",
  columns: 2,
)

#place(
  top + center,
  float: true,
  scope: "parent",
  clearance: 2em,
)[
  ... 

  #par(justify: false)[
    *Abstract* \ 
    #lorem(80)
  ]
]

= Introduction
#lorem(300)

= Related Work
#lorem(200)

```

--------------------------------

### Bytes Constructor

Source: https://typst.app/docs/reference/foundations/bytes

Converts a value to bytes. Strings are encoded in UTF-8. Arrays of integers between 0 and 255 are converted directly.

```APIDOC
## bytes Constructor

### Description
Converts a value to bytes.
* Strings are encoded in UTF-8.
* Arrays of integers between `0` and `255` are converted directly. The dedicated byte representation is much more efficient than the array representation and thus typically used for large byte buffers (e.g. image data).

### Syntax
```
bytes(strbytesarray)
```

### Parameters
#### `value`
- **value** (str or bytes or array) - Required Positional - The value that should be converted to bytes.
```

--------------------------------

### Create an rp Element

Source: https://typst.app/docs/reference/html/typed

Use html.rp to define parenthesis for ruby annotation text. It takes content as its only parameter.

```typst
html.rp(content,)
```

--------------------------------

### Calculate Arcsine with calc.asin

Source: https://typst.app/docs/reference/foundations/calc

Use calc.asin to find the arcsine of a number. The input value must be between -1 and 1.

```typst
#calc.asin(0) 
#calc.asin(1)
```

--------------------------------

### Default Outline Entry Formatting

Source: https://typst.app/docs/reference/model/outline

The default `show outline.entry` rule formats entries by linking to the element's location and applying indentation. It uses helper functions `prefix` and `inner` to construct the entry.

```typst
#show outline.entry: it => link(
  it.element.location(),
  it.indented(it.prefix(), it.inner()),
)
```

--------------------------------

### Skew with Different Origins

Source: https://typst.app/docs/reference/layout/skew

Illustrates how the `origin` parameter affects the skew transformation by fixing different points as the origin for the skew.

```typst
X #box(skew(ax: -30deg, origin: center + horizon)[X]) X 
X #box(skew(ax: -30deg, origin: bottom + left)[X]) X 
X #box(skew(ax: -30deg, origin: top + right)[X]) X

```

--------------------------------

### Outline Function Parameters

Source: https://typst.app/docs/reference/model/outline

Customizable parameters for the outline function, including depth and indentation.

```APIDOC
## `depth`

### Description
The maximum level up to which elements are included in the outline. When this argument is `none`, all elements are included.

### Settable
Yes

### Default
`none`

### Example
```ty
#set heading(numbering: "1.")
#outline(depth: 2)

= Yes
Top-level section.

== Still
Subsection.

=== Nope
Not included.
```

## `indent`

### Description
How to indent the outline's entries. Can be `auto`, a relative length, or a function.

### Settable
Yes

### Default
`auto`

### Example
```ty
#set heading(numbering: "I-I.")
#set outline(title: none)

#outline() // Default auto indent
#line(length: 100%)
#outline(indent: 3em) // Custom indent

= Software engineering technologies
== Requirements
== Tools and technologies
=== Code editors
== Analyzing alternatives
= Designing software components
= Testing and integration
```
```

--------------------------------

### Assert equality between two values

Source: https://typst.app/docs/reference/foundations/assert

Use `assert.eq` to verify that two values are equal. An optional message can be provided for failure cases. This function does not produce any output.

```typst
#assert.eq(10, 10)
```

--------------------------------

### State At Method

Source: https://typst.app/docs/reference/introspection/state

Retrieves the value of the state at a specific location in the document.

```APIDOC
## state.at(selector)

### Description
Retrieves the value of the state at the location specified by the selector. The selector must uniquely match one element in the document.

### Parameters
#### `selector` (label or selector or location or function) - Required
The selector specifying the location at which to retrieve the state's value.

### Request Example
```typst
#let my_state = state("data", "initial")

#context my_state.at(label("my_label"))
```

### Response Example
(Returns the value of the state at the specified location)
```

--------------------------------

### Global Attributes

Source: https://typst.app/docs/reference/html/typed

Common parameters applicable to all typed HTML functions for consistent styling and accessibility.

```APIDOC
## Global Attributes

### Description
These parameters are common to all typed HTML functions. They are listed here once instead of explicitly on each element for readability.

### Parameters
#### Path Parameters
- **accesskey** (str or array) - Optional - Keyboard shortcut to activate or focus element. Expects a single-codepoint string or an array thereof.
- **aria-activedescendant** (str) - Optional - Identifies the currently active element when DOM focus is on a composite widget, textbox, group, or application.
- **aria-atomic** (bool) - Optional - Indicates whether assistive technologies will present all, or only parts of, the changed region based on the change notifications defined by the aria-relevant attribute.
- **aria-autocomplete** (none or str) - Optional - Indicates whether inputting text could trigger display of one or more predictions of the user's intended value for an input and specifies how predictions would be presented if they are made. Variants: `"inline"`, `"list"`, `"both"`.
- **aria-busy** (bool) - Optional - Indicates an element is being modified and that assistive technologies MAY want to wait until the modifications are complete before exposing them to the user.
- **aria-checked** (bool or str) - Optional - Indicates the current "checked" state of checkboxes, radio buttons, and other widgets. See related aria-pressed and aria-selected. Variant: `"mixed"`.
```

--------------------------------

### Set Language for Raw Code Blocks

Source: https://typst.app/docs/reference/text/raw

Use the `lang` parameter to specify the language for syntax highlighting. Supports standard language tags and Typst-specific tags like 'typ', 'typc', and 'typm'.

```typ
#set raw(lang: "typ")
This is *Typst!*

```

```typ
This is ```typ also *Typst*```, but inline!

```

--------------------------------

### Set Uniform Gap with `gap`

Source: https://typst.app/docs/reference/math/mat

Sets a uniform gap between rows and columns using the `gap` parameter. This is a shorthand for setting `row-gap` and `column-gap` to the same value. Requires a `set` rule for global application.

```typst
#set math.mat(gap: 1em)
$ mat(1, 2; 3, 4) $
```

--------------------------------

### Iframe Element

Source: https://typst.app/docs/reference/html/typed

Documentation for the iframe element in Typst.

```APIDOC
### `iframe`
- **Description**: Child navigable.
- **Syntax**: `html.iframe(allow: str, allowfullscreen: bool, height: int, loading: str, name: str, referrerpolicy: nonestr, sandbox: strarray, src: str, srcdoc: str, width: int, content)
- **Parameters**:
  - **allow** (string) - Permissions policy to be applied to the iframe's contents.
  - **allowfullscreen** (boolean) - Whether to allow the iframe's contents to use requestFullscreen().
  - **height** (integer) - Vertical dimension.
  - **loading** (string) - Used when determining loading deferral.
    - **Variants**: `"lazy"`, `"eager"`
  - **name** (string) - Name of content navigable.
    - **Variants**: `"_blank"`, `"_self"`, `"_parent"`, `"_top"`
  - **referrerpolicy** (none or string) - Referrer policy for fetches initiated by the element.
    - **Variants**: `"no-referrer"`, `"no-referrer-when-downgrade"`, `"same-origin"`, `"origin"`, `"strict-origin"`, `"origin-when-cross-origin"`, `"strict-origin-when-cross-origin"`, `"unsafe-url"`
  - **sandbox** (string array) - Sandbox restrictions for the iframe.
  - **src** (string) - The URL of the content to embed.
  - **srcdoc** (string) - An HTML document to embed.
  - **width** (integer) - Horizontal dimension.
  - **content** - Positional. The contents of the HTML element.
```

--------------------------------

### Force Second Script Style in Math

Source: https://typst.app/docs/reference/math/sizes

Use the `sscript` function for the smallest size, used in second-level sub/superscripts. It takes content and an optional `cramped` boolean, defaulting to true.

```typst
$sum_i x_i/2 = sscript(sum_i x_i/2)$
```

--------------------------------

### Underbracket Element with Annotation in Typst

Source: https://typst.app/docs/reference/math/underover

Use the `underbracket` function to create a bracket under content, with an optional annotation below the bracket. The annotation is the second argument.

```typst
$ underbracket(0 + 1 + dots.c + n, n + 1 "numbers") $
```

--------------------------------

### Use Callable Symbols for Floor and Ceil in Math

Source: https://typst.app/docs/changelog/0.12.0

The `floor` and `ceil` functions in math mode can now be used as callable symbols, allowing for a more direct syntax when typesetting these operations.

```typst
$ floor(x) = lr(floor.l x floor.r) $
```

--------------------------------

### Set CJK-Latin Spacing

Source: https://typst.app/docs/reference/text/text

Demonstrates enabling and disabling automatic spacing between CJK and Latin characters. 'auto' inserts spacing, while 'none' prevents it.

```typst
#set text(cjk-latin-spacing: auto)
第4章介绍了基本的API。

#set text(cjk-latin-spacing: none)
第4章介绍了基本的API。
```

--------------------------------

### Controlling Strike Line Extent

Source: https://typst.app/docs/reference/text/strike

Demonstrates how to control the length of the strike line relative to the content using the extent parameter. Negative values shorten the line, while positive values extend it.

```typst
This #strike(extent: -2pt)[skips] parts of the word.
This #strike(extent: 2pt)[extends] beyond the word.

```

--------------------------------

### Set US Letter Page Size

Source: https://typst.app/docs/reference/layout/page

Applies the US Letter page size. This is a shorthand for setting width and height.

```typst
#set page("us-letter")

There you go, US friends!

```

--------------------------------

### Text Layout Costs Configuration

Source: https://typst.app/docs/reference/text/text

Adjust the 'costs' associated with different text layout choices, such as hyphenation, runts, widows, and orphans. Higher costs make the layout engine less likely to make that choice.

```APIDOC
## SET TEXT COSTS

### Description
Customizes the 'cost' of various text layout decisions, influencing the layout engine's choices. Costs are relative to the default. Affects hyphenation, runts (single words at line end), widows (single lines at paragraph end), and orphans (single lines at paragraph start).

### Method
SET

### Parameters
#### Settable Parameters
- **costs** (dictionary) - Settable - A dictionary specifying costs for layout choices. Keys can include `hyphenation`, `runt`, `widow`, `orphan`. Values are percentages (e.g., `50%`, `200%`).

### Request Example
```ty
#set text(costs: (hyphenation: 1000%))
```

### Response
This is a settable parameter. Modifies the behavior of the text layout engine regarding line and paragraph breaks.
```

--------------------------------

### Pad Element API

Source: https://typst.app/docs/reference/layout/pad

Documentation for the `pad` element, which adds spacing around content. It supports individual side padding and shorthand options.

```APIDOC
## `pad` Element

### Description
Adds spacing around content. The spacing can be specified for each side individually, or for all sides at once by specifying a positional argument.

### Method
Not Applicable (This is a function/element, not an HTTP endpoint)

### Endpoint
Not Applicable

### Parameters
#### Positional Parameters
- **content** (content) - Required - The content to pad at the sides.

#### Named Parameters
- **left** (relative) - Optional - The padding at the left side. Default: `0% + 0pt`
- **top** (relative) - Optional - The padding at the top side. Default: `0% + 0pt`
- **right** (relative) - Optional - The padding at the right side. Default: `0% + 0pt`
- **bottom** (relative) - Optional - The padding at the bottom side. Default: `0% + 0pt`
- **x** (relative) - Optional - A shorthand to set `left` and `right` to the same value. Default: `0% + 0pt`
- **y** (relative) - Optional - A shorthand to set `top` and `bottom` to the same value. Default: `0% + 0pt`
- **rest** (relative) - Optional - A shorthand to set all four sides to the same value. Default: `0% + 0pt`

### Request Example
```typst
#set align(center)

#pad(x: 16pt, image("typing.jpg"))
_Typing speeds can be
measured in words per minute._
```

### Response
#### Success Response (200)
- **content** (content) - The padded content.

#### Response Example
```typst
#pad(10pt, "Hello, World!")
```
```

--------------------------------

### Typst: Store Document Metadata with Document Element

Source: https://typst.app/docs/tutorial/advanced-styling

Explains how to use the `#set document()` rule to store document-level metadata such as the title. This metadata can then be referenced elsewhere in the document.

```typst
#set document(title: [A Fluid Dynamic Model for Glacier Flow])

```

--------------------------------

### Create an output Element

Source: https://typst.app/docs/reference/html/typed

Use html.output to display a calculated output value. It can be linked to other form controls using the 'for' attribute.

```typst
html.output(for: strarray,form: str,name: str,content,)
```

--------------------------------

### Typst Scripting: First-Class Types and Method Syntax

Source: https://typst.app/docs/changelog/0.8.0

Demonstrates the new first-class type system in Typst, where types are values and methods are syntactic sugar for scoped functions. This includes type checking with `type(value) == type_name` and method calls like `type_name.method(value)`.

```typst
#let x = 10
#let t = type(x)

// Type check
#assert(t == int)

// Method call equivalent
#assert(str.len("hello") == "hello".len())
```

--------------------------------

### Custom Star Shape with Fill Rules

Source: https://typst.app/docs/reference/visualize/curve

Defines a reusable `star` function using `curve.with` to pre-apply common arguments. It then shows how to use this star with different `fill-rule` values (`non-zero` and `even-odd`).

```typst
#let star = curve.with(
  fill: red,
  curve.move((25pt, 0pt)),
  curve.line((10pt, 50pt)),
  curve.line((50pt, 20pt)),
  curve.line((0pt, 20pt)),
  curve.line((40pt, 50pt)),
  curve.close(),
)

#star(fill-rule: "non-zero")
#star(fill-rule: "even-odd")

```

--------------------------------

### Force Inline Style in Math

Source: https://typst.app/docs/reference/math/sizes

Use the `inline` function for normal size in inline equations. It takes content and an optional `cramped` boolean.

```typst
$ sum_i x_i/2
    = inline(sum_i x_i/2) $
```

--------------------------------

### Predefined Color Maps

Source: https://typst.app/docs/reference/visualize/color

Typst provides preset color maps for use in gradients, available in the `color.map` module.

```APIDOC
## Predefined Color Maps
Typst also includes a number of preset color maps that can be used for gradients. These are simply arrays of colors defined in the module `color.map`.
```
#circle(fill: gradient.linear(..color.map.crest))
```

Map| Details  
---|---
`turbo`| A perceptually uniform rainbow-like color map. Read this blog post for more details.
`cividis`| A blue to gray to yellow color map. See this blog post for more details.
`rainbow`| Cycles through the full color spectrum. This color map is best used by setting the interpolation color space to HSL. The rainbow gradient is **not suitable** for data visualization because it is not perceptually uniform, so the differences between values become unclear to your readers. It should only be used for decorative purposes.
`spectral`| Red to yellow to blue color map.
`viridis`| A purple to teal to yellow color map.
`inferno`| A black to red to yellow color map.
`magma`| A black to purple to yellow color map.
`plasma`| A purple to pink to yellow color map.
`rocket`| A black to red to white color map.
`mako`| A black to teal to white color map.
`vlag`| A light blue to white to red color map.
`icefire`| A light teal to black to orange color map.
`flare`| A orange to purple color map that is perceptually uniform.
`crest`| A light green to blue color map.

Some popular presets are not included because they are not available under a free licence. Others, like Jet, are not included because they are not color blind friendly. Feel free to use or create a package with other presets that are useful to you!
```

--------------------------------

### direction.sign()

Source: https://typst.app/docs/reference/layout/direction

The `sign` definition returns the corresponding integer sign for a direction, useful in calculations.

```APIDOC
## direction.sign()

### Description
The corresponding sign, for use in calculations.

### Syntax
`self.sign() -> int`

### Usage Examples
```
#ltr.sign()
#rtl.sign()
#ttb.sign()
#btt.sign()
```
```

--------------------------------

### Heading Element Parameters

Source: https://typst.app/docs/reference/model/heading

Details the settable parameters for the heading element, including level, depth, offset, numbering, and outline behavior.

```APIDOC
## Parameters

`heading(`
level: autoint,
depth: int,
offset: int,
numbering: nonestrfunction,
supplement: noneautocontentfunction,
outlined: bool,
bookmarked: autobool,
hanging-indent: autolength,
content,
) -> content

### `level`
auto or int
Settable
The absolute nesting depth of the heading, starting from one. If set to `auto`, it is computed from `offset + depth`.

Default: `auto`

### `depth`
int
Settable
The relative nesting depth of the heading, starting from one. This is combined with `offset` to compute the actual `level`.

Default: `1`

### `offset`
int
Settable
The starting offset of each heading's `level`, used to turn its relative `depth` into its absolute `level`.

```typ
= Level 1

#set heading(offset: 1, numbering: "1.1")
= Level 2

#heading(offset: 2, depth: 2)[I'm level 4]
```

Default: `0`

### `numbering`
none or str or function
Settable
How to number the heading. Accepts a numbering pattern or function taking multiple numbers.

```typ
#set heading(numbering: "1.a.")

= A section
== A subsection
=== A sub-subsection
```

Default: `none`

### `outlined`
bool
Settable
Whether the heading should be included in the document outline. Set to `false` to exclude.

Default: `true`

### `bookmarked`
autobool
Settable
Whether the heading should generate a bookmark for navigation.

Default: `auto`

### `hanging-indent`
autolength
Settable
Configures the hanging indent for the heading text.

Default: `auto`

### `supplement`
none or autocontentfunction
Settable
Used for numbering and referencing, typically for elements like figures or tables that might follow a heading.

Default: `none`
```

--------------------------------

### Create an optgroup Element

Source: https://typst.app/docs/reference/html/typed

Use html.optgroup to create a group of options in a list box. The disabled parameter can be used to disable the group.

```typst
html.optgroup(disabled: bool,label: str,content,)
```

--------------------------------

### Typst Math: Flexible `op` Function

Source: https://typst.app/docs/changelog/0.10.0

The `op` function in Typst math can now accept any content, not just strings, providing greater flexibility in defining operators and symbols.

```typst
op
```

--------------------------------

### Insert Image using Typst

Source: https://typst.app/docs/tutorial/writing-in-typst

Demonstrates the basic usage of the `image` function in Typst to insert an image file into a document. It requires the path to the image file as a string argument.

```typst
#image("glacier.jpg")
```

--------------------------------

### Create a Conic Gradient

Source: https://typst.app/docs/reference/visualize/gradient

Use gradient.conic to create a color transition changing radially around a center point. It accepts color stops, angle, color space, relative placement, and center coordinates.

```typst
circle(fill: gradient.conic(
    ..color.map.viridis,
  )),
  circle(fill: gradient.conic(
    ..color.map.viridis,
    center: (20%, 30%),
  ))

```

--------------------------------

### Textarea Element

Source: https://typst.app/docs/reference/html/typed

A multiline text input control.

```APIDOC
## `textarea`

### Description
Multiline text controls.

### Method
html.textarea

### Parameters
#### Path Parameters
- **autocomplete** (strarray) - Optional - Hint for form autofill feature. See HTML spec for variants.
- **cols** (int) - Optional - Maximum number of characters per line.
- **dirname** (str) - Optional - Name of form control to use for sending the element's directionality in form submission.
- **disabled** (bool) - Optional - Whether the form control is disabled.
- **form** (str) - Optional - Associates the element with a form element.
- **maxlength** (int) - Optional - Maximum length of value.
- **minlength** (int) - Optional - Minimum length of value.
- **name** (str) - Optional - Name of the element to use for form submission and in the form.elements API.
- **placeholder** (str) - Optional - User-visible label to be placed within the form control.
- **readonly** (bool) - Optional - Whether to allow the value to be edited by the user.
- **required** (bool) - Optional - Whether the control is required for form submission.
- **rows** (int) - Optional - Number of lines to show.
- **wrap** (str) - Optional - How the value of the form control is to be wrapped for form submission. Variants: "soft", "hard".

#### Positional Parameters
- **content** (any) - The contents of the HTML element.

### Response Example
```json
{
  "example": "textarea content"
}
```
```

--------------------------------

### Typst Math: Multi-letter Variables and Summation

Source: https://typst.app/docs/tutorial/writing-in-typst

Explains how to handle multi-letter variables in Typst math mode by enclosing them in quotes (e.g., `"time offset"`). It also demonstrates typesetting a summation formula using the `sum` symbol, specifying the range with sub- and superscripts, and handling fractions with the `/` operator. Parentheses are automatically resolved for complex expressions.

```typst
The flow rate of a glacier is given
by the following equation:

$ Q = rho A v + "time offset" $

Total displaced soil by glacial flow:

$ 7.32 beta +
  sum_(i=0)^nabla Q_i / 2 $


```

--------------------------------

### Dynamic CV Grid Layout

Source: https://typst.app/docs/reference/layout/grid

Generates a dynamic CV layout using a grid, with custom strokes between years and a header. The `cv` function processes job data to create the grid content.

```typst
#set page(height: 13em, width: 26em)

#let cv(..jobs) = grid(
  columns: 2,
  inset: 5pt,
  stroke: (x, y) => if x == 0 and y > 0 {
    (right: (
      paint: luma(180),
      thickness: 1.5pt,
      dash: "dotted",
    ))
  },
  grid.header(grid.cell(colspan: 2)[
    *Professional Experience*
    #box(width: 1fr, line(length: 100%, stroke: luma(180)))
  ]),
  ..{
    let last = none
    for job in jobs.pos() {
      (
        if job.year != last [*#job.year*],
        [
          *#job.company* - #job.role _(#job.timeframe)_ \
          #job.details
        ]
      )
      last = job.year
    }
  }
)

#cv(
  (
    year: 2012,
    company: [Pear Seed & Co.],
    role: [Lead Engineer],
    timeframe: [Jul - Dec],
    details: [
      - Raised engineers from 3x to 10x
      - Did a great job
    ],
  ),
  (
    year: 2012,
    company: [Mega Corp.],
    role: [VP of Sales],
    timeframe: [Mar - Jun],
    details: [- Closed tons of customers],
  ),
  (
    year: 2013,
    company: [Tiny Co.],
    role: [CEO],
    timeframe: [Jan - Dec],
    details: [- Delivered 4x more shareholder value],
  ),
  (
    year: 2014,
    company: [Glorbocorp Ltd],
    role: [CTO],
    timeframe: [Jan - Mar],
    details: [- Drove containerization forward],
  ),
)
```

--------------------------------

### Render binomial expressions in Typst

Source: https://typst.app/docs/reference/math/binom

Demonstrates how to use the binom function to create standard and multi-index binomial expressions. The function accepts an upper index and one or more lower indices.

```typst
$ binom(n, k) $
$ binom(n, k_1, k_2, k_3, ..., k_m) $
```

```typst
math.binom(content, ..content)
```

--------------------------------

### `smallcaps` with `all: true` Parameter

Source: https://typst.app/docs/reference/text/smallcaps

Use the `all: true` parameter to convert uppercase letters to small capitals as well. This enables the `c2sc` OpenType feature unless overridden by a show rule.

```typst
#smallcaps(all: true)[UNICEF] is an\nagency of #smallcaps(all: true)[UN].
```

--------------------------------

### Underbrace Element with Annotation in Typst

Source: https://typst.app/docs/reference/math/underover

Use the `underbrace` function to create a brace under content, with an optional annotation below the brace. The annotation is the second argument.

```typst
$ underbrace(0 + 1 + dots.c + n, n + 1 "numbers") $
```

--------------------------------

### Embed styling information

Source: https://typst.app/docs/reference/html/typed

Use `html.style` to embed styling information. It supports `blocking`, `media` attributes, and accepts content.

```typst
html.style(blocking: strarray,media: str,content)
```

--------------------------------

### Stylistic Alternates Configuration

Source: https://typst.app/docs/reference/text/text

Enables or disables the use of alternative glyphs for characters, controlled by the OpenType 'salt' font feature. This allows for font-specific stylistic variations.

```APIDOC
## `alternates`

### Description
Whether to apply stylistic alternates. Sometimes fonts contain alternative glyphs for the same codepoint. Setting this to `true` switches to these by enabling the OpenType `salt` font feature.

### Method
SET

### Parameters
#### Request Body
- **alternates** (bool) - Settable - Whether to apply stylistic alternates.

### Request Example
```json
{
  "alternates": true
}
```

### Response
#### Success Response (200)
- **alternates** (bool) - The current stylistic alternates setting.

#### Response Example
```json
{
  "alternates": true
}
```
```

--------------------------------

### Attachment Syntax

Source: https://typst.app/docs/reference/math/attach

Typst provides dedicated syntax for attachments using underscore (_) for subscripts and hat (^) for superscripts.

```APIDOC
## Syntax

This function also has dedicated syntax for attachments after the base: Use the underscore (`_`) to indicate a subscript i.e. bottom attachment and the hat (`^`) to indicate a superscript i.e. top attachment.

### Example
```
$ sum_(i=0)^n a_i = 2^(1+i) $
```
```

--------------------------------

### Duration Constructor

Source: https://typst.app/docs/reference/foundations/duration

Creates a new duration by specifying time units or by subtracting two datetimes.

```APIDOC
## Constructor duration

Creates a new duration. You can specify the duration using weeks, days, hours, minutes and seconds. You can also get a duration by subtracting two datetimes.

### Parameters
#### Path Parameters
- **seconds** (int) - Optional - The number of seconds. Default: `0`
- **minutes** (int) - Optional - The number of minutes. Default: `0`
- **hours** (int) - Optional - The number of hours. Default: `0`
- **days** (int) - Optional - The number of days. Default: `0`
- **weeks** (int) - Optional - The number of weeks. Default: `0`

### Request Example
```typ
#duration(
  days: 3,
  hours: 12,
).hours()
```
```

--------------------------------

### Disable Math Highlighting

Source: https://typst.app/docs/changelog/0.12.0

The `raw.theme` parameter can be set to `none` to disable syntax highlighting for raw code blocks, even if a language tag is present. Setting it to `auto` reverts to the default highlighting behavior.

```typst
raw.theme: none
```

```typst
raw.theme: auto
```

--------------------------------

### Typst Math: Accessible Equation with Alt Text

Source: https://typst.app/docs/reference/math

Demonstrates how to provide alternative text descriptions for mathematical equations in Typst using the `alt` parameter within `math.equation`. This is crucial for accessibility in exported documents.

```typst
#math.equation(
  alt: "d S equals delta q divided by T",
  block: true,
  $ dif S = (delta q) / T $
)
```

--------------------------------

### Typst: Bulleted and Nested Lists

Source: https://typst.app/docs/tutorial/writing-in-typst

Demonstrates how to create bulleted lists using the '-' character and how to nest lists within each other by using indentation. This allows for hierarchical organization of list items.

```typst
+ The climate
  - Temperature
  - Precipitation
+ The topography
+ The geology

```

--------------------------------

### Split String by Pattern

Source: https://typst.app/docs/reference/foundations/str

Use `split` to divide a string into an array of substrings based on a specified pattern. The pattern can be a string or a regular expression. If the pattern is an empty string, it splits at each Unicode code point.

```typst
self.split(
nonestrregex
) -> array
```