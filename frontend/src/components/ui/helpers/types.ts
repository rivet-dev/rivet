export type Breakpoint = "initial" | "sm" | "md" | "lg" | "xl" | "2xl";
export type Responsive<T> = Partial<Record<Breakpoint, T>> | T;
