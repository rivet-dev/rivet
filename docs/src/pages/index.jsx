import Main from '@/framer/main';

export default function Index() {
	return (
		<div>
			<Main.Responsive 
				style={{ width: '100%' }}
				variants={{
					lg: 'Desktop',
					md: 'Tablet',
					base: 'Phone',
				}}
			/>
		</div>
	);
}

Index.description = 'Open-Source Multiplayer Tooling. A Single Tool to Manage Your Game Servers & Backend.';
Index.prose = false;
Index.fullWidth = true;

