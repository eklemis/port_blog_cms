# Blogport — Design Lint

Six layout checks and one token check, run over the frames on a Figma page.

## Why it exists

The tablet set was rebuilt three times. Three separate defects were reported by
a person **after an audit had called the same frames clean** — each time the fix
was to widen the check, not to patch the frame. These are those widened checks.

Every failure they catch is silent: Figma reports nothing, the file has no
errors, and the frame just looks wrong.

## Install

Figma → Plugins → Development → **Import plugin from manifest…** → pick
`manifest.json`. Then run it from Plugins → Development.

## The checks

| Check | Catches | Why it is not obvious |
| --- | --- | --- |
| Overflows its parent | Wider than the box holding it | Works whether or not the parent clips. A non-clipping card hid a 176px overrun from every clip-based check. |
| Outside a manual parent | Past the right edge of a `layoutMode: NONE` box | `layoutSizingHorizontal` is a no-op there — it succeeds and changes nothing, so a repair can look applied and not be. |
| Leaves the artboard | Extends past the frame | Pinned rails and modals that kept a desktop width. |
| Text squeezed | >25 chars in <110px | The one-word-per-line failure. Almost always a FILL child inside a HUG parent, which collapses to its widest hug-sized sibling. |
| Control squeezed | Non-icon instance <40px wide | A row that shared space equally instead of letting prose absorb it. |
| Siblings overlap | Vertical auto layout, child runs into the next | A manual container whose height was not recomputed after its text rewrapped. |
| Colour off-system | Solid paint with no bound variable | It will not follow the theme. Image paints are excluded — they bind to nothing by nature. |

## The exclusions are part of the rules

A check that fires on every tab bar gets switched off within a day. Each of
these is a false positive that was actually observed:

- **`icon/*` instances** are excluded from the squeezed-control check. Icons are
  meant to be 16–24px; counting them flagged 13 mobile frames whose only crime
  was having a tab bar.
- **Hidden nodes** are excluded everywhere. Collapsing the console sidebar to an
  icon rail hides its labels; they are 58px wide inside a 40px row and entirely
  correct.
- **Pinned (`ABSOLUTE`) nodes** are excluded from *overflows its parent* — they
  are positioned deliberately — but still checked against the artboard.
- **Image paints** are excluded from the colour check.

## Reading the results

Findings group by frame, worst first. Click one to select and zoom to the node —
that is the point of the plugin over a console script. **Copy report** gives
plain text for a PR comment.

Turn individual checks off in the disclosure at the top when you want to sweep
for one class of problem.

## Interpreting a clean result honestly

Clean means these seven rules found nothing. It does not mean the frame is
right: none of this checks that the design matches its spec, that copy is
correct, or that a layout reads well. It catches the mechanical failures that
survive review because nothing surfaces them.
