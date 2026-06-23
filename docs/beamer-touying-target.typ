#import "@preview/touying:0.7.3": *
#import themes.metropolis: *

#show: metropolis-theme.with(
  aspect-ratio: "16-9",
  config-info(
    title: [Scaling Laws for Efficient Neural Models],
    subtitle: [An Empirical Study],
    author: [Jane Researcher #h(1em) Sam Colleague],
    institution: [Example University],
    date: [June 2026],
  ),
)

#title-slide()

== Outline
#outline(title: none, indent: 1em)

= Motivation

== Why Scaling Laws?
- Compute budgets are growing fast.
- Predicting loss saves expensive runs.
- #text(fill: red)[Power laws] fit empirically across scales.
We study the regime $N in [10^6, 10^9]$.

= Method

== Loss Model
#cols[
  We fit
  $ L(N) = a N^(-alpha) + L_oo, $
  with $alpha approx 0.34$.
  #block(stroke: 0.5pt, inset: 6pt, radius: 2pt)[*Key Insight* \ Bigger models generalize better, predictably.]
][
  #table(columns: 2, align: (left, right),
    table.hline(), [Model], [Loss], table.hline(),
    [Small], [3.1], [Large], [2.4], table.hline())
]

= Conclusion

== Takeaways
#block(fill: green.lighten(80%), inset: 8pt, radius: 2pt, width: 100%)[*Result* \ A simple power law predicts loss within 5%.]
Thank you!
