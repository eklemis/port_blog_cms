type Ok<T> = { data: T; error: undefined; response: Response };
type Err<E> = { data: undefined; error: E; response: Response };
export type HttpResult<T, E> = Ok<T> | Err<E>;

export function assertOk<T, E>(result: HttpResult<T, E>): asserts result is Ok<T> {
	if (result.error !== undefined) {
		// Keep the shape simple; throw a plain object and let callers map it to errors.
		throw {
			status: result.response.status,
			error: result.error
		};
	}
}
