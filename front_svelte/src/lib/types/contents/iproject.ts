export default interface IProject {
	id: string;
	title: string;
	description: string;
	bulletItems: string[];
	location: string;
	period: string;
	link: { url: string };
	samplePictures: { urls: string[] };
}
