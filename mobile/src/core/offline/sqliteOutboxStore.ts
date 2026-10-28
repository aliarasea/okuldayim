import type { SQLiteDatabase } from 'expo-sqlite';

import type { HttpMethod } from '../api/transport';
import type { OutboxCounts, OutboxItem, OutboxState, OutboxStore } from './outbox';

interface OutboxRow {
  id: string;
  method: string;
  path: string;
  body: string | null;
  created_at: number;
  attempts: number;
  next_attempt_at: number;
  last_error: string | null;
  state: string;
}

export function createSqliteOutboxStore(getDatabase: () => Promise<SQLiteDatabase>): OutboxStore {
  return {
    async insert(item) {
      const db = await getDatabase();
      await db.runAsync(
        `INSERT INTO outbox (id, method, path, body, created_at, attempts, next_attempt_at, last_error, state)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
        item.id,
        item.method,
        item.path,
        JSON.stringify(item.body ?? null),
        item.createdAt,
        item.attempts,
        item.nextAttemptAt,
        item.lastError,
        item.state,
      );
    },

    async listPending() {
      const db = await getDatabase();
      const rows = await db.getAllAsync<OutboxRow>(
        "SELECT * FROM outbox WHERE state = 'PENDING' ORDER BY created_at, rowid",
      );
      return rows.map(fromRow);
    },

    async update(item) {
      const db = await getDatabase();
      await db.runAsync(
        'UPDATE outbox SET attempts = ?, next_attempt_at = ?, last_error = ?, state = ? WHERE id = ?',
        item.attempts,
        item.nextAttemptAt,
        item.lastError,
        item.state,
        item.id,
      );
    },

    async remove(id) {
      const db = await getDatabase();
      await db.runAsync('DELETE FROM outbox WHERE id = ?', id);
    },

    async counts() {
      const db = await getDatabase();
      const rows = await db.getAllAsync<{ state: string; n: number }>(
        'SELECT state, COUNT(*) AS n FROM outbox GROUP BY state',
      );
      const counts: OutboxCounts = { pending: 0, failed: 0 };
      for (const row of rows) {
        if (row.state === 'PENDING') counts.pending = row.n;
        if (row.state === 'FAILED') counts.failed = row.n;
      }
      return counts;
    },
  };
}

function fromRow(row: OutboxRow): OutboxItem {
  return {
    id: row.id,
    method: row.method as HttpMethod,
    path: row.path,
    body: row.body === null ? null : JSON.parse(row.body),
    createdAt: row.created_at,
    attempts: row.attempts,
    nextAttemptAt: row.next_attempt_at,
    lastError: row.last_error,
    state: row.state as OutboxState,
  };
}
