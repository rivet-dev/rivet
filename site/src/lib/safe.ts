export async function safeAwait(promise) {
    try {
        return [null, await promise];
    } catch (error) {
        console.error(error);
        return [error, null];
    }
}
