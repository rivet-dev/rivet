// biome-ignore lint/suspicious/noExplicitAny: FIXME
function AutoFormTooltip({ fieldConfigItem }: { fieldConfigItem: any }) {
	return (
		<>
			{fieldConfigItem?.description && (
				<p className="text-sm text-gray-500 dark:text-white">
					{fieldConfigItem.description}
				</p>
			)}
		</>
	);
}

export default AutoFormTooltip;
