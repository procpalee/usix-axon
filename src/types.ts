export interface Table {
  headers: string[];
  rows: string[][];
  keys?: string[];
}

export interface Preview {
  rows: string[][];
  total: number;
}

export interface ImportResult {
  table: Table;
  dataset_id: string;
}
