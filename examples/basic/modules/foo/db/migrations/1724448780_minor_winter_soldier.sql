CREATE SCHEMA "module_foo";
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "module_foo"."db_entry" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"test2" text,
	"test3" text,
	"test5" boolean
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "module_foo"."table6" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"hello2" text
);
