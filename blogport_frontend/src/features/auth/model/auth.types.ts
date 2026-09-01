export interface RegisterDto {
	email: string;
	full_name: string;
	username: string;
	password: string;
}

export interface LoginDto {
	email: string;
	password: string;
}
