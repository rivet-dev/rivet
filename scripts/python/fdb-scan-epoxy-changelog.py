"""
Scan an epoxy changelog subspace and stop at the first suspected v3 entry.

Usage: python3 fdb-scan-epoxy-changelog.py <replica_id>

v2 ChangelogEntry BARE layout: key: data, value: data, version: u64 LE, mutable: bool.
v3 changed value to optional<data>. Entries have no version header, so detection
relies on a roundtrip check through the v2 schema. Empty-value entries are skipped
because v2 empty write and v3 None deletion both encode with a \x00 length prefix.
"""
import fdb, struct, sys

fdb.api_version(730)
db = fdb.open('/etc/foundationdb/fdb.cluster')

REPLICA_ID = int(sys.argv[1])
# Tuple prefix: (RIVET=0, EPOXY_V2=124, REPLICA=89, replica_id, CHANGELOG=123)
PREFIX = fdb.tuple.pack((0, 124, 89, REPLICA_ID, 123))
PREFIX_END = PREFIX + b'\xff'


def decode_uleb128(data, offset):
    result = 0
    shift = 0
    while True:
        b = data[offset]
        offset += 1
        result |= (b & 0x7F) << shift
        if not (b & 0x80):
            break
        shift += 7
    return result, offset


def encode_uleb128(n):
    if n == 0:
        return b'\x00'
    out = []
    while n:
        b = n & 0x7F
        n >>= 7
        if n:
            b |= 0x80
        out.append(b)
    return bytes(out)


def decode_data(raw, offset):
    n, offset = decode_uleb128(raw, offset)
    val = raw[offset:offset + n]
    return val, offset + n


def encode_data(b):
    return encode_uleb128(len(b)) + b


def parse_v2(raw):
    offset = 0
    key, offset = decode_data(raw, offset)
    value, offset = decode_data(raw, offset)
    if offset + 9 != len(raw):
        raise ValueError('unexpected trailing bytes: consumed %d, total %d' % (offset, len(raw)))
    version = struct.unpack_from('<Q', raw, offset)[0]
    offset += 8
    mutable_byte = raw[offset]
    if mutable_byte not in (0, 1):
        raise ValueError('invalid bool byte: %d' % mutable_byte)
    return key, value, version, mutable_byte


def serialize_v2(key, value, version, mutable):
    return encode_data(key) + encode_data(value) + struct.pack('<Q', version) + bytes([mutable])


@fdb.transactional
def scan_chunk(tr, start, limit=1000):
    return list(tr.get_range(start, PREFIX_END, limit=limit))


start = PREFIX
total = 0
v2_count = 0
v3_count = 0
chunk = 0

while True:
    rows = scan_chunk(db, start)
    if not rows:
        break
    chunk += 1
    for kv in rows:
        total += 1
        raw = bytes(kv.value)
        try:
            key, value, version, mutable = parse_v2(raw)
        except ValueError as e:
            print('[#%d] SUSPECTED V3 (%s)' % (total, e))
            print('  fdb_key  : %s' % bytes(kv.key).hex())
            print('  fdb_value: %s' % raw.hex())
            sys.exit(0)

        if len(value) == 0:
            # Ambiguous: v2 empty write and v3 None deletion both encode with \x00 length prefix.
            v3_count += 1
            continue

        roundtrip = serialize_v2(key, value, version, mutable)
        if roundtrip != raw:
            print('[#%d] SUSPECTED V3 (roundtrip mismatch)' % total)
            print('  fdb_key  : %s' % bytes(kv.key).hex())
            print('  fdb_value: %s' % raw.hex())
            print('  roundtrip: %s' % roundtrip.hex())
            sys.exit(0)

        v2_count += 1

    start = fdb.KeySelector.first_greater_than(rows[-1].key)
    sys.stderr.write('chunk %d done: total=%d v2=%d v3=%d\n' % (chunk, total, v2_count, v3_count))
    sys.stderr.flush()

print('scan complete: total=%d v2=%d v3=%d' % (total, v2_count, v3_count))
