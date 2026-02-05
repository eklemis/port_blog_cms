// List all color variants you plan to use in your theme
const allowedColors = [
	'blue-900',
	'orange-700',
	'gray-500',
	'red-500',
	'green-600'
	// Add more if needed
] as const;

type AllowedColor = (typeof allowedColors)[number];
type Utility = 'text' | 'border' | 'bg' | 'outline' | 'ring';

export function getColorClass(type: Utility, color: string | null | undefined): string {
	if (!color || !allowedColors.includes(color as AllowedColor)) {
		console.warn(`[getColorClass] Invalid color: "${color}"`);
		return '';
	}
	return `${type}-${color}`;
}

// Optional: Get all class combinations for safelisting
export function generateColorSafelist(): string[] {
	const types: Utility[] = ['text', 'border', 'bg', 'outline', 'ring'];
	return allowedColors.flatMap((color) => types.map((type) => `${type}-${color}`));
}
