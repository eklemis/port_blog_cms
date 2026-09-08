import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import SkeletonRows from './skeleton-rows.svelte';

/**
 * What a list looks like before it arrives.
 *
 * Rows shaped like real rows, never a centred spinner: a spinner says "wait"
 * and a skeleton says "a table is coming and it is about this big", which is
 * the difference between waiting and knowing.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

test('draws a page of rows, not one', async () => {
	render(SkeletonRows, { label: 'Loading posts' });

	// Six, because that is what a page of rows looks like before it arrives.
	expect(document.querySelectorAll('[data-skeleton-row]')).toHaveLength(6);
});

test('says what is loading, once, and only to a screen reader', async () => {
	// The shapes are the sighted answer; announcing them as well would be
	// saying the same thing twice.
	const screen = render(SkeletonRows, { label: 'Loading applications' });

	await expect.element(screen.getByRole('status')).toHaveTextContent('Loading applications');
});

test('the shapes themselves are decoration', async () => {
	// Nothing here is content. A screen reader walking the rows would read a
	// dozen empty boxes.
	render(SkeletonRows, { label: 'Loading posts' });

	for (const row of document.querySelectorAll('[data-skeleton-row]')) {
		expect(row.getAttribute('aria-hidden')).toBe('true');
	}
});

test('how many rows is the caller’s to say', async () => {
	render(SkeletonRows, { label: 'Loading topics', rows: 3 });

	expect(document.querySelectorAll('[data-skeleton-row]')).toHaveLength(3);
});

test('has no accessibility violations', async () => {
	render(SkeletonRows, { label: 'Loading posts' });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
