import { expect, test } from 'vitest';
import { experienceSummary, isCurrent, years } from './experience';

/**
 * A role on a résumé, as the builder summarises it — Screen / CV builder 69:2.
 *
 * The frame collapses each row to one line, and says why: "Rows collapse to a
 * one-line summary. One opens at a time — ten expanded rows is a wall, not a
 * form."
 */

const role = (over: Record<string, unknown> = {}) => ({
	company: 'Tokopedia',
	position: 'Backend Engineer',
	location: 'Jakarta',
	start_date: '2022-01',
	end_date: '2025-04',
	tasks: ['Built the order-events pipeline'],
	achievements: [],
	description: '',
	...over
});

test('a collapsed row reads as the job, not the record', () => {
	// "Backend Engineer · Tokopedia" — what it was, then where.
	expect(experienceSummary(role())).toBe('Backend Engineer · Tokopedia');
});

test('a role missing half its identity still reads', () => {
	expect(experienceSummary(role({ position: '' }))).toBe('Tokopedia');
	expect(experienceSummary(role({ company: '' }))).toBe('Backend Engineer');
});

test('a role with neither is named as the new thing it is', () => {
	// A row added and not yet filled in has to be findable in the list.
	expect(experienceSummary(role({ company: '', position: '' }))).toBe('New role');
});

test('the years are the years, not the months', () => {
	// The frame writes "2020 – 2022" under a collapsed row, where the fields
	// themselves hold "2020-01".
	expect(years(role({ start_date: '2020-03', end_date: '2022-11' }))).toBe('2020 – 2022');
});

test('a job someone still has says so rather than trailing off', () => {
	expect(years(role({ start_date: '2022-01', end_date: null }))).toBe('2022 – now');
});

test('a role with no start yet is not given a dash to nowhere', () => {
	expect(years(role({ start_date: '', end_date: null }))).toBe('');
});

test('no end date is what "I work here now" means', () => {
	// `end_date` is documented as "Absent for a current position", so the
	// checkbox is the absence rather than a separate flag.
	expect(isCurrent(role({ end_date: null }))).toBe(true);
	expect(isCurrent(role({ end_date: '' }))).toBe(true);
	expect(isCurrent(role())).toBe(false);
});
