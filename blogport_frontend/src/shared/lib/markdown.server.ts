import { Marked, type Tokens } from 'marked';

/**
 * Markdown → HTML, on the server and only on the server.
 *
 * The frame says so itself (Screen / Public post 72:32): "Markdown → HTML
 * happens in +page.server.ts; no parser ships to the reader." A reader of a
 * published post downloads the finished article — not a parser, and not the
 * source it would have to parse. The `.server` in the filename is what enforces
 * it: SvelteKit refuses to bundle this module into the client.
 *
 * Two things it does beyond parsing, both of them rules from elsewhere:
 *
 * - **Media references resolve here.** Stored bodies hold `![alt](media:<id>)`,
 *   never a URL, because a read URL is signed and expires — writing one into
 *   the body would store a link that dies. See ADR 0006.
 * - **Raw HTML never survives.** The body is stored text belonging to whoever
 *   holds the account; rendering it as markup is what turns a stolen session
 *   into a defaced page. Markdown is the whole vocabulary.
 */

/**
 * Where an image lives, or `null` when it has nowhere to point yet.
 *
 * A media row exists before its variants do, so a post published while its
 * image is still being processed carries the reference with no URL behind it.
 */
export type ResolveMedia = (mediaId: string) => string | null;

/** Anything that is not plainly a page, a fragment or a mail address. */
function unsafeHref(href: string): boolean {
	// Control characters and spaces are stripped first, because `java\0script:`
	// and `java script:` are both read as `javascript:` by a browser. Done by
	// character code rather than a regex: a regex holding literal control
	// characters is its own kind of unreadable.
	const bare = [...href].filter((c) => c.charCodeAt(0) > 0x20).join('');
	const scheme = /^([a-z][a-z0-9+.-]*):/i.exec(bare);
	if (!scheme) return false;
	return !['http', 'https', 'mailto'].includes(scheme[1].toLowerCase());
}

const escapes: Record<string, string> = {
	'&': '&amp;',
	'<': '&lt;',
	'>': '&gt;',
	'"': '&quot;'
};

function escape(text: string): string {
	return text.replace(/[&<>"]/g, (c) => escapes[c]);
}

export function renderMarkdown(source: string, resolveMedia: ResolveMedia): string {
	if (!source.trim()) return '';

	const marked = new Marked({ gfm: true, breaks: false });

	marked.use({
		renderer: {
			// Both halves of the raw-HTML door, block and inline.
			html: () => '',
			image(token: Tokens.Image) {
				const reference = /^media:(.+)$/.exec(token.href);
				const src = reference ? resolveMedia(reference[1]) : token.href;

				// Nowhere to point is not a broken image on the page — it is no image.
				if (!src || unsafeHref(src)) return '';

				const title = token.title ? ` title="${escape(token.title)}"` : '';
				return `<img src="${escape(src)}" alt="${escape(token.text ?? '')}"${title} loading="lazy" />`;
			},
			link(token: Tokens.Link) {
				const text = this.parser.parseInline(token.tokens);

				// The words stay; only the destination goes. A reader still reads the
				// sentence they were written into.
				if (unsafeHref(token.href)) return text;

				const title = token.title ? ` title="${escape(token.title)}"` : '';
				return `<a href="${escape(token.href)}"${title} rel="nofollow noopener">${text}</a>`;
			}
		}
	});

	return marked.parse(source, { async: false });
}
