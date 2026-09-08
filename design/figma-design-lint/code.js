/* ============================================================================
   Blogport — Design Lint
   ----------------------------------------------------------------------------
   Six checks that catch a frame whose layout has silently broken, plus one
   token check. Every rule here exists because it was missed: the tablet set was
   rebuilt three times, and three separate defects were reported by a person
   after an audit had called the same frames clean.

   The failures these catch are silent. Figma reports nothing, the file has no
   errors, and the frame just looks wrong — which is why they survive until
   somebody screenshots them.

   WHY THE EXCLUSIONS ARE PART OF THE RULE
   A check that fires on every tab bar gets switched off within a day. Each
   exclusion below is a real false positive that was observed, not a guess.
   ============================================================================ */

const RULES = {
	overflowsParent: {
		label: 'Overflows its parent',
		why: 'Wider than the box holding it — whether or not that box clips. A non-clipping card hides this from any clip-based check; that is what let a 176px overrun through.'
	},
	outsideManualParent: {
		label: 'Outside a manual parent',
		why: 'Sits past the right edge of a layoutMode:NONE container. Nothing reflows on its own there, so it stays wrong.'
	},
	spillsFrame: {
		label: 'Leaves the artboard',
		why: 'Extends past the frame itself. Usually a pinned rail or a modal that kept its desktop width.'
	},
	squeezedText: {
		label: 'Text squeezed',
		why: 'Long text in a very narrow box — the one-word-per-line failure. Almost always a FILL child inside a HUG parent.'
	},
	squeezedControl: {
		label: 'Control squeezed',
		why: 'A button or field narrowed past usability, usually by a row that shared space equally instead of letting prose absorb it.'
	},
	overlap: {
		label: 'Siblings overlap',
		why: 'In vertical auto layout one child runs into the next. Follows a manual container whose height was not recomputed after its text rewrapped.'
	},
	unboundColour: {
		label: 'Colour off-system',
		why: 'A solid fill or stroke not bound to a theme variable. It will not change with the theme.'
	}
};

const TEXT_MIN_WIDTH = 110; // below this, long copy wraps one word per line
const TEXT_MIN_CHARS = 25;
const CONTROL_MIN_WIDTH = 40;
const CONTROL_MIN_HEIGHT = 20;
const TOL = 2; // sub-pixel rounding

function inner(node) {
	return node.width - (node.paddingLeft || 0) - (node.paddingRight || 0);
}

/** Icons are meant to be 16–24px. Counting them as squeezed controls flagged
    13 mobile frames whose only crime was having a tab bar. */
function isIcon(node) {
	return node.name.indexOf('icon/') === 0;
}

function isManualContainer(node) {
	return !node.layoutMode || node.layoutMode === 'NONE';
}

function check(frame) {
	const found = [];
	const frameBox = frame.absoluteBoundingBox;
	const add = (rule, node, detail) =>
		found.push({ rule: rule, id: node.id, name: node.name, type: node.type, detail: detail });

	const nodes = frame.findAll(n => n.visible !== false);

	for (const n of nodes) {
		const p = n.parent;
		const pinned = n.layoutPositioning === 'ABSOLUTE';

		if (p && p.type === 'FRAME' && p.visible !== false && typeof n.width === 'number') {
			// 1 — pinned nodes are positioned deliberately; they are covered by rule 3
			if (!pinned && n.width > inner(p) + TOL) {
				add('overflowsParent', n, Math.round(n.width) + 'px in ' + Math.round(inner(p)) + 'px');
			}
			// 2 — FILL is a no-op outside auto layout, so these need explicit widths
			if (isManualContainer(p) && n.x + n.width > p.width - (p.paddingRight || 0) + TOL) {
				add(
					'outsideManualParent',
					n,
					'right edge ' + Math.round(n.x + n.width) + ' of ' + Math.round(p.width)
				);
			}
		}

		// 3
		const b = n.absoluteBoundingBox;
		if (b && frameBox) {
			if (b.x + b.width > frameBox.x + frameBox.width + TOL || b.x < frameBox.x - TOL) {
				add('spillsFrame', n, 'extends ' + Math.round(b.x + b.width - (frameBox.x + frameBox.width)) + 'px past');
			}
		}

		// 4
		if (n.type === 'TEXT' && n.characters.length > TEXT_MIN_CHARS && n.width < TEXT_MIN_WIDTH) {
			add('squeezedText', n, Math.round(n.width) + 'px for ' + n.characters.length + ' chars');
		}

		// 5
		if (
			n.type === 'INSTANCE' &&
			!isIcon(n) &&
			n.width < CONTROL_MIN_WIDTH &&
			n.height > CONTROL_MIN_HEIGHT
		) {
			add('squeezedControl', n, Math.round(n.width) + '×' + Math.round(n.height));
		}

		// 7 — tokens. Image paints bind to nothing by nature and are not a fault.
		const paints = [];
		if (Array.isArray(n.fills)) paints.push.apply(paints, n.fills);
		if (Array.isArray(n.strokes)) paints.push.apply(paints, n.strokes);
		for (const paint of paints) {
			if (paint.type !== 'SOLID' || paint.visible === false) continue;
			if (paint.boundVariables && paint.boundVariables.color) continue;
			add('unboundColour', n, 'unbound solid paint');
			break;
		}
	}

	// 6 — stacked children running into one another
	const stacks = frame.findAll(
		n => n.type === 'FRAME' && n.visible !== false && n.layoutMode === 'VERTICAL' && n.children
	);
	for (const box of stacks) {
		const kids = box.children.filter(c => c.visible !== false && c.layoutPositioning !== 'ABSOLUTE');
		for (let i = 0; i < kids.length - 1; i++) {
			const a = kids[i].absoluteBoundingBox;
			const c = kids[i + 1].absoluteBoundingBox;
			if (a && c && a.y + a.height > c.y + 1) {
				add('overlap', kids[i], 'runs into "' + kids[i + 1].name.slice(0, 24) + '"');
			}
		}
	}
	return found;
}

function targetFrames(scope) {
	const page = figma.currentPage;
	if (scope === 'selection') {
		const sel = page.selection.filter(n => n.type === 'FRAME');
		return sel.length ? sel : [];
	}
	return page.children.filter(n => n.type === 'FRAME');
}

function run(scope, rulesOn) {
	const frames = targetFrames(scope);
	const results = [];
	let total = 0;
	for (const f of frames) {
		const hits = check(f).filter(h => rulesOn.indexOf(h.rule) !== -1);
		if (!hits.length) continue;
		const byRule = {};
		for (const h of hits) {
			byRule[h.rule] = byRule[h.rule] || [];
			byRule[h.rule].push(h);
		}
		results.push({ frame: f.name, id: f.id, count: hits.length, byRule: byRule });
		total += hits.length;
	}
	results.sort((a, b) => b.count - a.count);
	return { framesScanned: frames.length, framesWithFindings: results.length, total: total, results: results };
}

figma.showUI(__html__, { width: 460, height: 620, themeColors: true });

figma.ui.onmessage = async msg => {
	if (msg.type === 'run') {
		const report = run(msg.scope, msg.rules);
		figma.ui.postMessage({ type: 'report', report: report, rules: RULES });
		return;
	}
	if (msg.type === 'reveal') {
		const node = await figma.getNodeByIdAsync(msg.id);
		if (!node) {
			figma.notify('That node no longer exists — re-run the lint.');
			return;
		}
		// selecting needs a node on the current page
		let top = node;
		while (top.parent && top.parent.type !== 'PAGE') top = top.parent;
		if (top.parent && top.parent.id !== figma.currentPage.id) {
			await figma.setCurrentPageAsync(top.parent);
		}
		figma.currentPage.selection = [node];
		figma.viewport.scrollAndZoomIntoView([node]);
		return;
	}
	if (msg.type === 'close') figma.closePlugin();
};

// Run once on open so the panel is never empty.
figma.ui.postMessage({ type: 'rules', rules: RULES });
