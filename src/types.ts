export interface Table {
  headers: string[];
  rows: string[][];
  keys?: string[];
}

export interface Preview {
  rows: string[][];
  tail_rows: string[][];
  tail_start: number;
  total: number;
}

export interface ImportResult {
  table: Table;
  dataset_id: string;
}
