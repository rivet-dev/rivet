#!/usr/bin/env tsx

/**
 * Converts a number from big endian to little endian byte order
 */

function bigEndianToLittleEndian(num: bigint): bigint {
	// Convert number to 64-bit buffer (big endian)
	const buffer = Buffer.allocUnsafe(8);
	buffer.writeBigUInt64BE(num);

	// Read as little endian
	const littleEndian = buffer.readBigUInt64LE();

	return littleEndian;
}

// Main execution
const inputNumber = process.argv[2] || '360287970189639680';
const num = BigInt(inputNumber);

console.log('\nBig Endian to Little Endian Conversion\n');
console.log('='.repeat(50));

// Show original (big endian)
const beBuf = Buffer.allocUnsafe(8);
beBuf.writeBigUInt64BE(num);
console.log('\nOriginal (Big Endian):');
console.log(`  Decimal (u64): ${num}`);
console.log(`  Hex: 0x${num.toString(16).padStart(16, '0')}`);
console.log(`  Bytes: ${Array.from(beBuf).map(b => b.toString(16).padStart(2, '0')).join(' ')}`);

// Show as signed integer
const beI64 = beBuf.readBigInt64BE();
console.log(`  Decimal (i64): ${beI64}`);

// Convert to little endian
const leBuf = Buffer.allocUnsafe(8);
leBuf.writeBigUInt64LE(num);

const leU64 = leBuf.readBigUInt64BE();
const leI64 = leBuf.readBigInt64BE();

console.log('\nConverted (Little Endian):');
console.log(`  Decimal (u64): ${leU64}`);
console.log(`  Decimal (i64): ${leI64}`);
console.log(`  Hex: 0x${leU64.toString(16).padStart(16, '0')}`);
console.log(`  Bytes: ${Array.from(leBuf).map(b => b.toString(16).padStart(2, '0')).join(' ')}`);


console.log('\n' + '='.repeat(50) + '\n');
