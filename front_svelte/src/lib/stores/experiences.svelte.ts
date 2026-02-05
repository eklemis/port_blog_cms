import type IExperience from '$lib/types/contents/iexperience';

export const experiences = $state<IExperience[]>([
	{
		id: 'opd-001',
		title: 'Operation & Data Coordinator',
		companyName: 'Save the Children Indonesia',
		companyLogo: '',
		companyDescription: 'Non-Governmental Organization (NGO)',
		location: 'Waikabubak, Indonesia',
		period: '03/2018 - Present',
		bulletItems: [
			'Design, develop, and maintain a high-performance data ecosystem, integrating PostgreSQL, SQLite, and MS SQL Server databases with Power BI dashboards and custom applications (e.g., AI Virtual Assistant using NextJS/Typescript) to support sponsorship operations and data-driven decision-making for child-focused programs.',
			'Architect and optimize ETL pipelines using Python (Pandas, NumPy) to streamline data collection, quality assurance, and reporting, ensuring timely and accurate submission of child inventories and sponsorship deliverables to the ASISt system and U.S. counterparts.',
			'Lead the Operations team (1 officer, 3 assistants), developing strategies to enhance sponsorship retention through high-quality communications, real-time monitoring, and stakeholder collaboration, achieving 100% completion rates for over 10,000 annual family updates in 2024.',
			'Implement data governance practices, securing sensitive child data with backup/recovery protocols and quality checks, aligning with Save the Children’s child safeguarding and operational standards.',
			'Deploy and manage cloud-based solutions (e.g., AWS, Azure) and hybrid infrastructure, coordinating with Sr. IT Manager to maintain ASISt server uptime and troubleshoot issues, ensuring scalability and minimal data loss.',
			'Train staff on data system ASISt and analytics tools, building capacity for accurate data entry, photo standards, and actionable insights, while mentoring team members to meet KPIs rated by Global Sponsorship.',
			'Develop and present comprehensive reports and visualizations, translating complex data into actionable insights for decision making regarding program and operation progress and impact.'
		]
	},
	{
		id: 'opd-002',
		title: 'Database Officer',
		companyName: 'Save the Children Indonesia',
		companyLogo: '',
		companyDescription: 'Non-Governmental Organization (NGO)',
		location: 'Waikabubak, Indonesia',
		period: '03/2018 - 07/2024',
		bulletItems: [
			'Supervised two Database Assistants, overseeing day-to-day sponsorship data management tasks to ensure accuracy, completeness, and timeliness across 172 partner schools.',
			"Managed the sponsorship database (ASISt) for record-keeping of children's profiles, correspondence tracking, and program reporting, including maintaining data backup and disaster recovery planning.",
			'Liaised with Global Sponsorship Operations teams to troubleshoot technical issues related to server connectivity, database access, and system upgrades, ensuring continued functionality of ASISt.',
			'Developed internal digital tools (e.g., Data Quality Checker, Tamo App) to improve data validation, error detection, and operational efficiency, reducing manual errors and turnaround time.',
			'Coordinated with field teams and school contact points to monitor the collection and verification of annual child updates, including photographs and school progress data.',
			'Conducted regular data quality reviews, automated validation processes, and trained program staff on data entry standards and quality assurance protocols.',
			'Supported the integration of safeguarding standards into data processes, ensuring responsible handling of sensitive child information in compliance with organizational guidelines.'
		]
	},
	{
		id: 'opd-003',
		title: 'IT Web Dev Consultant',
		companyName: 'Adriansiaril',
		companyLogo: '',
		companyDescription: 'Individual',
		location: 'Remote',
		period: '07/2024 - 09/2024',
		bulletItems: [
			'Designed and developed the AI Virtual Assistant, a full-stack web application using NextJS with Typescript and PostgreSQL, enabling automated exit interviews and knowledge management for organizational staff.',
			'Collaborated with users to gather and refine requirements, delivering a tailored solution to streamline workflows.',
			'Architected a clean backend structure, developed data models, and implemented robust unit and integration tests to ensure reliability.',
			'Built an intuitive frontend interface and deployed the app to a cloud environment (e.g., Vercel/AWS), optimizing for scalability and performance.',
			'Researched and recommended best-fit technologies to address user-specific challenges, enhancing system efficiency and usability.'
		]
	}
]);
