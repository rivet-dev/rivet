import _ from "lodash";

export function mergeWatchStreams(
	oldData: unknown,
	newChunk: unknown,
): unknown {
	return _.mergeWith({}, oldData, newChunk, (objValue, srcValue) => {
		if (_.isArray(objValue)) {
			return objValue.concat(srcValue);
		}
	});
}
