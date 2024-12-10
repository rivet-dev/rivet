import SalesFramer from '@/framer/sales';

export default function Index() {
	return (
		<div>
			<SalesFramer.Responsive
				style={{ width: '100%', background: '#000000' }}
				variants={{
					xl: 'Desktop',
					md: 'Tablet',
					sm: 'Phone',
				}}
			/>
		</div>
	);
}

Index.prose = false;
Index.fullWidth = true;

