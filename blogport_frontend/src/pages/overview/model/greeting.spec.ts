import { expect, test } from 'vitest';
import { greeting } from './greeting';

const at = (hour: number) => new Date(2026, 8, 8, hour, 0, 0);

test('names the part of the day from the viewer’s own clock', () => {
	expect(greeting('Jane Doe', at(9))).toBe('Good morning, Jane');
	expect(greeting('Jane Doe', at(14))).toBe('Good afternoon, Jane');
	expect(greeting('Jane Doe', at(20))).toBe('Good evening, Jane');
});

test('the boundaries fall where the words do', () => {
	expect(greeting('Jane Doe', at(11))).toMatch(/morning/);
	expect(greeting('Jane Doe', at(12))).toMatch(/afternoon/);
	expect(greeting('Jane Doe', at(17))).toMatch(/afternoon/);
	expect(greeting('Jane Doe', at(18))).toMatch(/evening/);
	expect(greeting('Jane Doe', at(0))).toMatch(/morning/);
});

test('uses the name someone goes by, not all of it', () => {
	expect(greeting('Ursula K. Le Guin', at(9))).toBe('Good morning, Ursula');
});

test('one word is a whole name', () => {
	expect(greeting('Prince', at(9))).toBe('Good morning, Prince');
});

test('greets nobody in particular rather than a stray comma', () => {
	expect(greeting('   ', at(9))).toBe('Good morning');
	expect(greeting('', at(9))).toBe('Good morning');
});
