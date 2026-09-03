# 0009 — A reflection never reaches a prompt that writes for the user

**Status:** Accepted
**Component:** `career/`, and every future AI surface

## Context

After an unsuccessful application, the product asks three optional questions:
how far it got, what happened, and what the person would change. The answers are
the most sensitive data this product will hold — somebody's private account of
why they think they were rejected, written in the expectation that it is theirs.

They are also, on the face of it, the most useful thing in the database for
generating a better CV. That is exactly the problem. The obvious feature —
"tailor my CV using what I learned" — quietly turns a private note into an input
to text that goes to an employer, and the person who wrote it has no way to see
that happening.

## Decision

**A reflection is never included in any prompt that produces user-facing
content.** Not a CV bullet, not a cover letter, not a tailoring suggestion, not
an interview answer.

The single exception is a pattern summary the user explicitly asks for — "four
of your last six stalled after a take-home". That exception is subject to two
constraints:

1. **It is a distinct endpoint**, not a flag on an existing one. A boolean that
   switches sensitive data into a prompt is one careless default away from
   being on everywhere, and defaults drift. A separate route cannot be reached
   by accident.
2. **The user triggers it.** Never a side effect of opening a screen, never a
   background job, never part of another generation.

The frontend says so plainly in the interface either way. That sentence is not
decoration: being explicit about where this goes is the reason someone would
use the feature at all.

## Consequences

Some obviously desirable features are off the table. A tailoring pass that knows
"you keep losing at the take-home stage" would be better than one that does not,
and we are choosing not to build it. That is the cost, and it is worth paying:
the alternative is a product that reads a private note back to an employer, and
no amount of usefulness survives someone discovering that.

The schema does its share — reflections live in their own table, so nothing
reading an application picks them up incidentally, and no join added later
sweeps them into a context window by accident.

This is a rule about **prompts**, not about storage or display. The user can
read, edit and delete their own reflections freely; deletion is real deletion.

Anyone adding an AI surface should read this before deciding what goes into the
context. If a future feature genuinely needs it, that is a new decision to
record here — not a judgement call at the call site.
