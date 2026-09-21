import type { MediaRole, MediaState } from './media';

/**
 * What a media tile says about itself — Screen / Media library 69:242.
 *
 * Four lines, and the one that carries weight is the prompt. An image with no
 * alt text says "No alt text — add one" in accent ink where the others are
 * muted: it is a job, not a fact. The library is the only screen that shows a
 * whole collection at once, so it is the only place that missing descriptions
 * can be seen as a set.
 */

export type Tile = {
	media_id: string;
	original_filename: string;
	role: MediaRole;
	status: MediaState;
	alt_text: string;
};

export type TileStatus = {
	text: string;
	/** Drawn as something to fix rather than something to read. */
	prompt: boolean;
};

export function tileStatus(tile: Pick<Tile, 'role' | 'status' | 'alt_text'>): TileStatus {
	if (tile.status === 'failed') return { text: 'Processing failed', prompt: false };

	// Nothing is described until there is something to describe, and two
	// problems on one tile reads as two jobs when there is one.
	if (tile.status !== 'ready') return { text: 'Processing', prompt: false };

	if (!tile.alt_text.trim()) return { text: 'No alt text — add one', prompt: true };

	return { text: `Alt text set · ${tile.role}`, prompt: false };
}
