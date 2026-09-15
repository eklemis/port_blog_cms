/**
 * The sentence under "No posts match those filters".
 *
 * Screen / Posts — filtered empty draws "You have 24 posts — none of them are
 * drafts tagged “Rust”." It says what there is, and which filter is the reason
 * nothing shows — the whole point of this state is that nobody reads it as
 * their work having gone.
 *
 * The count is every post in the main list, from the summary, not the filtered
 * total: the filtered total is the zero that brought someone here.
 */
export function filteredSentence({
	everything,
	published = null,
	topic = null,
	search = ''
}: {
	/** Live and draft posts together. `null` when it could not be had. */
	everything: number | null;
	published?: string | null;
	topic?: string | null;
	search?: string;
}): string {
	const status = published === 'false' ? 'drafts' : published === 'true' ? 'published' : null;
	const tagged = topic ? `tagged “${topic}”` : null;
	const matching = search ? `“${search}”` : null;

	// "are drafts tagged “Rust” matching “kafka”", or "match “kafka”" alone.
	const described =
		status || tagged
			? [status, tagged, matching && `matching ${matching}`].filter(Boolean).join(' ')
			: `match ${matching}`;

	if (everything === null) {
		return status || tagged
			? `None of your posts are ${described}.`
			: `None of your posts ${described}.`;
	}

	if (everything === 1) {
		return status || tagged
			? `You have 1 post — it is not ${described}.`
			: `You have 1 post — it does not ${described}.`;
	}

	return status || tagged
		? `You have ${everything} posts — none of them are ${described}.`
		: `You have ${everything} posts — none of them ${described}.`;
}
