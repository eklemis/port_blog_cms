export enum ESectionTypes {
	Header,
	Achievement,
	Certification,
	Education,
	Experience,
	Interest,
	Language,
	Project,
	Publication,
	Reference,
	Skill,
	SocialMedia,
	Strength,
	Summary,
	TrainingCourse,
	Volunteering
}

export interface ISection {
	sectionType: ESectionTypes;
	sectionTitle: string;
	data: {
		rows: any[];
		displaySetting: object;
	};
	column: number;
}
export function cloneSection(section: ISection): ISection {
	return {
		sectionType: section.sectionType,
		sectionTitle: section.sectionTitle,
		data: {
			rows: [...section.data.rows], // shallow copy rows array
			displaySetting: { ...section.data.displaySetting } // shallow copy settings
		},
		column: section.column
	};
}
