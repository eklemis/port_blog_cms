export default interface IEducation {
	title: string;
	gpa: { valueAt: string; valueMax: string };
	location: string;
	period: string;
	bulletItems: string[];
	institutionLogo: { url: string };
}
