# Styling Consistency TODO

Update all solution pages and static marketing pages to match the landing page styling.

## High Priority Changes

### 1. Feature Card Icon Treatment
**Landing Page (New):** Icon only with `text-zinc-500` or `text-zinc-600`, `mb-3`
**Solution Pages (Old):** Icon wrapped in box `flex h-8 w-8 items-center justify-center rounded-lg border border-white/10 bg-white/5`

**Action:** Remove icon boxes, use plain icons with `text-zinc-500` color.

### 2. Feature Card Title Styling
**Landing Page:** `text-sm font-normal text-white` or `text-base font-normal text-white`
**Solution Pages:** `text-sm font-medium uppercase tracking-wider text-white`

**Action:** Remove `uppercase tracking-wider`, change `font-medium` to `font-normal`.

### 3. Feature Card Title Margin
**Landing Page:** `mb-1` or `mb-2`
**Solution Pages:** `mb-4`

**Action:** Standardize to `mb-1`.

### 4. Description Text Color
**Landing Page:** `text-zinc-500`
**Solution Pages:** `text-zinc-400`

**Action:** Standardize to `text-zinc-500`.

### 5. Section Header Margin
**Landing Page:** `mb-2` on h2
**Solution Pages:** `mb-3` on h2

**Action:** Standardize to `mb-2`.

---

## Medium Priority Changes

### 6. CodeBlock Standardization
- Window dots: `w-2.5 h-2.5` (not `w-3 h-3`)
- Border radius: `rounded-lg` (not `rounded-xl`)
- Header padding: `px-4 py-2` (not `px-4 py-3`)

### 7. Badge Component
**Landing Page:** `px-2 py-0.5 text-xs` with motion wrapper
**Solution Pages:** `px-3 py-1 text-xs` no animation

**Action:** Standardize to `px-3 py-1`.

### 8. Border Variations
**Action:** Use `border-white/10` for section borders and cards, `border-white/5` for subtle internal elements only.

### 9. Ecosystem/Integration Tags
**Landing Page:** `rounded-md border-white/5 px-2 py-1 text-xs`
**Solution Pages:** `rounded-full border-white/10 px-4 py-2 text-sm`

**Action:** Standardize to `rounded-md border border-white/5 px-2 py-1 text-xs`.

### 10. CTA Section Container
**Action:** Standardize to `max-w-3xl mx-auto px-6 text-center`.

---

## Low Priority Changes

### 11. Animation Stagger Delay
**Action:** Standardize to `idx * 0.1`.

### 12. Grid Gap
**Action:** Standardize feature grids to `gap-8`.

### 13. Text Selection (Global)
**Action:** Move `selection:bg-[#FF4500]/30 selection:text-orange-200` to `main.css`.

### 14. Diagram/Visualization Containers
**Action:** Standardize to `rounded-lg border border-white/10 bg-black`.

### 15. Status Badges
**Action:** Standardize to `rounded-full border border-[#FF4500]/20 bg-[#FF4500]/10 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider text-[#FF4500]`.

---

## Files to Update

- [ ] `AgentsPage.tsx`
- [ ] `WorkflowsPage.tsx`
- [ ] `GamesPage.tsx`
- [ ] `page.tsx` (solutions)
- [ ] `PricingPageClient.tsx`
- [ ] `GameServersPage.tsx`
- [ ] `GeoDistributedDBPage.tsx`
- [ ] `PerTenantDBPage.tsx`
- [ ] `CollaborativeStatePage.tsx`
- [ ] `AppGeneratorsPage.tsx`
- [ ] `UserSessionStorePage.tsx`
- [ ] `main.css` (global selection styles)
