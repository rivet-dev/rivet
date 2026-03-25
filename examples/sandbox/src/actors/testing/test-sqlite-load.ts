import { actor } from "rivetkit";
import { db } from "rivetkit/db";

export const testSqliteLoad = actor({
	db: db({
		onMigrate: async (db) => {
			// Migration 1: schema version tracking
			await db.execute(`
				CREATE TABLE IF NOT EXISTS schema_version (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					version INTEGER NOT NULL DEFAULT 50
				)
			`);
			await db.execute(
				"INSERT OR IGNORE INTO schema_version (id, version) VALUES (1, 50)",
			);

			// Migration 2
			await db.execute(`
				CREATE TABLE IF NOT EXISTS users (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL,
					email TEXT,
					created_at INTEGER NOT NULL
				)
			`);

			// Migration 3
			await db.execute(`
				CREATE TABLE IF NOT EXISTS products (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL,
					price REAL NOT NULL DEFAULT 0,
					created_at INTEGER NOT NULL
				)
			`);

			// Migration 4
			await db.execute(`
				CREATE TABLE IF NOT EXISTS orders (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					user_id INTEGER NOT NULL,
					total REAL NOT NULL DEFAULT 0,
					status TEXT NOT NULL DEFAULT 'pending',
					created_at INTEGER NOT NULL,
					FOREIGN KEY (user_id) REFERENCES users(id)
				)
			`);

			// Migration 5
			await db.execute(`
				CREATE TABLE IF NOT EXISTS order_items (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					order_id INTEGER NOT NULL,
					product_id INTEGER NOT NULL,
					quantity INTEGER NOT NULL DEFAULT 1,
					price REAL NOT NULL,
					FOREIGN KEY (order_id) REFERENCES orders(id),
					FOREIGN KEY (product_id) REFERENCES products(id)
				)
			`);

			// Migration 6
			await db.execute(`
				CREATE TABLE IF NOT EXISTS categories (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL UNIQUE,
					description TEXT
				)
			`);

			// Migration 7
			await db.execute(`
				CREATE TABLE IF NOT EXISTS product_categories (
					product_id INTEGER NOT NULL,
					category_id INTEGER NOT NULL,
					PRIMARY KEY (product_id, category_id),
					FOREIGN KEY (product_id) REFERENCES products(id),
					FOREIGN KEY (category_id) REFERENCES categories(id)
				)
			`);

			// Migration 8
			await db.execute(`
				CREATE TABLE IF NOT EXISTS reviews (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					user_id INTEGER NOT NULL,
					product_id INTEGER NOT NULL,
					rating INTEGER NOT NULL CHECK (rating BETWEEN 1 AND 5),
					comment TEXT,
					created_at INTEGER NOT NULL,
					FOREIGN KEY (user_id) REFERENCES users(id),
					FOREIGN KEY (product_id) REFERENCES products(id)
				)
			`);

			// Migration 9
			await db.execute(`
				CREATE TABLE IF NOT EXISTS addresses (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					user_id INTEGER NOT NULL,
					street TEXT NOT NULL,
					city TEXT NOT NULL,
					state TEXT,
					zip TEXT,
					country TEXT NOT NULL DEFAULT 'US',
					FOREIGN KEY (user_id) REFERENCES users(id)
				)
			`);

			// Migration 10
			await db.execute(`
				CREATE TABLE IF NOT EXISTS payments (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					order_id INTEGER NOT NULL,
					amount REAL NOT NULL,
					method TEXT NOT NULL DEFAULT 'card',
					status TEXT NOT NULL DEFAULT 'pending',
					processed_at INTEGER,
					FOREIGN KEY (order_id) REFERENCES orders(id)
				)
			`);

			// Migration 11
			await db.execute(`
				CREATE TABLE IF NOT EXISTS inventory (
					product_id INTEGER PRIMARY KEY,
					quantity INTEGER NOT NULL DEFAULT 0,
					reserved INTEGER NOT NULL DEFAULT 0,
					last_restocked_at INTEGER,
					FOREIGN KEY (product_id) REFERENCES products(id)
				)
			`);

			// Migration 12
			await db.execute(`
				CREATE TABLE IF NOT EXISTS coupons (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					code TEXT NOT NULL UNIQUE,
					discount_percent REAL NOT NULL,
					max_uses INTEGER,
					used_count INTEGER NOT NULL DEFAULT 0,
					expires_at INTEGER
				)
			`);

			// Migration 13
			await db.execute(`
				CREATE TABLE IF NOT EXISTS shipping (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					order_id INTEGER NOT NULL,
					address_id INTEGER NOT NULL,
					carrier TEXT,
					tracking_number TEXT,
					shipped_at INTEGER,
					delivered_at INTEGER,
					FOREIGN KEY (order_id) REFERENCES orders(id),
					FOREIGN KEY (address_id) REFERENCES addresses(id)
				)
			`);

			// Migration 14
			await db.execute(`
				CREATE TABLE IF NOT EXISTS tags (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL UNIQUE
				)
			`);

			// Migration 15
			await db.execute(`
				CREATE TABLE IF NOT EXISTS product_tags (
					product_id INTEGER NOT NULL,
					tag_id INTEGER NOT NULL,
					PRIMARY KEY (product_id, tag_id),
					FOREIGN KEY (product_id) REFERENCES products(id),
					FOREIGN KEY (tag_id) REFERENCES tags(id)
				)
			`);

			// Migration 16
			await db.execute(`
				CREATE TABLE IF NOT EXISTS wishlists (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					user_id INTEGER NOT NULL,
					name TEXT NOT NULL DEFAULT 'Default',
					created_at INTEGER NOT NULL,
					FOREIGN KEY (user_id) REFERENCES users(id)
				)
			`);

			// Migration 17
			await db.execute(`
				CREATE TABLE IF NOT EXISTS wishlist_items (
					wishlist_id INTEGER NOT NULL,
					product_id INTEGER NOT NULL,
					added_at INTEGER NOT NULL,
					PRIMARY KEY (wishlist_id, product_id),
					FOREIGN KEY (wishlist_id) REFERENCES wishlists(id),
					FOREIGN KEY (product_id) REFERENCES products(id)
				)
			`);

			// Migration 18
			await db.execute(`
				CREATE TABLE IF NOT EXISTS notifications (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					user_id INTEGER NOT NULL,
					type TEXT NOT NULL,
					message TEXT NOT NULL,
					read INTEGER NOT NULL DEFAULT 0,
					created_at INTEGER NOT NULL,
					FOREIGN KEY (user_id) REFERENCES users(id)
				)
			`);

			// Migration 19
			await db.execute(`
				CREATE TABLE IF NOT EXISTS audit_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					entity_type TEXT NOT NULL,
					entity_id INTEGER NOT NULL,
					action TEXT NOT NULL,
					details TEXT,
					performed_at INTEGER NOT NULL
				)
			`);

			// Migration 20
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sessions (
					id TEXT PRIMARY KEY,
					user_id INTEGER NOT NULL,
					token TEXT NOT NULL UNIQUE,
					expires_at INTEGER NOT NULL,
					created_at INTEGER NOT NULL,
					FOREIGN KEY (user_id) REFERENCES users(id)
				)
			`);

			// Migration 21
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)",
			);

			// Migration 22
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_orders_user ON orders(user_id)",
			);

			// Migration 23
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status)",
			);

			// Migration 24
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_reviews_product ON reviews(product_id)",
			);

			// Migration 25
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_reviews_user ON reviews(user_id)",
			);

			// Migration 26
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_notifications_user ON notifications(user_id)",
			);

			// Migration 27
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_audit_entity ON audit_log(entity_type, entity_id)",
			);

			// Migration 28
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id)",
			);

			// Migration 29
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token)",
			);

			// Migration 30
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_inventory_quantity ON inventory(quantity)",
			);

			// Migration 31
			await db.execute(`
				CREATE TABLE IF NOT EXISTS returns (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					order_id INTEGER NOT NULL,
					reason TEXT NOT NULL,
					status TEXT NOT NULL DEFAULT 'requested',
					requested_at INTEGER NOT NULL,
					processed_at INTEGER,
					FOREIGN KEY (order_id) REFERENCES orders(id)
				)
			`);

			// Migration 32
			await db.execute(`
				CREATE TABLE IF NOT EXISTS return_items (
					return_id INTEGER NOT NULL,
					order_item_id INTEGER NOT NULL,
					quantity INTEGER NOT NULL,
					PRIMARY KEY (return_id, order_item_id),
					FOREIGN KEY (return_id) REFERENCES returns(id),
					FOREIGN KEY (order_item_id) REFERENCES order_items(id)
				)
			`);

			// Migration 33
			await db.execute(`
				CREATE TABLE IF NOT EXISTS suppliers (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL,
					contact_email TEXT,
					country TEXT
				)
			`);

			// Migration 34
			await db.execute(`
				CREATE TABLE IF NOT EXISTS product_suppliers (
					product_id INTEGER NOT NULL,
					supplier_id INTEGER NOT NULL,
					cost REAL NOT NULL,
					lead_time_days INTEGER,
					PRIMARY KEY (product_id, supplier_id),
					FOREIGN KEY (product_id) REFERENCES products(id),
					FOREIGN KEY (supplier_id) REFERENCES suppliers(id)
				)
			`);

			// Migration 35
			await db.execute(`
				CREATE TABLE IF NOT EXISTS price_history (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					product_id INTEGER NOT NULL,
					old_price REAL NOT NULL,
					new_price REAL NOT NULL,
					changed_at INTEGER NOT NULL,
					FOREIGN KEY (product_id) REFERENCES products(id)
				)
			`);

			// Migration 36
			await db.execute(`
				CREATE TABLE IF NOT EXISTS user_preferences (
					user_id INTEGER PRIMARY KEY,
					theme TEXT NOT NULL DEFAULT 'dark',
					language TEXT NOT NULL DEFAULT 'en',
					notifications_enabled INTEGER NOT NULL DEFAULT 1,
					FOREIGN KEY (user_id) REFERENCES users(id)
				)
			`);

			// Migration 37
			await db.execute(`
				CREATE TABLE IF NOT EXISTS cart (
					user_id INTEGER NOT NULL,
					product_id INTEGER NOT NULL,
					quantity INTEGER NOT NULL DEFAULT 1,
					added_at INTEGER NOT NULL,
					PRIMARY KEY (user_id, product_id),
					FOREIGN KEY (user_id) REFERENCES users(id),
					FOREIGN KEY (product_id) REFERENCES products(id)
				)
			`);

			// Migration 38
			await db.execute(`
				CREATE TABLE IF NOT EXISTS saved_searches (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					user_id INTEGER NOT NULL,
					query TEXT NOT NULL,
					filters TEXT,
					created_at INTEGER NOT NULL,
					FOREIGN KEY (user_id) REFERENCES users(id)
				)
			`);

			// Migration 39
			await db.execute(`
				CREATE TABLE IF NOT EXISTS product_images (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					product_id INTEGER NOT NULL,
					url TEXT NOT NULL,
					alt_text TEXT,
					sort_order INTEGER NOT NULL DEFAULT 0,
					FOREIGN KEY (product_id) REFERENCES products(id)
				)
			`);

			// Migration 40
			await db.execute(`
				CREATE TABLE IF NOT EXISTS discounts (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					product_id INTEGER NOT NULL,
					discount_percent REAL NOT NULL,
					starts_at INTEGER NOT NULL,
					ends_at INTEGER,
					FOREIGN KEY (product_id) REFERENCES products(id)
				)
			`);

			// Migration 41
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_order_items_order ON order_items(order_id)",
			);

			// Migration 42
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_order_items_product ON order_items(product_id)",
			);

			// Migration 43
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_payments_order ON payments(order_id)",
			);

			// Migration 44
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_shipping_order ON shipping(order_id)",
			);

			// Migration 45
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_returns_order ON returns(order_id)",
			);

			// Migration 46
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_price_history_product ON price_history(product_id)",
			);

			// Migration 47
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_product_images_product ON product_images(product_id)",
			);

			// Migration 48
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_discounts_product ON discounts(product_id)",
			);

			// Migration 49
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_cart_user ON cart(user_id)",
			);

			// Migration 50
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_saved_searches_user ON saved_searches(user_id)",
			);

			// Migration 51: counter for benchmarking
			await db.execute(`
				CREATE TABLE IF NOT EXISTS counter (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					value INTEGER NOT NULL DEFAULT 0
				)
			`);
			await db.execute(
				"INSERT OR IGNORE INTO counter (id, value) VALUES (1, 0)",
			);
		},
	}),
	actions: {
		increment: async (c, amount: number = 1) => {
			await c.db.execute(
				"UPDATE counter SET value = value + ? WHERE id = 1",
				amount,
			);
			const rows = await c.db.execute(
				"SELECT value FROM counter WHERE id = 1",
			);
			return (rows[0] as { value: number }).value;
		},
		getCount: async (c) => {
			const rows = await c.db.execute(
				"SELECT value FROM counter WHERE id = 1",
			);
			return (rows[0] as { value: number }).value;
		},
		reset: async (c) => {
			await c.db.execute("UPDATE counter SET value = 0 WHERE id = 1");
			return 0;
		},
		runLoadTest: async (c) => {
			const now = Date.now();
			const results: string[] = [];

			// Query 1: Insert a user
			await c.db.execute(
				"INSERT INTO users (name, email, created_at) VALUES (?, ?, ?)",
				"Load Test User",
				`load-${now}@test.com`,
				now,
			);
			results.push("inserted user");

			// Query 2: Get the user back
			const users = await c.db.execute(
				"SELECT * FROM users WHERE email = ?",
				`load-${now}@test.com`,
			);
			results.push(`fetched user: ${(users[0] as { name: string }).name}`);
			const userId = (users[0] as { id: number }).id;

			// Query 3: Insert a product
			await c.db.execute(
				"INSERT INTO products (name, price, created_at) VALUES (?, ?, ?)",
				"Test Widget",
				29.99,
				now,
			);
			results.push("inserted product");

			// Query 4: Get products
			const products = await c.db.execute("SELECT * FROM products LIMIT 10");
			results.push(`fetched ${(products as unknown[]).length} products`);
			const productId = (products[0] as { id: number }).id;

			// Query 5: Insert a category
			await c.db.execute(
				"INSERT OR IGNORE INTO categories (name, description) VALUES (?, ?)",
				`test-cat-${now}`,
				"A test category",
			);
			results.push("inserted category");

			// Query 6: Get categories
			const categories = await c.db.execute("SELECT * FROM categories");
			results.push(`fetched ${(categories as unknown[]).length} categories`);

			// Query 7: Insert an order
			await c.db.execute(
				"INSERT INTO orders (user_id, total, status, created_at) VALUES (?, ?, ?, ?)",
				userId,
				29.99,
				"pending",
				now,
			);
			results.push("inserted order");

			// Query 8: Get orders for user
			const orders = await c.db.execute(
				"SELECT * FROM orders WHERE user_id = ?",
				userId,
			);
			results.push(`fetched ${(orders as unknown[]).length} orders for user`);
			const orderId = (orders[0] as { id: number }).id;

			// Query 9: Insert order item
			await c.db.execute(
				"INSERT INTO order_items (order_id, product_id, quantity, price) VALUES (?, ?, ?, ?)",
				orderId,
				productId,
				2,
				29.99,
			);
			results.push("inserted order item");

			// Query 10: Insert inventory
			await c.db.execute(
				"INSERT OR REPLACE INTO inventory (product_id, quantity, reserved, last_restocked_at) VALUES (?, ?, ?, ?)",
				productId,
				100,
				2,
				now,
			);
			results.push("inserted inventory");

			// Query 11: Insert a review
			await c.db.execute(
				"INSERT INTO reviews (user_id, product_id, rating, comment, created_at) VALUES (?, ?, ?, ?, ?)",
				userId,
				productId,
				5,
				"Great product!",
				now,
			);
			results.push("inserted review");

			// Query 12: Get reviews with join
			const reviews = await c.db.execute(
				"SELECT r.*, u.name as reviewer FROM reviews r JOIN users u ON r.user_id = u.id WHERE r.product_id = ?",
				productId,
			);
			results.push(`fetched ${(reviews as unknown[]).length} reviews`);

			// Query 13: Insert notification
			await c.db.execute(
				"INSERT INTO notifications (user_id, type, message, created_at) VALUES (?, ?, ?, ?)",
				userId,
				"order",
				"Your order has been placed",
				now,
			);
			results.push("inserted notification");

			// Query 14: Insert audit log entry
			await c.db.execute(
				"INSERT INTO audit_log (entity_type, entity_id, action, details, performed_at) VALUES (?, ?, ?, ?, ?)",
				"order",
				orderId,
				"created",
				`Order created by user ${userId}`,
				now,
			);
			results.push("inserted audit log");

			// Query 15: Aggregate query on orders
			const orderStats = await c.db.execute(
				"SELECT status, COUNT(*) as count, SUM(total) as total_value FROM orders GROUP BY status",
			);
			results.push(
				`order stats: ${(orderStats as unknown[]).length} statuses`,
			);

			// Query 16: Insert address
			await c.db.execute(
				"INSERT INTO addresses (user_id, street, city, state, zip, country) VALUES (?, ?, ?, ?, ?, ?)",
				userId,
				"123 Test St",
				"Testville",
				"CA",
				"90210",
				"US",
			);
			results.push("inserted address");

			// Query 17: Complex join query
			const orderDetails = await c.db.execute(`
				SELECT o.id, o.status, o.total, u.name as customer, COUNT(oi.id) as item_count
				FROM orders o
				JOIN users u ON o.user_id = u.id
				LEFT JOIN order_items oi ON oi.order_id = o.id
				GROUP BY o.id
				LIMIT 10
			`);
			results.push(
				`fetched ${(orderDetails as unknown[]).length} order details`,
			);

			// Query 18: Update order status
			await c.db.execute(
				"UPDATE orders SET status = ? WHERE id = ?",
				"completed",
				orderId,
			);
			results.push("updated order status");

			// Query 19: Get schema version
			const version = await c.db.execute(
				"SELECT version FROM schema_version WHERE id = 1",
			);
			results.push(
				`schema version: ${(version[0] as { version: number }).version}`,
			);

			// Query 20: Count all tables
			const tableCounts = await c.db.execute(`
				SELECT 'users' as tbl, COUNT(*) as cnt FROM users
				UNION ALL SELECT 'products', COUNT(*) FROM products
				UNION ALL SELECT 'orders', COUNT(*) FROM orders
				UNION ALL SELECT 'reviews', COUNT(*) FROM reviews
				UNION ALL SELECT 'categories', COUNT(*) FROM categories
			`);
			results.push(
				`table counts: ${(tableCounts as unknown[]).length} tables checked`,
			);

			return {
				queriesRun: 20,
				results,
			};
		},
	},
});
