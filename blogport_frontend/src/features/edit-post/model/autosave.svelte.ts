import type { SaveState } from '$lib/shared/lib/save-state';

/**
 * Saving as you go — J4 step three.
 *
 * "PATCH on a two-second idle debounce. The status line reads Saving… →
 * Saved 10:42 → Unsaved changes, and it is the only place that reports save
 * state." The fourth state is the branch below it: a failed save keeps the
 * buffer, says so, and retries with backoff.
 *
 * The rule the states exist to protect: never say "Saved" over text that is
 * not. Typing while a call is in the air leaves the buffer dirty, because the
 * call carries the older words.
 */

/** J4's number. Long enough to be one save per sentence, not one per word. */
const IDLE = 2000;

/** 1s, 2s, 4s … up to a minute. A save that keeps failing must not keep hammering. */
const FIRST_BACKOFF = 1000;
const MAX_BACKOFF = 60_000;

export type Autosave = {
	readonly state: SaveState;
	/**
	 * When the last successful save landed, in epoch milliseconds, for the
	 * status line to name. A number rather than a `Date` because a `Date` held
	 * in reactive state is a trap — mutating one in place changes nothing on
	 * screen — and this one is only ever replaced whole.
	 */
	readonly savedAt: number | null;
	/** Whether there are changes the server has not taken. */
	readonly dirty: boolean;
	/** Something changed. Starts the clock. */
	edited: () => void;
	/** Save now — for someone about to navigate away. */
	flush: () => Promise<void>;
	destroy: () => void;
};

export function createAutosave({
	save,
	idle = IDLE
}: {
	/** Resolves true when the server took it. Never rejects — it reports. */
	save: () => Promise<boolean>;
	idle?: number;
}): Autosave {
	let state = $state<SaveState>('saved');
	let savedAt = $state<number | null>(null);
	let dirty = $state(false);

	let timer: ReturnType<typeof setTimeout> | undefined;
	let backoff = FIRST_BACKOFF;
	let stopped = false;

	function schedule(delay: number) {
		clearTimeout(timer);
		if (stopped) return;
		timer = setTimeout(run, delay);
	}

	async function run() {
		if (stopped || !dirty) return;

		state = 'saving';
		// Cleared before the call, not after: anything typed while it is in the
		// air belongs to the next save, and this one must not claim it.
		dirty = false;

		const ok = await save();
		if (stopped) return;

		if (!ok) {
			// The buffer is still ahead of the server, whatever was typed since.
			dirty = true;
			state = 'retrying';
			schedule(backoff);
			backoff = Math.min(backoff * 2, MAX_BACKOFF);
			return;
		}

		backoff = FIRST_BACKOFF;
		savedAt = Date.now();
		// Dirty again already means someone typed during the call.
		state = dirty ? 'unsaved' : 'saved';
	}

	return {
		get state() {
			return state;
		},
		get savedAt() {
			return savedAt;
		},
		get dirty() {
			return dirty;
		},

		edited() {
			dirty = true;
			// Immediately, because "Unsaved changes" is the honest state the moment
			// a keystroke lands. The two seconds are the debounce, not the truth.
			if (state !== 'saving' && state !== 'retrying') state = 'unsaved';
			schedule(idle);
		},

		async flush() {
			clearTimeout(timer);
			if (!dirty) return;
			await run();
		},

		destroy() {
			stopped = true;
			clearTimeout(timer);
		}
	};
}
