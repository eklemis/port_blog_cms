#!/usr/bin/env python3
"""
fsd-guard — a Claude Code PostToolUse hook for blogport_frontend.

Reads the hook payload on stdin, inspects the file that was just written, and
exits 2 with an explanation on stderr when a house rule is broken. Exit code 2
is the one the harness feeds back to the agent, so a violation becomes something
it must fix rather than something it is merely asked not to do.

Rules, in the order they fire:
  1  Svelte 4 syntax          — export let / $: / svelte/store in a component
  2  Tailwind 4               — no tailwind.config.*, no raw hex, no arbitrary colour
  3  Feature-Sliced Design    — layer order, no same-layer imports, public API only
  4  TDD                      — a new component needs its spec to exist first

Wire it up in .claude/settings.json (PostToolUse, matcher "Write|Edit").
Run it directly to check the whole tree:  python3 scripts/fsd-guard.py --all
"""

import json
import os
import re
import sys
from pathlib import Path

FRONTEND = Path(__file__).resolve().parent.parent
SRC = FRONTEND / "src"

# FSD layers, highest first. Index = rank; a layer may only import a HIGHER rank.
LAYERS = ["app", "pages", "widgets", "features", "entities", "shared"]
RANK = {name: i for i, name in enumerate(LAYERS)}

HEX = re.compile(r"#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})\b")
ARBITRARY_COLOUR = re.compile(r"(?:text|bg|border|fill|stroke|ring|from|to|via)-\[(?:#|rgb|hsl|oklch)")
EXPORT_LET = re.compile(r"^\s*export\s+let\s+", re.M)
REACTIVE = re.compile(r"^\s*\$:\s", re.M)
STORE_IMPORT = re.compile(r"""from\s+['"]svelte/store['"]""")
# svelte.config.js sets files.lib='src' AND alias '@/*' -> 'src/*', so BOTH
# `$lib/entities/blog` and `@/entities/blog` resolve. Check both, or a violation
# just gets written with the other prefix.
IMPORTS = re.compile(r"""(?:from|import)\s+['"]((?:@|\$lib)/[^'"]+)['"]""")


def layer_and_slice(path: Path):
    """('entities', 'blog') for src/entities/blog/ui/card.svelte, else (None, None)."""
    try:
        rel = path.relative_to(SRC).parts
    except ValueError:
        return None, None
    if not rel or rel[0] not in RANK:
        return None, None
    layer = rel[0]
    if layer in ("app", "shared"):
        return layer, None          # these two are not sliced
    return layer, (rel[1] if len(rel) > 1 else None)


def check(path: Path, text: str, tool: str = "Write"):
    problems = []
    name = path.name
    rel = path.relative_to(FRONTEND) if FRONTEND in path.parents else path
    layer, slice_ = layer_and_slice(path)
    is_component = path.suffix == ".svelte"
    in_routes = "app/routes" in path.as_posix()

    # ---- 2a. Tailwind 4 is CSS-first ---------------------------------------
    if name.startswith("tailwind.config"):
        problems.append(
            "Tailwind 4 is CSS-first — there is no tailwind.config file. "
            "Put theme values in `@theme` inside CSS. Delete this file."
        )

    # ---- 1. Svelte 5 runes only --------------------------------------------
    if is_component or path.suffixes[-2:] == [".svelte", ".ts"]:
        if EXPORT_LET.search(text):
            problems.append(
                "Svelte 4 props found (`export let`). Use runes:\n"
                "    let { post, onsave }: { post: Post; onsave: (p: Post) => void } = $props();"
            )
        if REACTIVE.search(text):
            problems.append(
                "Svelte 4 reactive statement found (`$:`). Use `$derived(...)` "
                "for values, `$effect(() => ...)` for side effects."
            )
        if STORE_IMPORT.search(text):
            problems.append(
                "`svelte/store` in a component. Shared reactive state belongs in a "
                "`.svelte.ts` module using `$state`, not a store."
            )

    # ---- 2b. Colour comes from tokens --------------------------------------
    # src/app/app.css is where the tokens are DEFINED — the one file whose job is
    # to hold raw hex. Everywhere else, a hex literal is a colour that escaped the
    # system and will not theme.
    TOKEN_SOURCES = {"app.css", "theme.css"}
    if path.suffix in (".svelte", ".css", ".ts") and "docs/" not in path.as_posix():
        if name not in TOKEN_SOURCES:
            if HEX.search(text):
                found = ", ".join(sorted(set(HEX.findall(text)))[:4])
                problems.append(
                    f"Raw hex colour ({found}). Every colour resolves through the theme "
                    "tokens — use `bg-arch-surface`, `text-arch-muted`, "
                    "`border-arch-line-control`. See docs/theme.css."
                )
            if ARBITRARY_COLOUR.search(text):
                problems.append(
                    "Arbitrary Tailwind colour value (e.g. `text-[#8f5000]`). "
                    "Use the arch-* token classes instead."
                )

    # ---- 3. Feature-Sliced Design ------------------------------------------
    if layer:
        for spec in IMPORTS.findall(text):
            bare = spec[5:] if spec.startswith("$lib/") else spec[2:]
            parts = bare.split("/")
            if not parts or parts[0] not in RANK:
                continue
            target_layer = parts[0]
            target_slice = parts[1] if len(parts) > 1 else None

            if RANK[target_layer] < RANK[layer]:
                problems.append(
                    f"FSD layer violation: `{layer}` imports from `{target_layer}`, "
                    f"which sits above it ({spec}). Allowed order, high to low: "
                    + " > ".join(LAYERS) + "."
                )
            elif target_layer == layer and slice_ and target_slice and target_slice != slice_:
                problems.append(
                    f"FSD cross-slice import: `{layer}/{slice_}` imports "
                    f"`{layer}/{target_slice}` ({spec}). Slices in one layer must not "
                    "know about each other — move the shared part down a layer."
                )
            elif target_layer not in ("app", "shared") and len(parts) > 2:
                problems.append(
                    f"FSD deep import: `{spec}` reaches past the slice's public API. "
                    f"Import `$lib/{target_layer}/{target_slice}` and re-export what "
                    f"you need from its index.ts."
                )

    # ---- 4. TDD: the spec exists before the component ----------------------
    # Only on Write (a new or rewritten component). Enforcing this on every Edit
    # would block all work on the legacy components that predate the rule, and a
    # guard that stops everything gets switched off. `--all` still reports them.
    if is_component and layer and not in_routes and tool == "Write":
        # vite.config.ts runs *.svelte.spec.ts in a real browser and *.spec.ts in
        # node, so a component's test must carry the .svelte.spec.ts name.
        candidates = [
            path.parent / f"{path.stem}.svelte.spec.ts",
            path.parent / f"{path.stem}.svelte.test.ts",
            path.with_suffix("").with_suffix(".spec.ts"),
            path.with_suffix("").with_suffix(".test.ts"),
        ]
        if not any(c.exists() for c in candidates):
            problems.append(
                f"No spec for this component. TDD is the rule here: write "
                f"`{candidates[0].name}` first, run it, watch it fail, then write "
                f"the component. A test that has never failed proves nothing."
            )

    return rel, problems


def report(results):
    bad = [(rel, ps) for rel, ps in results if ps]
    if not bad:
        return 0
    print("blogport_frontend house rules — see blogport_frontend/CLAUDE.md\n", file=sys.stderr)
    for rel, ps in bad:
        print(f"  {rel}", file=sys.stderr)
        for p in ps:
            first, *rest = p.split("\n")
            print(f"    - {first}", file=sys.stderr)
            for line in rest:
                print(f"      {line}", file=sys.stderr)
        print("", file=sys.stderr)
    return 2


def main():
    if "--all" in sys.argv:
        results = []
        for p in SRC.rglob("*"):
            if p.suffix in (".svelte", ".ts", ".css") and p.is_file():
                results.append(check(p, p.read_text(encoding="utf-8", errors="ignore")))
        sys.exit(report(results))

    try:
        payload = json.load(sys.stdin)
    except Exception:
        sys.exit(0)                       # not a hook invocation; stay out of the way

    fp = (payload.get("tool_input") or {}).get("file_path")
    if not fp:
        sys.exit(0)
    path = Path(fp)
    if not path.is_file() or FRONTEND not in path.parents:
        sys.exit(0)                       # not ours
    if path.suffix not in (".svelte", ".ts", ".css", ".js"):
        sys.exit(0)

    text = path.read_text(encoding="utf-8", errors="ignore")
    sys.exit(report([check(path, text, payload.get("tool_name", "Write"))]))


if __name__ == "__main__":
    main()
