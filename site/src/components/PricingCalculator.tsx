import React, { useState } from 'react';

const PRIMITIVES = [
  { label: 'Function', value: 'function' },
  { label: 'Container', value: 'container' },
  { label: 'Actor', value: 'actor' },
];

const MEMORY_OPTIONS = [128, 256, 512, 1024, 2048]; // in MB

const MS_IN_HOUR = 3600 * 1000;
const COST_PER_MS = 0.00000000163;
const COST_PER_GB_BW = 0.05;
const COST_PER_MILLION_READS = 0.20;
const COST_PER_MILLION_WRITES = 1.00;
const COST_PER_GB_MONTH = 0.20;

function calculateCost({
  memory,
  requests,
  durationMs,
  bandwidthGB,
  reads,
  writes,
  storedGB,
}) {
  const totalMs = requests * durationMs;
  const computeCost = totalMs * COST_PER_MS * (memory / 128);
  const bandwidthCost = Math.max(0, bandwidthGB - 10) * COST_PER_GB_BW;
  const readCost = Math.max(0, reads - 1_000_000) / 1_000_000 * COST_PER_MILLION_READS;
  const writeCost = Math.max(0, writes - 1_000_000) / 1_000_000 * COST_PER_MILLION_WRITES;
  const storageCost = Math.max(0, storedGB) * COST_PER_GB_MONTH;
  return computeCost + bandwidthCost + readCost + writeCost + storageCost;
}

const inputStyle = {
  background: '#222',
  color: '#fff',
  border: '1px solid #444',
  borderRadius: 6,
  padding: '6px 10px',
  fontSize: 16,
  marginLeft: 8,
  marginTop: 2,
  marginBottom: 2,
  width: 180,
};

function PrimitiveEntry({ entry, onChange, onRemove, index }) {
  return (
    <div style={{ marginBottom: 16, border: '1px solid #333', borderRadius: 8, padding: 16, background: '#181818' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <strong>Entry {index + 1}</strong>
        <button onClick={onRemove} style={{ color: '#f87171', background: 'none', border: 'none', fontSize: 18, cursor: 'pointer' }}>Remove</button>
      </div>
      <div style={{ marginBottom: 8 }}>
        <label>Memory Allocation:&nbsp;
          <select value={entry.memory} onChange={e => onChange({ ...entry, memory: Number(e.target.value) })} style={inputStyle}>
            {MEMORY_OPTIONS.map(mb => <option key={mb} value={mb}>{mb} MB</option>)}
          </select>
        </label>
      </div>
      <div style={{ marginBottom: 8 }}>
        <label>Requests per month:&nbsp;
          <input type="number" min={0} step={1000} value={entry.requests} onChange={e => onChange({ ...entry, requests: Number(e.target.value) })} style={inputStyle} />
        </label>
      </div>
      <div style={{ marginBottom: 8 }}>
        <label>Duration per request (ms):&nbsp;
          <input type="number" min={1} step={1} value={entry.durationMs} onChange={e => onChange({ ...entry, durationMs: Number(e.target.value) })} style={inputStyle} />
        </label>
      </div>
      <div style={{ marginBottom: 8 }}>
        <label>Bandwidth usage (GB/month):&nbsp;
          <input type="number" min={0} step={1} value={entry.bandwidthGB} onChange={e => onChange({ ...entry, bandwidthGB: Number(e.target.value) })} style={inputStyle} />
        </label>
      </div>
      <div style={{ marginBottom: 8 }}>
        <label>Storage Reads per month:&nbsp;
          <input type="number" min={0} step={1000} value={entry.reads} onChange={e => onChange({ ...entry, reads: Number(e.target.value) })} style={inputStyle} />
        </label>
      </div>
      <div style={{ marginBottom: 8 }}>
        <label>Storage Writes per month:&nbsp;
          <input type="number" min={0} step={1000} value={entry.writes} onChange={e => onChange({ ...entry, writes: Number(e.target.value) })} style={inputStyle} />
        </label>
      </div>
      <div style={{ marginBottom: 8 }}>
        <label>Stored Data (GB):&nbsp;
          <input type="number" min={0} step={1} value={entry.storedGB} onChange={e => onChange({ ...entry, storedGB: Number(e.target.value) })} style={inputStyle} />
        </label>
      </div>
      <div style={{ fontWeight: 'bold', fontSize: 16, marginTop: 8 }}>
        Entry Cost: <span style={{ color: '#4ade80' }}>${calculateCost(entry).toFixed(2)}</span>
      </div>
    </div>
  );
}

function CollapsibleSection({ title, children, open, onToggle }) {
  return (
    <div style={{ marginBottom: 24 }}>
      <button onClick={onToggle} style={{
        width: '100%',
        textAlign: 'left',
        background: 'none',
        border: 'none',
        color: '#fff',
        fontSize: 20,
        fontWeight: 600,
        padding: '12px 0',
        cursor: 'pointer',
        borderBottom: '1px solid #333',
      }}>
        {open ? '▼' : '►'} {title}
      </button>
      {open && <div style={{ marginTop: 12 }}>{children}</div>}
    </div>
  );
}

const defaultEntry = {
  memory: 128,
  requests: 1000000,
  durationMs: 50,
  bandwidthGB: 0,
  reads: 1000000,
  writes: 1000000,
  storedGB: 0,
};

export default function PricingCalculator() {
  const [sections, setSections] = useState({
    function: { open: true, entries: [{ ...defaultEntry }] },
    container: { open: false, entries: [] },
    actor: { open: false, entries: [] },
  });

  const handleAdd = (type) => {
    setSections(s => ({
      ...s,
      [type]: { ...s[type], entries: [...s[type].entries, { ...defaultEntry }] },
    }));
  };

  const handleRemove = (type, idx) => {
    setSections(s => ({
      ...s,
      [type]: { ...s[type], entries: s[type].entries.filter((_, i) => i !== idx) },
    }));
  };

  const handleChange = (type, idx, entry) => {
    setSections(s => ({
      ...s,
      [type]: {
        ...s[type],
        entries: s[type].entries.map((e, i) => (i === idx ? entry : e)),
      },
    }));
  };

  const handleToggle = (type) => {
    setSections(s => ({
      ...s,
      [type]: { ...s[type], open: !s[type].open },
    }));
  };

  const totalCost = Object.values(sections).flatMap(s => s.entries).reduce((sum, entry) => sum + calculateCost(entry), 0);

  return (
    <div style={{ border: '1px solid #333', borderRadius: 12, padding: 32, maxWidth: 600, margin: '2rem auto', background: '#181818' }}>
      <h2 style={{ fontSize: 28, marginBottom: 24 }}>Estimate Your Monthly Cost</h2>
      {PRIMITIVES.map(p => (
        <CollapsibleSection
          key={p.value}
          title={p.label + 's'}
          open={sections[p.value].open}
          onToggle={() => handleToggle(p.value)}
        >
          {sections[p.value].entries.map((entry, idx) => (
            <PrimitiveEntry
              key={idx}
              entry={entry}
              index={idx}
              onChange={e => handleChange(p.value, idx, e)}
              onRemove={() => handleRemove(p.value, idx)}
            />
          ))}
          <button
            onClick={() => handleAdd(p.value)}
            style={{
              background: '#4ade80',
              color: '#181818',
              border: 'none',
              borderRadius: 6,
              padding: '8px 16px',
              fontWeight: 600,
              fontSize: 16,
              marginTop: 8,
              cursor: 'pointer',
            }}
          >
            + Add {p.label}
          </button>
        </CollapsibleSection>
      ))}
      <div style={{ fontWeight: 'bold', fontSize: 24, marginTop: 32, textAlign: 'center' }}>
        Estimated Monthly Cost: <span style={{ color: '#4ade80' }}>${totalCost.toFixed(2)}</span>
      </div>
    </div>
  );
} 