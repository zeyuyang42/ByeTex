// Fixture body for the synthetic layout variants.
//
// COMMITTED. The PDFs built from it are NOT: they are a deterministic function
// of this file plus the typst version, so committing them would pin a version
// and rot. What is goldened is variants.json's `must_fire` / `must_not_fire`
// assertions — named properties, which are stable across typst versions.
// Goldening numeric profiles instead would reproduce Chromium's documented
// layout-tree-dump failure: baseline churn from semantic over-specification.
//
// This file NEVER changes between variants. Every knob a variant needs is a
// #let in the PREAMBLE, so a variant is exactly one preamble string and the
// content stays byte-identical unless the variant is explicitly about content.
// That is what makes "layout changed" and "text changed" separable, which is
// the whole orthogonality control.

= Introduction

#para(110)

== Method

#para(130)

#if showfig [
  #figure(
    rect(width: 60%, height: 2.4cm, stroke: 0.5pt),
    caption: [Schematic of the proposed method and its two stages.],
  ) <fig:method>
]

== Results

#para(120)

// Duplicated content: hurts ordered_precision while leaving recall intact.
#if dup [ #para(120) ]

// The small tier. A converter that flattens \small / \footnotesize into the
// body size leaves every other property untouched — that is a real, measured
// live bug (truth 17.8% of characters at 8.5pt, output 1.2%), and it is
// invisible to every metric the harness had before this one.
#small[#para(45)]

= Discussion

#para(115)

= References

#text(size: 9pt)[
  #for i in biborder [
    #refnum(i) #REFS.at(i - 1) \
  ]
]
