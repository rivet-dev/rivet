export interface Request<Args extends Array<unknown>> {
	args: Args,
}

export interface ResponseOk<T> {
 output: T;
}

export interface ResponseError {
	message: string;
}

