import { expect } from 'vitest';
import axe, { type ElementContext, type RunOptions, type Result } from 'axe-core';

/**
 * Accessibility assertions for component specs.
 *
 * The Accessibility Specification is 53K of prose. Prose does not fail a build.
 * This does — it runs axe against the real rendered DOM in a real browser, so a
 * missing label or a broken aria reference stops the commit rather than being
 * caught in review, or not at all.
 *
 * What it CANNOT check, and why the spec still matters: axe finds roughly a
 * third of WCAG issues. It cannot tell you focus order is sensible, that an
 * error message is useful, that a live region announces at the right moment, or
 * that motion respects a preference. Those stay human judgement against the
 * spec — see docs/04-accessibility-spec.html §15.
 */

/** WCAG 2.2 Level AA — the standard this product targets, per the spec §01. */
const AA_TAGS = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

function format(violations: Result[]): string {
	return violations
		.map((v) => {
			const where = v.nodes
				.slice(0, 3)
				.map((n) => `        ${n.html}`)
				.join('\n');
			return [
				`  ✖ ${v.id} (${v.impact ?? 'unknown'} impact) — ${v.help}`,
				where,
				`        → ${v.helpUrl}`
			].join('\n');
		})
		.join('\n\n');
}

/**
 * Fails the test if axe finds any WCAG 2.2 AA violation in `context`.
 *
 * @param context  Defaults to the whole document. Pass an element to scope it.
 * @param options  Escape hatch for a rule that genuinely does not apply — e.g.
 *                 `{ rules: { 'color-contrast': { enabled: false } } }` when a
 *                 component is rendered without its real page background.
 *                 Disabling a rule needs a comment saying why.
 */
export async function expectNoA11yViolations(
	context: ElementContext = document.body,
	options: RunOptions = {}
): Promise<void> {
	const results = await axe.run(context, { runOnly: { type: 'tag', values: AA_TAGS }, ...options });

	// Must go through `expect`, not `throw`. `expect.requireAssertions` is on in
	// vite.config.ts, so a helper that throws on failure and returns silently on
	// success registers no assertion at all — and the PASSING case then fails the
	// suite with "expected any number of assertion, but got none".
	const report =
		results.violations.length === 0
			? ''
			: `\n\n${format(results.violations)}\n\n` +
				'See docs/04-accessibility-spec.html. Fix the markup; do not disable the\n' +
				'rule unless you can say in a comment why it does not apply here.\n';

	expect(report, 'WCAG 2.2 AA violations').toBe('');
}
