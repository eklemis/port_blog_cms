<script lang="ts">
	import type IDesignFont from '$lib/types/designfont_settings/idesfont';

	import type IHeader from '$lib/types/contents/iheader';
	import type IDisplayHeader from '$lib/types/display_settings/idheader';

	import type ISummary from '$lib/types/contents/isummary';
	import type IDisplaySummary from '$lib/types/display_settings/idsummary';

	import type IExperience from '$lib/types/contents/iexperience';
	import type IDisplayExperience from '$lib/types/display_settings/idexperience';

	import type IProject from '$lib/types/contents/iproject';
	import type IDisplayProject from '$lib/types/display_settings/idproject';

	import type IPublication from '$lib/types/contents/ipublication';
	import type IDisplayPublication from '$lib/types/display_settings/idpublication';

	import type IVolunteering from '$lib/types/contents/ivolunteering';
	import type IDisplayVolunteering from '$lib/types/display_settings/idvolunteering';

	import type ITraining from '$lib/types/contents/itraining';
	import type IDisplayTraining from '$lib/types/display_settings/idtraining';

	import type ISkill from '$lib/types/contents/iskill';
	import type IDisplaySkill from '$lib/types/display_settings/idskill';

	import type ILanguage from '$lib/types/contents/ilanguage';
	import type IDisplayLanguage from '$lib/types/display_settings/idlanguage';

	import { experiences } from '$lib/stores/experiences.svelte';
	import Timeline from '$lib/components/cv/templates/timeline/Template.svelte';

	let designFont: IDesignFont = $state({
		paperBackgroundImage: '',
		pageMargin: 0,
		columnLayoutOption: 1,
		sectionSpacing: 1,
		fontSize: '',
		fontStyle: 0,
		lineHeight: 1,
		primaryColor: 'blue-900',
		secondaryColor: 'orange-700'
	});
	let headerContent: IHeader[] = [
		{
			title: 'AAS Awardee',
			name: 'Eklemis Santo Ndun',
			phone: '+62-8523-7470-411',
			link: 'ekstion.com',
			extraLink: 'eknotes.app',
			email: 'Eklemis.Ndun@gmail.com',
			location: 'Waikabubak, Indonesia',
			profilePhotoUrl: '',
			extraField: ''
		}
	];
	let headerDisplay: IDisplayHeader = {
		nameUppercase: true,
		showPhone: true,
		showLink: true,
		showExtraLink: false,
		showEmail: true,
		showLocation: true,
		showPhoto: true,
		photoStyle: '',
		showExtraField: false
	};

	let summaryContent: ISummary[] = [
		{
			description:
				'Data operations professional with a background in computer science and a focus on strengthening evidence-based practices within education programs in remote areas of Indonesia. Over the past seven years, contributed to improving operational integrity by developing digital validation tools, enhancing data quality systems, and facilitating school-community coordination to support program monitoring. Work reflects a commitment to leveraging technology to address systemic barriers in educational data management. Currently seeking to deepen expertise in data science methodologies, human-centered system design, and ethical data governance to contribute to more inclusive, data-informed strategies for improving early-grade learning outcomes in underserved contexts.'
		}
	];
	let summaryDisplay: IDisplaySummary = {
		showDescription: true
	};

	let experienceContent: IExperience[] = experiences.map((exp) => ({ ...exp }));
	let experienceDisplay: IDisplayExperience = {
		showCompanyLogo: false,
		showCompanyDescription: true
	};

	let projectContent: IProject[] = [
		{
			id: 'proj-001',
			title: 'AI Virtual Assistant for Exit Interviews',
			description:
				'Designed and developed a web-based application (Next.js and PostgreSQL) as an interaction layer to OpenAI, aimed at assisting exit interviews with resigning staff. Led requirement gathering, prototyping, clean architecture design, backend and frontend development, and database modeling. The tool supports knowledge management by recording and summarizing exit interview insights for organizational learning.',
			bulletItems: ['item-1'],
			location: 'Remote',
			period: '07/2024 - 09/2024',
			link: { url: '' },
			samplePictures: { urls: [''] }
		},
		{
			id: 'proj-002',
			title: 'Program Performance Dashboard – Cross-Partner Monitoring Tool',
			description:
				'Led the technical development of a program performance dashboard using Power BI to support collaborative monitoring across Save the Children and two implementing partners (PSI and YWKW). The dashboard consolidates multi-source data to track annual indicator targets and Reading Fun activity coverage, and is used during quarterly review meetings. I was responsible for redesigning the data schema, preparing and cleansing field data, and adjusting dashboard components to meet evolving program requirements.',
			bulletItems: ['item-1'],
			location: 'Waikabubak, Indonesia',
			period: '2024 - Present',
			link: { url: '' },
			samplePictures: { urls: [''] }
		},
		{
			id: 'proj-003',
			title: 'E-AFU Data Monitor Dashboard – Field Data Collection and Entry Tracker',
			description:
				'Developed a Power BI dashboard to monitor real-time progress of field data collection and data entry during the Annual Family Update (AFU) Campaign. The dashboard allows segregation of data by community, school, and data entry staff, enabling precise tracking and early identification of bottlenecks. It also integrates children’s feedback collected during the campaign, providing a more holistic view of progress and quality. This tool helped ensure that data targets were achieved on time with improved data quality standards.',
			bulletItems: ['item-1'],
			location: 'Waikabubak, Indonesia',
			period: '2023 - Present',
			link: { url: '' },
			samplePictures: { urls: [''] }
		},
		{
			id: 'proj-004',
			title: 'Intervention Tracker – Web Application for Program Monitoring',
			description:
				'Designed and developed a web application using SvelteKit, Supabase, and PostgreSQL to track and visualize program interventions across supported schools and communities. The tool imports intervention records from Excel files and displays them in both graphical and tabular formats, allowing users to easily review the type and frequency of support provided each year. It strengthens transparency and program management by providing stakeholders with clear historical data on intervention coverage.',
			bulletItems: ['item-1'],
			location: 'Waikabubak, Indonesia',
			period: '08/2024 - 11/2024',
			link: { url: '' },
			samplePictures: { urls: [''] }
		},
		{
			id: 'proj-005',
			title: 'Photo Review Tool – Desktop App for Photograph Verification and Tracking',
			description:
				"Designed, developed, and maintained a desktop application using Rust, Tauri, Svelte, and SQLite to support the review and verification of children's photographs collected for sponsorship records. The tool allows users to rotate misaligned images, tag photos with unique identifiers, and monitor collection progress across photographers and schools. It has improved photo quality assurance and reduced processing errors by replacing manual tracking with a faster, more reliable system.",
			bulletItems: ['item-1'],
			location: 'Waikabubak, Indonesia',
			period: '02/2022 - 04/2024',
			link: { url: '' },
			samplePictures: { urls: [''] }
		},
		{
			id: 'proj-006',
			title: 'Tamo App – Sponsorship Letter Management System',
			description:
				"Designed, developed, and maintained a web application using Node.js, Next.js, Prisma ORM, and PostgreSQL to support the correspondence process between donors and sponsored children. The system injects donor and child information into letter templates, tracks incoming and outgoing letters, and enables staff to monitor each letter's progress across multiple processing stages. This tool streamlined the high-volume communication workflow and reduced manual tracking errors.",
			bulletItems: ['item-1'],
			location: 'Waikabubak, Indonesia',
			period: '02/2021 - Present',
			link: { url: '' },
			samplePictures: { urls: [''] }
		},
		{
			id: 'proj-007',
			title: 'Data Quality Checker – Batch Validation and Import Tool',
			description:
				'Developed Python scripts to automate the validation and import of child data from Excel files into the MS Access-based E-AFU application used for sponsorship data entry. The tool checks for errors and inconsistencies before uploading, reducing manual correction efforts and supporting batch processing during peak data collection periods. It significantly improved data accuracy and efficiency, particularly when handling thousands of records under tight deadlines.',
			bulletItems: ['item-1'],
			location: 'Waikabubak, Indonesia',
			period: '02/2019 - 10/2022',
			link: { url: '' },
			samplePictures: { urls: [''] }
		}
	];
	let projectDisplay: IDisplayProject = {
		showDescription: true,
		showBullets: false,
		showLocation: true,
		showPeriod: true,
		showLink: false,
		showSamplePictures: false
	};

	let pubContent: IPublication[] = [
		{
			id: 'pub-001',
			title:
				'Pornography Detection System in Digital Images based on Color and Shape Features using Support Vector Machine (SVM)',
			publisher: 'J-ICON, Vol. 2No. 1, March 2014, pp. 10~20',
			authors: 'Eklemis Santo Ndun, Adriana Fanggidae, Sebastianus A. S. Mola',
			period: '03/2014',
			link: { url: '' }
		}
	];
	let pubDisplay: IDisplayPublication = {
		showPublisher: true,
		showAuthors: true,
		showPeriod: true,
		showLink: true
	};
	let volunteeringContent: IVolunteering[] = [
		{
			id: 'volunt-001',
			title: 'Debate Competition Judge (Senior High School Level)',
			institution: 'Dinas Pendidikan Provinsi Nusa Tenggara Timur',
			description:
				'Served as a judge in a two-day philosophical debate competition involving 12 senior high school teams from across Sumba Barat. Assessed the structure, evidence, and relevance of arguments presented. Compiled scores from all judges and determined final rankings for the top four teams. Participated in the technical meeting with teachers to align on judging criteria and ensure fair competition.',
			bullets: [],
			location: 'Sumba Barat, Indonesia',
			period: '2024'
		},
		{
			id: 'volunt-002',
			title: 'Youth Design Workshop',
			institution: 'Save the Children Indonesia',
			description:
				'Delivered a two-day graphic design workshop for youth groups, introducing Canva, color theory, and layout design. Each group produced a promotional poster for their community initiative. The workshop enhanced participants’ creativity and confidence in using digital tools for advocacy and storytelling.',
			bullets: [],
			location: 'Waikabubak, Indonesia',
			period: '2022'
		},
		{
			id: 'volunt-003',
			title: 'Frontend Mentor Contributor',
			institution: 'Frontend Mentor Community',
			description:
				'Actively participated in the Frontend Mentor open-source learning community by completing 18 frontend development challenges and offering constructive feedback to fellow developers. Shared technical guidance on responsive design, layout techniques, and JavaScript optimization. Promoted peer learning through detailed comments and solutions, helping others improve while refining personal frontend development skills in HTML, Javascript, CSS and ReactJS.',
			bullets: [],
			location: 'Online',
			period: '2021-2022'
		},
		{
			id: 'volunt-004',
			title: 'Free TOEFL Preparation Course for University Alumni',
			institution: 'Independent Community Initiative',
			description:
				'Initiated and co-facilitated a free TOEFL preparation course for university alumni in Kupang to improve English proficiency and increase access to scholarship opportunities. Collaborated with an LPDP awardee and mentored high-performing participants to take on peer-teaching roles. The course built a supportive learning environment for over 40 participants, several of whom later secured international postgraduate scholarships.',
			bullets: [],
			location: 'Kupang, Indonesia',
			period: '2017-2018'
		},
		{
			id: 'volunt-005',
			title: 'ICT Training for Village Officials and Teachers (KKN Program)',
			institution: 'Universitas Nusa Cendana – Community Service Program (KKN)',
			description:
				'Provided digital literacy training for village officials and school teachers during a university-led community service program. Installed OS and productivity software on unused village computers, and delivered hands-on training in MS Word and Excel. The program helped local staff transition from typewriters to digital document processing.',
			bullets: [],
			location: 'Kupang, Indonesia',
			period: '2012'
		}
	];
	let volunteeringDisplay: IDisplayVolunteering = {
		showDescription: true,
		showBullets: false,
		showLocation: true,
		showPeriod: true
	};
	let trainingCoursesContent: ITraining[] = [
		{
			id: 'tc-001',
			title: 'Develop Applications with Azure OpenAI Service',
			institution: 'Microsoft Learn',
			period: '2025'
		},
		{
			id: 'tc-002',
			title: 'Design a data ingestion strategy for machine learning projects',
			institution: 'Microsoft Learn',
			period: '2025'
		},
		{
			id: 'tc-003',
			title: 'Become Back-End Developer Expert with JavaScript',
			institution: 'Dicoding Indonesia',
			period: '2024'
		},
		{
			id: 'tc-004',
			title: 'Become Node.js Application Developer',
			institution: 'Dicoding Indonesia',
			period: '2024'
		},
		{
			id: 'tc-005',
			title: 'Software Architecture & Design of Modern Large Scale Systems',
			institution: 'Top Developer Academy LLC',
			period: '2023'
		},
		{
			id: 'tc-006',
			title: 'The Data Strategy Course: Building a Data-driven Business',
			institution: '365 Careers',
			period: '2023'
		},
		{
			id: 'tc-007',
			title: 'Intro to UI Design Fundamentals',
			institution: 'Scrimba.com',
			period: '2023'
		},
		{
			id: 'tc-008',
			title: 'Big Data Analytics: Opportunities, Challenges and the Future',
			institution: 'Griffith University',
			period: '2020'
		}
	];
	let trainingCoursesDisplay: IDisplayTraining = {
		showInstitution: true,
		showPeriod: true
	};

	let skillContent: ISkill[] = [
		{ group: '', name: 'Data Visualization' },
		{ group: '', name: 'Web Application Development' },
		{ group: '', name: 'Software Architecture' },
		{ group: '', name: 'Clean Architecture' },
		{ group: '', name: 'Digital Tool Development' },
		{ group: '', name: 'Cross Sector Collaboration' }
	];
	let skillDisplay: IDisplaySkill = {
		showGroupName: false,
		compactMode: false,
		borderStyle: 'BottomSide'
	};

	let languageContent: ILanguage[] = [
		{
			id: 'lang-001',
			name: 'English',
			proficiencyLabel: 'Advance',
			proficiencySlider: { valueAt: 4, valueMax: 5 }
		},
		{
			id: 'lang-002',
			name: 'Bahasa Indonesia',
			proficiencyLabel: 'Native',
			proficiencySlider: { valueAt: 5, valueMax: 5 }
		}
	];
	let languageDisplay: IDisplayLanguage = {
		showProficiencyLabel: true,
		showProficiencySlider: true,
		sliderStyle: 'Battery'
	};

	let sections = {
		header: {
			content: headerContent,
			displaySetting: headerDisplay
		},
		summary: {
			content: summaryContent,
			displaySetting: summaryDisplay
		},
		experience: {
			content: experienceContent,
			displaySetting: experienceDisplay
		},
		publication: {
			content: pubContent,
			displaySetting: pubDisplay
		},
		projects: {
			content: projectContent,
			displaySetting: projectDisplay
		},
		volunteerings: {
			content: volunteeringContent,
			displaySetting: volunteeringDisplay
		},
		trainingCourses: {
			content: trainingCoursesContent,
			displaySetting: trainingCoursesDisplay
		},
		skills: {
			content: skillContent,
			displaySetting: skillDisplay
		},
		languages: {
			content: languageContent,
			displaySetting: languageDisplay
		}
	};
</script>

<Timeline {sections} {designFont}></Timeline>
