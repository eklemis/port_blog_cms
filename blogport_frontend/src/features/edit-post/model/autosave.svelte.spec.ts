import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { createAutosave } from './autosave.svelte';

/**
 * Saving as you go — J4 step three.
 *
 * "PATCH on a two-second idle debounce. The status line reads Saving… →
 * Saved 10:42 → Unsaved changes, and it is the only place that reports save
 * state." The fourth state is the branch below it: a failed save keeps the
 * buffer, says "Couldn't save — retrying", and retries with backoff.
 */

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

function autosave(save: () => Promise<boolean>) {
	return createAutosave({ save });
}

test('it starts settled, because nothing has been typed yet', () => {
	const saver = autosave(async () => true);

	expect(saver.state).toBe('saved');
	expect(saver.dirty).toBe(false);
});

test('typing says so immediately, and saves on a pause', async () => {
	// Immediately, because "Unsaved changes" is the honest state the moment a
	// keystroke lands — the two seconds are the debounce, not the truth.
	const save = vi.fn(async () => true);
	const saver = autosave(save);

	saver.edited();
	expect(saver.state).toBe('unsaved');
	expect(save).not.toHaveBeenCalled();

	await vi.advanceTimersByTimeAsync(2000);
	expect(save).toHaveBeenCalledTimes(1);
	expect(saver.state).toBe('saved');
});

test('a burst of keystrokes is one save, not one per key', async () => {
	const save = vi.fn(async () => true);
	const saver = autosave(save);

	saver.edited();
	await vi.advanceTimersByTimeAsync(500);
	saver.edited();
	await vi.advanceTimersByTimeAsync(500);
	saver.edited();
	await vi.advanceTimersByTimeAsync(2000);

	expect(save).toHaveBeenCalledTimes(1);
});

test('it says Saving while the call is in the air', async () => {
	let release: (ok: boolean) => void = () => {};
	const saver = autosave(() => new Promise<boolean>((resolve) => (release = resolve)));

	saver.edited();
	await vi.advanceTimersByTimeAsync(2000);
	expect(saver.state).toBe('saving');

	release(true);
	await vi.advanceTimersByTimeAsync(0);
	expect(saver.state).toBe('saved');
});

test('a save records when it happened, for the status line to name', async () => {
	// Epoch milliseconds, not a Date: a Date held in reactive state is a trap,
	// because mutating one in place changes nothing on screen.
	const saver = autosave(async () => true);

	saver.edited();
	await vi.advanceTimersByTimeAsync(2000);

	expect(typeof saver.savedAt).toBe('number');
});

test('typing during a save leaves the work unsaved, not falsely saved', async () => {
	// The in-flight call carries the older text. Calling it done would show
	// "Saved" over changes that are not.
	let release: (ok: boolean) => void = () => {};
	const saver = autosave(() => new Promise<boolean>((resolve) => (release = resolve)));

	saver.edited();
	await vi.advanceTimersByTimeAsync(2000);
	saver.edited();

	release(true);
	await vi.advanceTimersByTimeAsync(0);

	expect(saver.state).toBe('unsaved');
	expect(saver.dirty).toBe(true);
});

// ── when it fails ──────────────────────────────────────────────────────────

test('a failed save says so and keeps the buffer dirty', async () => {
	const saver = autosave(async () => false);

	saver.edited();
	await vi.advanceTimersByTimeAsync(2000);

	expect(saver.state).toBe('retrying');
	expect(saver.dirty).toBe(true);
});

test('it retries on a backoff rather than hammering', async () => {
	const save = vi.fn(async () => false);
	const saver = autosave(save);

	saver.edited();
	await vi.advanceTimersByTimeAsync(2000);
	expect(save).toHaveBeenCalledTimes(1);

	// One second, then two, then four.
	await vi.advanceTimersByTimeAsync(1000);
	expect(save).toHaveBeenCalledTimes(2);
	await vi.advanceTimersByTimeAsync(2000);
	expect(save).toHaveBeenCalledTimes(3);
	await vi.advanceTimersByTimeAsync(4000);
	expect(save).toHaveBeenCalledTimes(4);
});

test('a retry that works settles, and forgets the backoff', async () => {
	let works = false;
	const save = vi.fn(async () => works);
	const saver = autosave(save);

	saver.edited();
	await vi.advanceTimersByTimeAsync(2000);
	works = true;
	await vi.advanceTimersByTimeAsync(1000);

	expect(saver.state).toBe('saved');
	expect(saver.dirty).toBe(false);
});

// ── leaving ────────────────────────────────────────────────────────────────

test('flush saves now, for someone who is about to navigate away', async () => {
	const save = vi.fn(async () => true);
	const saver = autosave(save);

	saver.edited();
	await saver.flush();

	expect(save).toHaveBeenCalledTimes(1);
	expect(saver.state).toBe('saved');
});

test('flushing with nothing to save does not call the API', async () => {
	const save = vi.fn(async () => true);
	const saver = autosave(save);

	await saver.flush();

	expect(save).not.toHaveBeenCalled();
});

test('destroying stops the timers, so a closed editor saves nothing', async () => {
	const save = vi.fn(async () => true);
	const saver = autosave(save);

	saver.edited();
	saver.destroy();
	await vi.advanceTimersByTimeAsync(5000);

	expect(save).not.toHaveBeenCalled();
});
