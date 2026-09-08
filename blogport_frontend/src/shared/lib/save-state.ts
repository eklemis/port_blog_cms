/**
 * The four things an editor can say about its own saving.
 *
 * Down here because two layers need the same word for the same thing: the
 * autosave loop that produces the state, and the indicator that renders it —
 * and a shared component may not reach up into a feature for a type.
 *
 * J4: "The status line reads Saving… → Saved 10:42 → Unsaved changes, and it is
 * the only place that reports save state." `retrying` is the branch below it.
 */
export type SaveState = 'saved' | 'unsaved' | 'saving' | 'retrying';
