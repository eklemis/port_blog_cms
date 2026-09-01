import createClient from 'openapi-fetch';
import type { paths } from './v1';
import { backendBaseUrl } from '$lib/shared/config/backend';

export const { GET, POST, PUT, DELETE } = createClient<paths>({
	baseUrl: backendBaseUrl
});
