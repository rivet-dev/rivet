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
console.log(`  Decimal: ${num}`);
console.log(`  Hex: 0x${num.toString(16).padStart(16, '0')}`);
console.log(`  Bytes: ${Array.from(beBuf).map(b => b.toString(16).padStart(2, '0')).join(' ')}`);

// Convert to little endian
const leBuf = Buffer.allocUnsafe(8);
leBuf.writeBigUInt64LE(num);
const littleEndianValue = leBuf.readBigUInt64BE(); // Read the LE bytes as if they were BE

console.log('\nConverted (Little Endian):');
console.log(`  Decimal: ${littleEndianValue}`);
console.log(`  Hex: 0x${littleEndianValue.toString(16).padStart(16, '0')}`);
console.log(`  Bytes: ${Array.from(leBuf).map(b => b.toString(16).padStart(2, '0')).join(' ')}`);

console.log('\n' + '='.repeat(50) + '\n');
