import { expect, test } from 'vitest';
import { UNEXPECTED, rateLimited, retryAfterSeconds } from './api-failure';

test('the countdown reads in whole units and gets the singulars right', () => {
	expect(rateLimited(1)).toBe('Too many attempts. Try again in 1 second.');
	expect(rateLimited(45)).toBe('Too many attempts. Try again in 45 seconds.');
	expect(rateLimited(60)).toBe('Too many attempts. Try again in 1 minute.');
	expect(rateLimited(90)).toBe('Too many attempts. Try again in 2 minutes.');
	expect(rateLimited(2520)).toBe('Too many attempts. Try again in 42 minutes.');
});

test('the lead can name the action that was limited', () => {
	// J1: "Too many sign-up attempts. Try again in 42 minutes."
	expect(rateLimited(2520, 'Too many sign-up attempts')).toBe(
		'Too many sign-up attempts. Try again in 42 minutes.'
	);
	expect(rateLimited(null, 'Too many sign-up attempts')).toBe(
		'Too many sign-up attempts. Try again shortly.'
	);
});

test('a missing Retry-After still says something useful', () => {
	expect(rateLimited(null)).toBe('Too many attempts. Try again shortly.');
	expect(rateLimited(0)).toBe('Too many attempts. Try again shortly.');
});

test('reads Retry-After off a response', () => {
	const response = new Response(null, { status: 429, headers: { 'retry-after': '300' } });

	expect(retryAfterSeconds(response)).toBe(300);
});

test('an absent or nonsense Retry-After is null, not NaN', () => {
	// A NaN reaching the countdown renders "Try again in NaN minutes."
	expect(retryAfterSeconds(new Response(null, { status: 429 }))).toBeNull();
	expect(
		retryAfterSeconds(new Response(null, { headers: { 'retry-after': 'Wed, 21 Oct 2026' } }))
	).toBeNull();
	expect(retryAfterSeconds(new Response(null, { headers: { 'retry-after': '-5' } }))).toBeNull();
});

test('the fallback is the blueprint’s sentence, not a paraphrase', () => {
	expect(UNEXPECTED).toBe('Something went wrong on our side.');
});
