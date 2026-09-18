import { describe, expect, test } from 'vitest';
import { renderMarkdown } from './markdown.server';

/**
 * Screen / Public post 72:32 says it in the frame itself: "Markdown → HTML
 * happens in +page.server.ts; no parser ships to the reader." So this module is
 * server-only, and these are node tests.
 */

const media = (id: string) => `https://api.example.test/api/public/media/${id}/large`;

/**
 * What a reader sees, with the highlighter's own markup taken back off.
 *
 * Highlighting splits code across spans, so asserting on a contiguous string
 * would be asserting on `highlight.js`'s internals. This asks the question that
 * matters instead: does the code read as written?
 */
function visible(html: string): string {
	return html
		.replace(/<[^>]+>/g, '')
		.replaceAll('&lt;', '<')
		.replaceAll('&gt;', '>')
		.replaceAll('&quot;', '"')
		.replaceAll('&#39;', "'")
		.replaceAll('&amp;', '&');
}

describe('the body a reader gets', () => {
	test('turns the ordinary marks into ordinary elements', () => {
		const html = renderMarkdown('## Layout\n\nOne **service**, split into *slices*.', media);

		expect(html).toContain('<h2>Layout</h2>');
		expect(html).toContain('<strong>service</strong>');
		expect(html).toContain('<em>slices</em>');
	});

	test('keeps code as code rather than running it through the formatter', () => {
		const html = renderMarkdown('Use `cargo check`.\n\n```rust\nlet x = 1;\n```', media);

		expect(html).toContain('<code>cargo check</code>');
		expect(visible(html)).toContain('let x = 1;');
	});

	test('resolves a media reference to the public path', () => {
		// Post bodies hold `![alt](media:<id>)` and resolve at render — the stored
		// Markdown can never hold a signed URL, because signed URLs expire.
		const html = renderMarkdown('![Hexagonal layout](media:8f1b2c3d)', media);

		expect(html).toContain('src="https://api.example.test/api/public/media/8f1b2c3d/large"');
		expect(html).toContain('alt="Hexagonal layout"');
	});

	test('drops an image whose media has no variant yet, rather than a broken one', () => {
		// A media row exists before its variants do, so a post published while an
		// image is still processing carries the reference with no URL behind it.
		const html = renderMarkdown(
			'Before\n\n![Still processing](media:pending)\n\nAfter',
			() => null
		);

		expect(html).not.toContain('<img');
		expect(html).toContain('Before');
	});

	test('leaves an ordinary image URL alone', () => {
		const html = renderMarkdown('![A chart](https://example.test/chart.png)', media);

		expect(html).toContain('src="https://example.test/chart.png"');
	});

	test('drops raw HTML instead of passing it through to the reader', () => {
		// The body is stored text. Rendering it as markup is what makes a stolen
		// session into a defaced page, so the parser renders Markdown and nothing
		// else — there is no author who needs a <script> in a blog post.
		const html = renderMarkdown('Fine\n\n<script>alert(1)</script>\n\n<b>bold</b> too', media);

		expect(html).not.toContain('<script');
		expect(html).not.toContain('<b>');
		expect(html).toContain('Fine');
	});

	test('refuses a javascript: link while keeping its words', () => {
		const html = renderMarkdown('[press me](javascript:alert(1))', media);

		expect(html).not.toContain('javascript:');
		expect(html).toContain('press me');
	});

	test('answers empty for an empty body rather than throwing', () => {
		expect(renderMarkdown('', media)).toBe('');
	});
});

describe('code blocks', () => {
	test('highlights a fenced block in a language it knows', () => {
		// §03: "Code blocks need highlighting, and this audience will notice.
		// Highlight at render time on the server rather than shipping a
		// client-side highlighter."
		const html = renderMarkdown('```rust\nlet x = 1;\n```', media);

		expect(html).toContain('class="hljs language-rust"');
		expect(html).toContain('hljs-keyword');
		expect(html).toContain('let');
	});

	test('a fence with no language is still code, just not coloured', () => {
		const html = renderMarkdown('```\nplain text\n```', media);

		expect(html).toContain('<pre><code');
		expect(html).toContain('plain text');
		expect(html).not.toContain('hljs-');
	});

	test('a language nobody has heard of does not throw', () => {
		const html = renderMarkdown('```wingdings\nlet x = 1;\n```', media);

		expect(html).toContain('let x = 1;');
		expect(html).not.toContain('hljs-');
	});

	test('markup inside a code block stays text', () => {
		// The highlighter escapes what it emits, and a fence is the one place an
		// author legitimately writes a tag they do not want rendered.
		const html = renderMarkdown('```html\n<script>alert(1)</script>\n```', media);

		// Escaped, and therefore inert — but still exactly what was typed.
		expect(html).not.toContain('<script>alert(1)</script>');
		expect(visible(html)).toContain('<script>alert(1)</script>');
	});

	test('an unlabelled fence escapes its markup too', () => {
		const html = renderMarkdown('```\n<img src=x onerror=1>\n```', media);

		expect(html).not.toContain('<img src=x');
		expect(visible(html)).toContain('<img src=x onerror=1>');
	});
});
