import { actor } from "rivetkit";
import { db } from "rivetkit/db";
import { workflow } from "rivetkit/workflow";

const onboarding = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS account (id INTEGER PRIMARY KEY CHECK (id = 1), email TEXT NOT NULL)",
			);
		},
	}),
	onCreate: async (c, input: { email: string }) => {
		await c.db.execute("INSERT INTO account (id, email) VALUES (1, ?)", input.email);
	},
	run: workflow(async (ctx) => {
		const email = await ctx.step("load-email", async (step) => {
			const rows = await step.db.execute<{ email: string }>(
				"SELECT email FROM account WHERE id = 1",
			);
			return rows[0].email;
		});

		// Completed steps are recorded, so a restart resumes after the last one
		await ctx.step("send-welcome", async () => {
			await sendEmail(email, "Welcome!");
		});

		await ctx.sleep("wait-one-day", 24 * 60 * 60 * 1000);

		await ctx.step("send-tips", async () => {
			await sendEmail(email, "Getting started tips");
		});
	}),
});

async function sendEmail(to: string, subject: string): Promise<void> {
	const res = await fetch("https://api.example.com/email", {
		method: "POST",
		body: JSON.stringify({ to, subject }),
	});
	if (!res.ok) throw new Error(`Email failed: ${res.status}`);
}
