export default interface ILanguage {
	id: string;
	name: string;
	proficiencyLabel: string;
	proficiencySlider: { valueAt: number; valueMax: number };
}
